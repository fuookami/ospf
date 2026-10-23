//! 流解的独立生产校验及费用分解。 / Independent production validation and cost decomposition of flow solutions.

use std::collections::BTreeMap;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::NetworkSchedulingSolverValueAdapter;
use super::{FlowAggregation, FlowSolution};

fn finite(value: f64) -> Result<f64> {
    if value.is_finite() { Ok(value) } else {
        Err(NetworkSchedulingError::contract("流解中间值必须有限 / flow solution intermediate must be finite"))
    }
}

fn check_bounds(value: f64, lower: f64, upper: f64) -> Result<()> {
    let tolerance = finite(1e-7 + 1e-7 * value.abs().max(lower.abs()).max(upper.abs()))?;
    if finite(lower - value)? > tolerance || finite(value - upper)? > tolerance {
        return Err(NetworkSchedulingError::contract("流解违反容量或守恒 / flow solution violates capacity or conservation"));
    }
    Ok(())
}

pub(super) fn extract<V>(aggregation: &FlowAggregation<V>, model: &MetaModel<f64>) -> Result<FlowSolution<V>>
where
    V: SolveValue + UnitConversionValue,
{
    let problem = aggregation.problem();
    let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
    let quantity = |value, unit: &Unit| -> Result<Quantity<V, Unit>> {
        Ok(Quantity::new(adapter.from_solver(value)?, unit.clone()))
    };
    let value = |index: usize| -> Result<f64> {
        let result = model.tokens().get(index).and_then(|token| token.get_result())
            .ok_or_else(|| NetworkSchedulingError::contract("缺少流变量结果 / missing flow variable result"))?;
        if !result.is_finite() || result < 0.0 {
            return Err(NetworkSchedulingError::contract("流量结果必须有限非负 / flow result must be finite and nonnegative"));
        }
        Ok(result)
    };
    let mut flows = BTreeMap::new();
    let mut amounts = BTreeMap::new();
    let mut arc_totals = problem.arcs().iter().map(|arc| (arc.id.clone(), 0.0)).collect::<BTreeMap<_, _>>();
    let mut spills = BTreeMap::new();
    let mut shortages = BTreeMap::new();
    let mut operating_cost = 0.0;
    let mut spill_cost = 0.0;
    let mut shortage_cost = 0.0;
    for commodity in problem.commodities() {
        let unit = problem.commodity_unit(commodity);
        let step = adapter.to_solver(&problem.step(commodity)?)?;
        let mut balances = problem.nodes().iter().map(|node| (node.id.clone(), 0.0)).collect::<BTreeMap<_, _>>();
        for arc in problem.arcs().iter().filter(|arc| problem.is_allowed(commodity, &arc.id)) {
            let index = aggregation.variable_index(commodity.commodity.as_str(), &arc.id)
                .ok_or_else(|| NetworkSchedulingError::contract("缺少商品弧变量 / missing commodity-arc variable"))?;
            let count = value(index)?;
            if commodity.integer_step.is_some() && (count - count.round()).abs() > 1e-7 {
                return Err(NetworkSchedulingError::contract("整数流计数不是整数 / integral flow count is not an integer"));
            }
            let amount = finite(count * step)?;
            let usage = finite(amount * adapter.to_solver(&problem.capacity_coefficient(commodity, &arc.id)?)?)?;
            let total = arc_totals.get_mut(&arc.id).ok_or_else(|| NetworkSchedulingError::contract("缺少弧统计 / missing arc totals"))?;
            *total = finite(*total + usage)?;
            let from = balances.get_mut(&arc.from).ok_or_else(|| NetworkSchedulingError::contract("缺少起点统计 / missing origin totals"))?;
            *from = finite(*from + amount)?;
            let to = balances.get_mut(&arc.to).ok_or_else(|| NetworkSchedulingError::contract("缺少终点统计 / missing destination totals"))?;
            *to = finite(*to - amount)?;
            operating_cost = finite(operating_cost + finite(amount * adapter.to_solver(&problem.cost_coefficient(commodity, arc)?)?)?)?;
            let key = (commodity.commodity.clone(), arc.id.clone());
            flows.insert(key.clone(), quantity(amount, unit)?);
            amounts.insert(key, amount);
        }
        for (node, slack) in &commodity.slacks {
            let (spill_index, shortage_index) = aggregation.slack_indices().get(&(commodity.commodity.as_str().to_owned(), node.clone()))
                .ok_or_else(|| NetworkSchedulingError::contract("缺少松弛变量 / missing slack variables"))?;
            let spill = value(*spill_index)?;
            let shortage = value(*shortage_index)?;
            check_bounds(spill, 0.0, adapter.to_solver(&slack.spill_limit.value)?)?;
            check_bounds(shortage, 0.0, adapter.to_solver(&slack.shortage_limit.value)?)?;
            let balance = balances.get_mut(node).ok_or_else(|| NetworkSchedulingError::contract("缺少节点统计 / missing node totals"))?;
            *balance = finite(*balance + finite(spill - shortage)?)?;
            spill_cost = finite(spill_cost + finite(spill * adapter.to_solver(&slack.spill_cost.quantity.value)?)?)?;
            shortage_cost = finite(shortage_cost + finite(shortage * adapter.to_solver(&slack.shortage_cost.quantity.value)?)?)?;
            spills.insert((commodity.commodity.clone(), node.clone()), quantity(spill, unit)?);
            shortages.insert((commodity.commodity.clone(), node.clone()), quantity(shortage, unit)?);
        }
        for (node, actual) in balances {
            let expected = commodity.balances.get(&node).map(|balance| adapter.to_solver(&balance.value)).transpose()?.unwrap_or(0.0);
            check_bounds(actual, expected, expected)?;
        }
    }
    for arc in problem.arcs() {
        check_bounds(arc_totals[&arc.id], adapter.to_solver(&arc.capacity.lower.value)?, adapter.to_solver(&arc.capacity.upper.value)?)?;
    }
    let mut resources = BTreeMap::new();
    for resource in problem.resources() {
        let mut total = 0.0;
        for (key, usage) in &resource.consumptions {
            total = finite(total + finite(amounts.get(key).copied().unwrap_or(0.0) * adapter.to_solver(&usage.value)?)?)?;
        }
        check_bounds(total, adapter.to_solver(&resource.capacity.lower.value)?, adapter.to_solver(&resource.capacity.upper.value)?)?;
        resources.insert(resource.id.clone(), quantity(total, &resource.capacity.lower.unit)?);
    }
    let cost_unit = (&problem.units().cost_unit * &problem.units().flow_unit).build();
    let total_cost = finite(finite(operating_cost + spill_cost)? + shortage_cost)?;
    Ok(FlowSolution {
        flows, spills, shortages, resources,
        operating_cost: quantity(operating_cost, &cost_unit)?,
        spill_cost: quantity(spill_cost, &cost_unit)?,
        shortage_cost: quantity(shortage_cost, &cost_unit)?,
        total_cost: quantity(total_cost, &cost_unit)?,
    })
}
