//! 仅负责建设的可复用组件，不隐式创建流。 / Reusable construction-only component that never implicitly creates flows.

use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::variable::{Binary, VariableRange};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::UnitConversionValue;
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkNodeId, NetworkSchedulingSolverValueAdapter};
use super::model::{NetworkConstructionSolution, NetworkDesignProblem};
use super::pipeline::{LinearRow, register_row, validate_rows};

/// 模型绑定的建设变量组件，可由结构和韧性领域复用。 / Model-bound construction-variable component reusable by structure and resilience.
#[derive(Debug, Clone)]
pub struct NetworkConstruction<V: SolveValue + UnitConversionValue> {
    problem: NetworkDesignProblem<V>,
    name: String,
    binding: Option<u64>,
    facilities: BTreeMap<NetworkNodeId, usize>,
    services: BTreeMap<NetworkArcId, Vec<usize>>,
    cost: Vec<(usize, f64)>,
    rows: Vec<LinearRow>,
}

impl<V: SolveValue + UnitConversionValue> NetworkConstruction<V> {
    /// 创建无注册态的建设组件。 / Create an unregistered construction component.
    pub fn new(problem: NetworkDesignProblem<V>, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.is_empty() { return Err(NetworkSchedulingError::validation("建设名称不能为空 / construction name cannot be empty")); }
        Ok(Self { problem, name, binding: None, facilities: BTreeMap::new(), services: BTreeMap::new(), cost: Vec::new(), rows: Vec::new() })
    }

    /// 不可变问题 / Immutable problem.
    pub fn problem(&self) -> &NetworkDesignProblem<V> { &self.problem }

    /// 建设费用表达式 / Construction-cost expression.
    pub fn cost_expression(&self) -> &[(usize, f64)] { &self.cost }

    /// 设施变量；没有声明的设施视为固定启用。 / Facility variable; an undeclared facility is fixed open.
    pub fn facility_variable(&self, node: &NetworkNodeId) -> Option<usize> { self.facilities.get(node).copied() }

    /// 等级顺序与问题声明一致。 / Level order matches the problem declaration.
    pub fn service_variables(&self, arc: &NetworkArcId) -> Option<&[usize]> { self.services.get(arc).map(Vec::as_slice) }

    /// 原子注册互斥等级、必选设施和建设预算。 / Atomically register exclusive levels, required facilities and construction budget.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        if let Some(binding) = self.binding {
            return if binding == model.model_identity() { Ok(()) } else {
                Err(NetworkSchedulingError::contract("建设组件不能跨模型使用 / construction component cannot cross models"))
            };
        }
        let mut next = self.clone();
        model.transaction(|model| {
            let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
            for facility in next.problem.facilities() {
                let index = model.register_auto_variable_with_range::<Binary>(
                    &format!("{}_facility_{}", next.name, facility.node), VariableRange::bounded(0.0, 1.0),
                ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                next.facilities.insert(facility.node.clone(), index);
                next.cost.push((index, adapter.normalize(&facility.cost, next.problem.cost_unit())?));
                if facility.required {
                    next.rows.push(register_row(model, vec![(index, 1.0)], ConstraintRelation::Equal, 1.0,
                        &format!("{}_required_{}", next.name, facility.node))?);
                }
            }
            for service in next.problem.services() {
                let mut variables = Vec::new();
                for level in &service.levels {
                    let index = model.register_auto_variable_with_range::<Binary>(
                        &format!("{}_level_{}_{}", next.name, service.arc, level.id), VariableRange::bounded(0.0, 1.0),
                    ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                    next.cost.push((index, adapter.normalize(&level.cost, next.problem.cost_unit())?));
                    variables.push(index);
                }
                next.rows.push(register_row(model, variables.iter().map(|index| (*index, 1.0)).collect(),
                    if service.required { ConstraintRelation::Equal } else { ConstraintRelation::LessEqual }, 1.0,
                    &format!("{}_exclusive_{}", next.name, service.arc))?);
                next.services.insert(service.arc.clone(), variables);
            }
            if let Some(budget) = next.problem.budget() {
                next.rows.push(register_row(model, next.cost.clone(), ConstraintRelation::LessEqual,
                    adapter.normalize(budget, next.problem.cost_unit())?, &format!("{}_budget", next.name))?);
            }
            Ok(())
        })?;
        next.binding = Some(model.model_identity());
        *self = next;
        Ok(())
    }

    /// 独立校验二元性、必选、互斥、预算与费用后提取。 / Extract after independently validating binary values, requirements, exclusivity, budget and cost.
    pub fn extract_solution(&self, model: &MetaModel<f64>) -> Result<NetworkConstructionSolution<V>> {
        self.validate_binding(model)?;
        validate_rows(model, &self.rows)?;
        let mut open_facilities = BTreeSet::new();
        let mut service_levels = BTreeMap::new();
        for (node, index) in &self.facilities {
            if binary(model, *index)? == 1.0 { open_facilities.insert(node.clone()); }
        }
        for service in self.problem.services() {
            for (level, index) in service.levels.iter().zip(&self.services[&service.arc]) {
                if binary(model, *index)? == 1.0 { service_levels.insert(service.arc.clone(), level.id.clone()); }
            }
        }
        let mut cost = 0.0;
        for (index, coefficient) in &self.cost { cost += binary(model, *index)? * coefficient; }
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        Ok(NetworkConstructionSolution { open_facilities, service_levels,
            cost: Quantity::new(adapter.from_solver(cost)?, self.problem.cost_unit().clone()) })
    }

    pub(super) fn validate_binding(&self, model: &MetaModel<f64>) -> Result<()> {
        if self.binding != Some(model.model_identity()) {
            return Err(NetworkSchedulingError::contract("建设组件未注册或模型身份不匹配 / unregistered construction or model identity mismatch"));
        }
        Ok(())
    }
}

fn binary(model: &MetaModel<f64>, index: usize) -> Result<f64> {
    let value = model.tokens().get(index).and_then(|token| token.get_result())
        .ok_or_else(|| NetworkSchedulingError::contract("缺少建设选择结果 / missing construction choice result"))?;
    if !value.is_finite() || !(-1e-7..=1.0 + 1e-7).contains(&value) || (value - value.round()).abs() > 1e-7 {
        return Err(NetworkSchedulingError::contract("建设选择不是合法二元值 / invalid binary construction choice"));
    }
    Ok(value.round())
}
