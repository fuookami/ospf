//! 单组建设变量与显式场景 recourse 聚合。 / A single construction-variable set with explicit scenario recourse.

use std::collections::BTreeMap;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::variable::{Continuous, VariableRange};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};
use crate::domain::design::{NetworkConstruction, NetworkDesignProblem};
use crate::domain::design::pipeline::{LinearRow, embedded_flow, link_flow, register_row, validate_rows};
use crate::domain::flow::{FlowContext, FlowProblem, FlowRegistration};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkSchedulingSolverValueAdapter};
use super::connectivity::ConnectivityFlow;
use super::model::{FailureScenario, NetworkResilienceProblem, NetworkResilienceSolution};

#[derive(Debug, Clone)]
pub(super) struct ScenarioFlow<V: SolveValue + UnitConversionValue> {
    declared: FlowProblem<V>,
    hard_lowers: BTreeMap<NetworkArcId, Quantity<V, Unit>>,
    context: FlowContext<V>,
    rows: Vec<LinearRow>,
}

pub(super) fn prepare_scenario<V>(
    design: &NetworkDesignProblem<V>,
    scenario: &FailureScenario<V>,
) -> Result<ScenarioFlow<V>>
where
    V: SolveValue + UnitConversionValue,
{
    let nominal = design.flow();
    let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
    if scenario.closed_arcs.iter().any(|id| nominal.arc(id).is_none())
        || scenario.failed_nodes.iter().any(|id| nominal.node(id).is_none())
        || scenario.balances.keys().chain(scenario.slacks.keys()).any(|id| !nominal.commodities().iter().any(|item| &item.commodity == id))
    {
        return Err(NetworkSchedulingError::structure("故障场景引用未知资源或商品 / failure scenario references unknown assets or commodities"));
    }
    for (id, upper) in &scenario.upper_capacities {
        let arc = nominal.arc(id).ok_or_else(|| NetworkSchedulingError::structure("降级引用未知弧 / degradation references an unknown arc"))?;
        let value = adapter.normalize(upper, &nominal.units().flow_unit)?;
        if value < 0.0 || value > adapter.to_solver(&arc.capacity.upper.value)? {
            return Err(NetworkSchedulingError::validation("故障容量必须非负且不超过名义容量 / failure capacity must be nonnegative and no greater than nominal capacity"));
        }
    }
    let mut hard_lowers = BTreeMap::new();
    let arcs = nominal.arcs().iter().filter(|arc| !scenario.closed_arcs.contains(&arc.id)
        && !scenario.failed_nodes.contains(&arc.from) && !scenario.failed_nodes.contains(&arc.to))
        .map(|arc| {
            let mut arc = arc.clone();
            hard_lowers.insert(arc.id.clone(), arc.capacity.lower.clone());
            arc.capacity.lower.value = adapter.from_solver(0.0)?;
            if let Some(upper) = scenario.upper_capacities.get(&arc.id) { arc.capacity.upper = upper.clone(); }
            Ok(arc)
        }).collect::<Result<Vec<_>>>()?;
    let active = arcs.iter().map(|arc| arc.id.clone()).collect::<std::collections::HashSet<_>>();
    let mut commodities = nominal.commodities().to_vec();
    for item in &mut commodities {
        if let Some(balances) = scenario.balances.get(&item.commodity) { item.balances = balances.clone(); }
        if let Some(slacks) = scenario.slacks.get(&item.commodity) { item.slacks = slacks.clone(); }
        item.costs.retain(|id, _| active.contains(id));
        item.capacity_consumption.retain(|id, _| active.contains(id));
        if let Some(allowed) = &mut item.allowed_arcs { allowed.retain(|id| active.contains(id)); }
    }
    let mut resources = nominal.resources().to_vec();
    for resource in &mut resources { resource.consumptions.retain(|(_, id), _| active.contains(id)); }
    let declared = FlowProblem::new_with_units(nominal.nodes().to_vec(), arcs, commodities, nominal.units().clone())?
        .with_resources(resources)?;
    let context = FlowContext::new(embedded_flow(&declared)?);
    Ok(ScenarioFlow { declared, hard_lowers, context, rows: Vec::new() })
}

/// 公开建设、场景流与费用表达式供下游扩展。 / Expose construction, scenario flows and cost expressions for downstream extension.
#[derive(Debug, Clone)]
pub struct NetworkResilienceAggregation<V: SolveValue + UnitConversionValue> {
    problem: NetworkResilienceProblem<V>,
    construction: NetworkConstruction<V>,
    flows: BTreeMap<String, ScenarioFlow<V>>,
    connectivity: Vec<ConnectivityFlow>,
    epigraph: Option<usize>,
    rows: Vec<LinearRow>,
}

impl<V: SolveValue + UnitConversionValue> NetworkResilienceAggregation<V> {
    pub(super) fn new(problem: NetworkResilienceProblem<V>, name: &str) -> Result<Self> {
        let construction = NetworkConstruction::new(problem.design().clone(), name)?;
        let nominal = problem.design().flow();
        let mut flows = BTreeMap::new();
        flows.insert("nominal".into(), ScenarioFlow {
            declared: nominal.clone(), hard_lowers: BTreeMap::new(),
            context: FlowContext::new(embedded_flow(nominal)?).with_name(format!("{name}_nominal"))?, rows: Vec::new(),
        });
        for scenario in problem.scenarios() {
            let mut flow = prepare_scenario(problem.design(), scenario)?;
            flow.context = flow.context.with_name(format!("{name}_scenario_{}", scenario.id))?;
            flows.insert(scenario.id.clone(), flow);
        }
        Ok(Self { problem, construction, flows, connectivity: Vec::new(), epigraph: None, rows: Vec::new() })
    }

