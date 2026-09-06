//! 有界整数最小费用流 oracle / Bounded integral minimum-cost-flow oracle.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::unit::UnitConversionValue;

use super::model::{CommodityId, FlowGraph};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::NetworkArcId;

/// 有界整数最小费用流 oracle 的精确枚举结果 / Exact enumeration result of the bounded integral minimum-cost-flow oracle.
#[derive(Debug, Clone, PartialEq)]
pub struct IntegralFlowOracleResult<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 最小总成本 / Minimum total cost.
    pub objective: V,
    /// 每个 `(commodity, arc)` 的整数流 / Integral flow for each `(commodity, arc)` pair.
    pub flows: BTreeMap<(CommodityId, NetworkArcId), i64>,
}

/// 求解有界整数多商品最小费用流的完整枚举 oracle / Solve bounded integral multi-commodity minimum-cost flow by complete enumeration.
///
/// 每个商品弧流被枚举为整数，弧容量约束在所有商品之间共享，节点守恒和目标比较均在
/// `V` 数值域中完成。该实现用于小规模数学交叉校验，不适合生产规模网络。
/// Each commodity-arc flow is enumerated as an integer, arc capacities are shared across
/// commodities, and conservation and objective comparisons remain in the `V` numeric domain.
/// This implementation is for small-instance mathematical cross-checks, not production-scale
/// networks.
pub fn solve_bounded_integral_min_cost_flow<V>(
    graph: &FlowGraph<V>,
) -> Result<Option<IntegralFlowOracleResult<V>>>
where
    V: SolveValue + UnitConversionValue,
{
    let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding).map_err(
        |error| NetworkSchedulingError::Conversion {
            message: error.to_string(),
        },
    )?;
    let one = V::from_f64_with_policy(1.0, SolveValueConversionPolicy::AllowRounding).map_err(
        |error| NetworkSchedulingError::Conversion {
            message: error.to_string(),
        },
    )?;
    let variables = graph
        .commodities
        .iter()
        .enumerate()
        .flat_map(|(commodity_index, commodity)| {
            graph.arcs.iter().enumerate().map(move |(arc_index, arc)| {
                (
                    commodity_index,
                    arc_index,
                    commodity.commodity.clone(),
                    arc.id.clone(),
                    arc.from.clone(),
                    arc.to.clone(),
                    arc.cost.quantity.value.clone(),
                )
            })
        })
        .map(
            |(commodity_index, arc_index, commodity, arc, from, to, cost)| {
                let from_index = graph
                    .nodes
                    .iter()
                    .position(|node| node.id == from)
                    .ok_or_else(|| {
                        NetworkSchedulingError::structure(format!(
                            "弧 {} 的起点 {} 不存在 / arc {} references missing origin {}",
                            arc, from, arc, from
                        ))
                    })?;
                let to_index = graph
                    .nodes
                    .iter()
                    .position(|node| node.id == to)
                    .ok_or_else(|| {
                        NetworkSchedulingError::structure(format!(
                            "弧 {} 的终点 {} 不存在 / arc {} references missing destination {}",
                            arc, to, arc, to
                        ))
                    })?;
                let max_flow =
                    integral_upper_bound(&graph.arcs[arc_index].capacity.upper.value, &arc)?;
                Ok(FlowOracleVariable {
                    commodity_index,
                    arc_index,
                    commodity,
                    arc,
                    from_index,
                    to_index,
                    cost,
                    max_flow,
                })
            },
        )
        .collect::<Result<Vec<_>>>()?;

    let mut search = FlowOracleSearch {
        graph,
        variables,
        assignments: vec![0; graph.commodities.len() * graph.arcs.len()],
        arc_totals: vec![zero.clone(); graph.arcs.len()],
        node_balances: vec![vec![zero.clone(); graph.nodes.len()]; graph.commodities.len()],
        zero: zero.clone(),
        one,
        objective: zero,
        best: None,
    };
    search.visit(0)?;
    Ok(search.best)
}

