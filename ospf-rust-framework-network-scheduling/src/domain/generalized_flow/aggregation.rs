//! 计量归一化、弧变量与生产结果校验。 / Measurement normalization, arc variables and production-result validation.

use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::variable::{Continuous, VariableRange};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, NoneUnit, Unit, UnitConversionValue};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{
    NetworkArcId, NetworkNodeId, NetworkSchedulingSolverValueAdapter, validate_linear_units,
};
use super::model::{
    GeneralizedFlowArcResult, GeneralizedFlowNodeResult, GeneralizedFlowProblem,
    GeneralizedFlowSolution, GeneralizedFlowSolverTolerance,
};

#[derive(Debug, Clone)]
pub(super) struct PreparedArc {
    pub id: NetworkArcId,
    pub from: NetworkNodeId,
    pub to: NetworkNodeId,
    pub lower: f64,
    pub upper: f64,
    pub conversion: f64,
    pub output: f64,
    pub cost: f64,
}

#[derive(Debug, Clone)]
pub(super) struct PreparedProblem {
    pub arcs: Vec<PreparedArc>,
    pub supplies: BTreeMap<NetworkNodeId, f64>,
}

pub(super) fn prepare<V>(problem: &GeneralizedFlowProblem<V>) -> Result<PreparedProblem>
where
    V: SolveValue + UnitConversionValue,
{
    let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
    let dimensionless = NoneUnit::INSTANT;
    validate_linear_units([problem.cost_unit()])?;
    let mut nodes = BTreeMap::new();
    let mut supplies = BTreeMap::new();
    for node in problem.nodes() {
        if node.id.as_str().is_empty() || nodes.insert(node.id.clone(), node).is_some() {
            return Err(NetworkSchedulingError::validation(
                "广义流节点 ID 为空或重复 / empty or duplicate generalized-flow node ID",
            ));
        }
        supplies.insert(node.id.clone(), adapter.normalize(&node.net_supply, &node.net_supply.unit)?);
    }
    let mut ids = BTreeSet::new();
    let mut arcs = Vec::with_capacity(problem.arcs().len());
    for arc in problem.arcs() {
        if arc.id.as_str().is_empty() || !ids.insert(arc.id.clone()) {
            return Err(NetworkSchedulingError::validation(
                "广义流弧 ID 为空或重复 / empty or duplicate generalized-flow arc ID",
            ));
        }
        let from = nodes.get(&arc.from).ok_or_else(|| NetworkSchedulingError::structure(
            "广义流起点不存在 / missing generalized-flow origin",
        ))?;
        let to = nodes.get(&arc.to).ok_or_else(|| NetworkSchedulingError::structure(
            "广义流终点不存在 / missing generalized-flow destination",
        ))?;
        let source = &from.net_supply.unit;
        let destination = &to.net_supply.unit;
        // 必须在除法构造复合单位之前验证原始单位。 / Validate raw units before constructing quotient units.
        validate_linear_units([
            source, destination, &arc.cost.quantity.unit, &arc.input_capacity.lower.unit,
            &arc.input_capacity.upper.unit, &arc.production_yield.unit,
        ])?;
        if let Some(factor) = &arc.input_to_output_factor {
            validate_linear_units([&factor.unit])?;
        }
        let lower = adapter.normalize(&arc.input_capacity.lower, source)?;
        let upper = adapter.normalize(&arc.input_capacity.upper, source)?;
        let yield_value = adapter.normalize(&arc.production_yield, &dimensionless)?;
        if lower < 0.0 || upper < lower || yield_value < 0.0 {
            return Err(NetworkSchedulingError::validation(
                "投入容量或产率非法 / invalid input capacity or production yield",
            ));
        }
        let conversion = if let Some(factor) = &arc.input_to_output_factor {
            adapter.normalize(factor, &(destination / source).build())?
        } else {
            let one = Quantity::new(adapter.from_solver(1.0)?, source.clone());
            adapter.normalize(&one, destination)?
        };
        if conversion <= 0.0 {
            return Err(NetworkSchedulingError::validation(
                "投入到产出转换必须为正 / input-to-output conversion must be positive",
            ));
        }
        let cost = adapter.normalize(&arc.cost.quantity, &(problem.cost_unit() / source).build())?;
        let output = finite(conversion * yield_value)?;
        // 有限投入边界亦不能在派生费用或产出时溢出。 / Finite input bounds must not overflow derived costs or outputs.
        finite(upper * output)?;
        finite(upper * conversion)?;
        finite(upper * cost)?;
        arcs.push(PreparedArc {
            id: arc.id.clone(), from: arc.from.clone(), to: arc.to.clone(),
            lower, upper, conversion, output, cost,
        });
    }
    Ok(PreparedProblem { arcs, supplies })
}