    /// 不可变业务问题 / Immutable business problem.
    pub fn problem(&self) -> &NetworkResilienceProblem<V> { &self.problem }

    /// 唯一共享建设组件 / The single shared construction component.
    pub fn construction(&self) -> &NetworkConstruction<V> { &self.construction }

    /// 按显式场景 ID 获取流，名义场景为 nominal。 / Access flow by explicit scenario ID; nominal denotes the base case.
    pub fn flow(&self, scenario: &str) -> Result<&FlowContext<V>> {
        self.flows.get(scenario).map(|flow| &flow.context)
            .ok_or_else(|| NetworkSchedulingError::structure("未知场景 / unknown scenario"))
    }

    /// 建设加最坏费用的线性目标表达式。 / Linear construction-plus-worst-cost objective expression.
    pub fn cost_expression(&self) -> Result<Vec<(usize, f64)>> {
        let index = self.epigraph.ok_or_else(|| NetworkSchedulingError::contract("最坏费用尚未注册 / worst cost is not registered"))?;
        let mut terms = self.construction.cost_expression().to_vec();
        terms.push((index, 1.0));
        Ok(terms)
    }

    pub(super) fn register(&mut self, model: &mut MetaModel<f64>, name: &str) -> Result<()> {
        self.construction.register(model)?;
        let epigraph = model.register_auto_variable_with_range::<Continuous>(
            &format!("{name}_worst_cost"), VariableRange::unbounded(),
        ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        self.epigraph = Some(epigraph);
        for (id, flow) in &mut self.flows {
            flow.context.register_with_options(model, FlowRegistration { minimize_cost: false })?;
            flow.rows = link_flow(model, &self.construction, &flow.context, &flow.declared, &flow.hard_lowers, &format!("{name}_{id}"))?;
            let mut terms = flow.context.aggregation.cost_expression()?;
            terms.push((epigraph, -1.0));
            self.rows.push(register_row(model, terms, ConstraintRelation::LessEqual, 0.0, &format!("{name}_{id}_worst_cost"))?);
        }
        for (index, requirement) in self.problem.connectivity().iter().enumerate() {
            self.connectivity.push(ConnectivityFlow::register(model, &self.construction, requirement, &format!("{name}_connectivity_{index}"))?);
        }
        Ok(())
    }

    pub(super) fn extract(&mut self, model: &MetaModel<f64>) -> Result<NetworkResilienceSolution<V>> {
        let construction = self.construction.extract_solution(model)?;
        validate_rows(model, &self.rows)?;
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let cost_unit = self.problem.design().cost_unit();
        let mut scenarios = BTreeMap::new();
        let mut effective_capacities = BTreeMap::new();
        let mut worst = f64::NEG_INFINITY;
        let mut worst_scenario = String::new();
        for (id, flow) in &mut self.flows {
            validate_rows(model, &flow.rows)?;
            let solution = flow.context.extract_solution(model)?;
            let cost = adapter.to_solver(&solution.total_cost.value)?;
            if cost > worst { worst = cost; worst_scenario = id.clone(); }
            scenarios.insert(id.clone(), solution);
            let mut capacities = BTreeMap::new();
            for arc in self.problem.design().flow().arcs() {
                let mut upper = flow.declared.arc(&arc.id).map(|item| adapter.to_solver(&item.capacity.upper.value)).transpose()?.unwrap_or(0.0);
                if let Some(service) = self.problem.design().services().iter().find(|service| service.arc == arc.id) {
                    let selected = construction.service_levels.get(&arc.id);
                    upper = upper.min(service.levels.iter().find(|level| Some(&level.id) == selected)
                        .map(|level| adapter.normalize(&level.capacity, &self.problem.design().flow().units().flow_unit)).transpose()?.unwrap_or(0.0));
                }
                if self.problem.design().facilities().iter().any(|facility| (facility.node == arc.from || facility.node == arc.to)
                    && !construction.open_facilities.contains(&facility.node)) { upper = 0.0; }
                capacities.insert(arc.id.clone(), Quantity::new(adapter.from_solver(upper)?, self.problem.design().flow().units().flow_unit.clone()));
            }
            effective_capacities.insert(id.clone(), capacities);
        }
        let epigraph = self.epigraph.and_then(|index| model.tokens().get(index)).and_then(|token| token.get_result())
            .ok_or_else(|| NetworkSchedulingError::contract("缺少最坏费用结果 / missing worst-cost result"))?;
        let total = adapter.to_solver(&construction.cost.value)? + worst;
        Ok(NetworkResilienceSolution {
            connectivity_flows: self.connectivity.iter().map(|flow| flow.extract(model)).collect::<Result<_>>()?,
            construction, scenarios, worst_scenario, effective_capacities,
            worst_operating_cost: Quantity::new(adapter.from_solver(worst)?, cost_unit.clone()),
            operating_cost_epigraph: Quantity::new(adapter.from_solver(epigraph)?, cost_unit.clone()),
            total_cost: Quantity::new(adapter.from_solver(total)?, cost_unit.clone()),
        })
    }
}