/// 把容量上界转换成非负整数枚举上界 / Convert a capacity upper bound into a non-negative integral enumeration bound.
fn integral_upper_bound<V>(upper: &V, arc: &NetworkArcId) -> Result<i64>
where
    V: SolveValue,
{
    upper
        .to_nonnegative_integral_upper_bound()
        .map_err(|error| NetworkSchedulingError::Conversion {
            message: format!(
                "弧 {} 的整数枚举上界无效 / arc {} integral enumeration upper bound is invalid: {}",
                arc, arc, error
            ),
        })
}

/// 一个商品弧变量的递归描述 / Recursive description of one commodity-arc variable.
struct FlowOracleVariable<V>
where
    V: SolveValue + UnitConversionValue,
{
    commodity_index: usize,
    arc_index: usize,
    commodity: CommodityId,
    arc: NetworkArcId,
    from_index: usize,
    to_index: usize,
    cost: V,
    max_flow: i64,
}

/// 完整枚举搜索状态 / Complete-enumeration search state.
struct FlowOracleSearch<'a, V>
where
    V: SolveValue + UnitConversionValue,
{
    graph: &'a FlowGraph<V>,
    variables: Vec<FlowOracleVariable<V>>,
    assignments: Vec<i64>,
    arc_totals: Vec<V>,
    node_balances: Vec<Vec<V>>,
    zero: V,
    one: V,
    objective: V,
    best: Option<IntegralFlowOracleResult<V>>,
}

impl<'a, V> FlowOracleSearch<'a, V>
where
    V: SolveValue + UnitConversionValue,
{
    fn visit(&mut self, variable_index: usize) -> Result<()> {
        if variable_index == self.variables.len() {
            return self.consider_complete_assignment();
        }

        let variable = &self.variables[variable_index];
        let commodity_index = variable.commodity_index;
        let arc_index = variable.arc_index;
        let from_index = variable.from_index;
        let to_index = variable.to_index;
        let cost = variable.cost.clone();
        let max_flow = variable.max_flow;
        let mut flow_value = self.zero.clone();
        for flow in 0..=max_flow {
            let objective_term = cost.clone() * flow_value.clone();

            self.assignments[variable_index] = flow;
            self.arc_totals[arc_index] = self.arc_totals[arc_index].clone() + flow_value.clone();
            self.node_balances[commodity_index][from_index] =
                self.node_balances[commodity_index][from_index].clone() + flow_value.clone();
            self.node_balances[commodity_index][to_index] =
                self.node_balances[commodity_index][to_index].clone() - flow_value.clone();
            self.objective = self.objective.clone() + objective_term.clone();

            self.visit(variable_index + 1)?;

            self.objective = self.objective.clone() - objective_term;
            self.node_balances[commodity_index][to_index] =
                self.node_balances[commodity_index][to_index].clone() + flow_value.clone();
            self.node_balances[commodity_index][from_index] =
                self.node_balances[commodity_index][from_index].clone() - flow_value.clone();
            self.arc_totals[arc_index] = self.arc_totals[arc_index].clone() - flow_value.clone();
            flow_value = flow_value + self.one.clone();
        }
        Ok(())
    }

    fn consider_complete_assignment(&mut self) -> Result<()> {
        if !self.capacities_are_satisfied() || !self.conservation_is_satisfied() {
            return Ok(());
        }

        let is_better = self
            .best
            .as_ref()
            .is_none_or(|best| self.objective < best.objective);
        if !is_better {
            return Ok(());
        }

        let flows = self
            .variables
            .iter()
            .zip(self.assignments.iter().copied())
            .map(|(variable, flow)| ((variable.commodity.clone(), variable.arc.clone()), flow))
            .collect();
        self.best = Some(IntegralFlowOracleResult {
            objective: self.objective.clone(),
            flows,
        });
        Ok(())
    }

    fn capacities_are_satisfied(&self) -> bool {
        self.graph.arcs.iter().enumerate().all(|(arc_index, arc)| {
            let total = &self.arc_totals[arc_index];
            at_least(total, &arc.capacity.lower.value) && at_most(total, &arc.capacity.upper.value)
        })
    }

    fn conservation_is_satisfied(&self) -> bool {
        self.graph
            .commodities
            .iter()
            .enumerate()
            .all(|(commodity_index, commodity)| {
                self.graph
                    .nodes
                    .iter()
                    .enumerate()
                    .all(|(node_index, node)| {
                        let actual = &self.node_balances[commodity_index][node_index];
                        match commodity.balances.get(&node.id) {
                            Some(expected) => equal_values(actual, &expected.value),
                            None => equal_values(actual, &self.zero),
                        }
                    })
            })
    }
}