fn finite(value: f64) -> Result<f64> {
    if value.is_finite() { Ok(value) } else {
        Err(NetworkSchedulingError::contract(
            "广义流中间值必须有限 / generalized-flow intermediate value must be finite",
        ))
    }
}

/// 归一化表达式边界，可供下游约束与目标扩展。 / Normalized expression boundary for downstream constraints and objectives.
#[derive(Debug, Clone, Default)]
pub struct GeneralizedFlowExpressions {
    /// 每条弧的投入表达式 / Input expression per arc.
    pub inputs: BTreeMap<NetworkArcId, Vec<(usize, f64)>>,
    /// 每条弧的到达产出表达式 / Arrival-output expression per arc.
    pub outputs: BTreeMap<NetworkArcId, Vec<(usize, f64)>>,
    /// 各节点出投入减入产出 / Outgoing input minus incoming output per node.
    pub net_supplies: BTreeMap<NetworkNodeId, Vec<(usize, f64)>>,
    /// 总费用表达式 / Total-cost expression.
    pub cost: Vec<(usize, f64)>,
}

/// 每模型独立的弧变量组件与领域聚合。 / Per-model arc-variable component and domain aggregation.
#[derive(Debug, Clone)]
pub struct GeneralizedFlowAggregation<V> {
    problem: GeneralizedFlowProblem<V>,
    pub(super) prepared: PreparedProblem,
    pub(super) variables: BTreeMap<NetworkArcId, usize>,
    expressions: GeneralizedFlowExpressions,
}