fn equal_values<V>(left: &V, right: &V) -> bool
where
    V: PartialOrd,
{
    left.partial_cmp(right) == Some(Ordering::Equal)
}

fn at_least<V>(value: &V, lower: &V) -> bool
where
    V: PartialOrd,
{
    matches!(
        value.partial_cmp(lower),
        Some(Ordering::Greater | Ordering::Equal)
    )
}

fn at_most<V>(value: &V, upper: &V) -> bool
where
    V: PartialOrd,
{
    matches!(
        value.partial_cmp(upper),
        Some(Ordering::Less | Ordering::Equal)
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::borrow_interior_mutable_const)]

    use super::solve_bounded_integral_min_cost_flow;
    use crate::domain::flow::{FlowArc, FlowGraph, FlowNode, FlowUnits, SupplyDemand};
    use crate::domain::vrp::VrptwUnits;
    use crate::infrastructure::{CapacityBounds, NetworkCost};
    use ospf_rust_quantities::Quantity;
    use ospf_rust_quantities::unit::{CTUnit, Kilogram};

    #[test]
    fn oracle_handles_shared_capacity_negative_cost_and_self_loops() {
        let flow_unit = Kilogram::INSTANT.clone();
        let cost_unit = VrptwUnits::default().cost_unit;
        let capacity = |upper| {
            CapacityBounds::try_new(
                Quantity::new(0.0, flow_unit.clone()),
                Quantity::new(upper, flow_unit.clone()),
            )
            .expect("valid test capacity")
        };
        let graph = FlowGraph::new_with_units(
            vec![FlowNode::new("source"), FlowNode::new("sink")],
            vec![
                FlowArc::new(
                    "loop",
                    "source",
                    "source",
                    capacity(2.0),
                    NetworkCost::new(Quantity::new(-1.0, cost_unit.clone())),
                ),
                FlowArc::new(
                    "route-a",
                    "source",
                    "sink",
                    capacity(1.0),
                    NetworkCost::new(Quantity::new(2.0, cost_unit.clone())),
                ),
                FlowArc::new(
                    "route-b",
                    "source",
                    "sink",
                    capacity(1.0),
                    NetworkCost::new(Quantity::new(2.0, cost_unit.clone())),
                ),
            ],
            vec![
                SupplyDemand::new("first")
                    .with_balance("source", Quantity::new(1.0, flow_unit.clone()))
                    .with_balance("sink", Quantity::new(-1.0, flow_unit.clone())),
                SupplyDemand::new("second")
                    .with_balance("source", Quantity::new(1.0, flow_unit.clone()))
                    .with_balance("sink", Quantity::new(-1.0, flow_unit.clone())),
            ],
            FlowUnits::new(flow_unit, cost_unit).expect("valid test flow units"),
        )
        .expect("valid test graph");

        let result = solve_bounded_integral_min_cost_flow(&graph)
            .expect("oracle succeeds")
            .expect("test graph is feasible");

        assert_eq!(result.objective, 2.0);
        let loop_total = result.flows[&("first".into(), "loop".into())]
            + result.flows[&("second".into(), "loop".into())];
        assert_eq!(loop_total, 2);
        let route_a_total = result.flows[&("first".into(), "route-a".into())]
            + result.flows[&("second".into(), "route-a".into())];
        let route_b_total = result.flows[&("first".into(), "route-b".into())]
            + result.flows[&("second".into(), "route-b".into())];
        assert_eq!(route_a_total, 1);
        assert_eq!(route_b_total, 1);
    }

    #[cfg(feature = "big-decimal")]
    #[test]
    fn big_decimal_oracle_bound_preserves_integer_above_f64_exact_range() {
        use bigdecimal::BigDecimal;
        use std::str::FromStr;

        let upper = BigDecimal::from_str("9007199254740993").expect("valid decimal bound");
        let bound = super::integral_upper_bound(&upper, &"wide-arc".into())
            .expect("BigDecimal integral bound should stay exact");
        assert_eq!(bound, 9_007_199_254_740_993_i64);
    }
}