impl<V> GeneralizedFlowAggregation<V>
where
    V: SolveValue + UnitConversionValue,
{
    pub(super) fn new(problem: GeneralizedFlowProblem<V>) -> Result<Self> {
        let prepared = prepare(&problem)?;
        Ok(Self { problem, prepared, variables: BTreeMap::new(), expressions: Default::default() })
    }

    /// 不可变问题 / Immutable problem.
    pub fn problem(&self) -> &GeneralizedFlowProblem<V> { &self.problem }

    /// 已注册表达式；注册前为空 / Registered expressions, empty before registration.
    pub fn expressions(&self) -> &GeneralizedFlowExpressions { &self.expressions }

    /// 按稳定弧身份查询投入变量 / Look up an input variable by stable arc identity.
    pub fn input_variable(&self, id: &NetworkArcId) -> Option<usize> {
        self.variables.get(id).copied()
    }

    pub(super) fn register_variables(&mut self, model: &mut MetaModel<f64>, name: &str) -> Result<()> {
        self.expressions.net_supplies = self.prepared.supplies.keys()
            .map(|id| (id.clone(), Vec::new())).collect();
        for arc in &self.prepared.arcs {
            let index = model.register_auto_variable_with_range::<Continuous>(
                &format!("{name}_input_{}", arc.id),
                VariableRange::bounded(arc.lower, arc.upper),
            ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
            self.variables.insert(arc.id.clone(), index);
            self.expressions.inputs.insert(arc.id.clone(), vec![(index, 1.0)]);
            self.expressions.outputs.insert(arc.id.clone(), vec![(index, arc.output)]);
            self.expressions.cost.push((index, arc.cost));
            self.expressions.net_supplies.get_mut(&arc.from).ok_or_else(||
                NetworkSchedulingError::contract("缺少起点行 / missing origin row"))?.push((index, 1.0));
            self.expressions.net_supplies.get_mut(&arc.to).ok_or_else(||
                NetworkSchedulingError::contract("缺少终点行 / missing destination row"))?.push((index, -arc.output));
        }
        Ok(())
    }

    pub(super) fn extract(
        &self,
        model: &MetaModel<f64>,
        tolerance: GeneralizedFlowSolverTolerance,
    ) -> Result<GeneralizedFlowSolution<V>> {
        tolerance.for_values(&[])?;
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let units = self.problem.nodes().iter().map(|node| (node.id.clone(), &node.net_supply.unit))
            .collect::<BTreeMap<_, _>>();
        let quantity = |value: f64, unit: &Unit| -> Result<Quantity<V, Unit>> {
            Ok(Quantity::new(adapter.from_solver(value)?, unit.clone()))
        };
        let mut totals = self.prepared.supplies.keys().map(|id| (id.clone(), (0.0, 0.0)))
            .collect::<BTreeMap<_, _>>();
        let mut arcs = BTreeMap::new();
        let mut total_cost = 0.0;
        for arc in &self.prepared.arcs {
            let index = self.variables.get(&arc.id).ok_or_else(|| NetworkSchedulingError::contract(
                "缺少投入变量 / missing input variable",
            ))?;
            let input = model.tokens().get(*index).and_then(|token| token.get_result())
                .ok_or_else(|| NetworkSchedulingError::contract("缺少广义流投入结果 / missing generalized-flow input result"))?;
            if !input.is_finite() || input < 0.0 {
                return Err(NetworkSchedulingError::contract(
                    "广义流投入结果必须有限非负 / generalized-flow input result must be finite and nonnegative",
                ));
            }
            let delta = tolerance.for_values(&[input, arc.lower, arc.upper])?;
            if finite(arc.lower - input)? > delta || finite(input - arc.upper)? > delta {
                return Err(NetworkSchedulingError::contract("广义流投入违反容量 / generalized-flow input violates capacity"));
            }
            let converted = finite(input * arc.conversion)?;
            let output = finite(input * arc.output)?;
            let loss = finite(converted - output)?;
            let cost = finite(input * arc.cost)?;
            total_cost = finite(total_cost + cost)?;
            let from = totals.get_mut(&arc.from).ok_or_else(|| NetworkSchedulingError::contract("缺少起点统计 / missing origin totals"))?;
            from.0 = finite(from.0 + input)?;
            let to = totals.get_mut(&arc.to).ok_or_else(|| NetworkSchedulingError::contract("缺少终点统计 / missing destination totals"))?;
            to.1 = finite(to.1 + output)?;
            arcs.insert(arc.id.clone(), GeneralizedFlowArcResult {
                input: quantity(input, units[&arc.from])?,
                output: quantity(output, units[&arc.to])?,
                loss_or_gain: quantity(loss, units[&arc.to])?,
                cost: quantity(cost, self.problem.cost_unit())?,
            });
        }
        let mut nodes = BTreeMap::new();
        for (id, (outgoing, incoming)) in totals {
            let actual = finite(outgoing - incoming)?;
            let declared = self.prepared.supplies[&id];
            let residual = finite(actual - declared)?;
            let delta = tolerance.for_values(&[actual, declared])?;
            if residual.abs() > delta {
                return Err(NetworkSchedulingError::contract(format!(
                    "广义流节点守恒残差超限：{id} / generalized-flow node balance residual exceeds tolerance: {id}",
                )));
            }
            nodes.insert(id.clone(), GeneralizedFlowNodeResult {
                actual_net_supply: quantity(actual, units[&id])?,
                residual: quantity(residual, units[&id])?,
            });
        }
        Ok(GeneralizedFlowSolution { arcs, nodes, total_cost: quantity(total_cost, self.problem.cost_unit())? })
    }
}
