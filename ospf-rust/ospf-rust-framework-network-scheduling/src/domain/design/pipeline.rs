//! 有限包络联动与独立行校验。 / Finite-envelope linking and independent row validation.

use std::collections::BTreeMap;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::UnitConversionValue;
use crate::domain::flow::{FlowContext, FlowProblem};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkSchedulingSolverValueAdapter};
use super::construction::NetworkConstruction;

#[derive(Debug, Clone)]
pub(crate) struct LinearRow {
    terms: Vec<(usize, f64)>,
    relation: ConstraintRelation,
    rhs: f64,
}

pub(crate) fn register_row(
    model: &mut MetaModel<f64>,
    terms: Vec<(usize, f64)>,
    relation: ConstraintRelation,
    rhs: f64,
    name: &str,
) -> Result<LinearRow> {
    if !rhs.is_finite() || terms.iter().any(|(_, value)| !value.is_finite()) {
        return Err(NetworkSchedulingError::validation("联动行必须有限 / linking rows must be finite"));
    }
    model.add_linear_constraint(&terms, relation, rhs, name)
        .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
    Ok(LinearRow { terms, relation, rhs })
}

pub(crate) fn validate_rows(model: &MetaModel<f64>, rows: &[LinearRow]) -> Result<()> {
    for row in rows {
        let mut actual = 0.0;
        for (index, coefficient) in &row.terms {
            let value = model.tokens().get(*index).and_then(|token| token.get_result())
                .ok_or_else(|| NetworkSchedulingError::contract("缺少联动变量结果 / missing linked variable result"))?;
            actual += coefficient * value;
            if !value.is_finite() || !actual.is_finite() {
                return Err(NetworkSchedulingError::contract("联动结果必须有限 / linked results must be finite"));
            }
        }
        let residual = actual - row.rhs;
        let delta = 1e-7 + 1e-7 * actual.abs().max(row.rhs.abs());
        let valid = residual.is_finite() && match row.relation {
            ConstraintRelation::Equal => residual.abs() <= delta,
            ConstraintRelation::LessEqual => residual <= delta,
            ConstraintRelation::GreaterEqual => residual >= -delta,
        };
        if !valid { return Err(NetworkSchedulingError::contract("结果违反建设联动行 / result violates a construction linking row")); }
    }
    Ok(())
}

// 下界独立注册，避免降级小于硬下界时构造非法区间。 / Register lower bounds independently to avoid invalid intervals when degradation falls below a hard lower bound.
pub(crate) fn embedded_flow<V>(problem: &FlowProblem<V>) -> Result<FlowProblem<V>>
where
    V: SolveValue + UnitConversionValue,
{
    let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
    let mut arcs = problem.arcs().to_vec();
    for arc in &mut arcs { arc.capacity.lower.value = adapter.from_solver(0.0)?; }
    let mut resources = problem.resources().to_vec();
    for resource in &mut resources { resource.capacity.lower.value = adapter.from_solver(0.0)?; }
    FlowProblem::new_with_units(problem.nodes().to_vec(), arcs, problem.commodities().to_vec(), problem.units().clone())?
        .with_resources(resources)
}

pub(crate) fn link_flow<V>(
    model: &mut MetaModel<f64>,
    construction: &NetworkConstruction<V>,
    flow: &FlowContext<V>,
    declared: &FlowProblem<V>,
    hard_lowers: &BTreeMap<NetworkArcId, Quantity<V, ospf_rust_quantities::unit::Unit>>,
    name: &str,
) -> Result<Vec<LinearRow>>
where
    V: SolveValue + UnitConversionValue,
{
    construction.validate_binding(model)?;
    let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
    let nominal = construction.problem().flow();
    let mut rows = Vec::new();
    for arc in declared.arcs() {
        let original = nominal.arc(&arc.id).ok_or_else(|| NetworkSchedulingError::structure("场景引用未知弧 / scenario references an unknown arc"))?;
        if original.from != arc.from || original.to != arc.to {
            return Err(NetworkSchedulingError::structure("场景不能更改稳定弧端点 / scenario cannot change stable arc endpoints"));
        }
        let upper = adapter.normalize(&arc.capacity.upper, &nominal.units().flow_unit)?;
        if upper > adapter.to_solver(&original.capacity.upper.value)? {
            return Err(NetworkSchedulingError::validation("场景不能超过名义容量包络 / scenario exceeds the nominal capacity envelope"));
        }
        let lower = adapter.normalize(hard_lowers.get(&arc.id).unwrap_or(&arc.capacity.lower), &nominal.units().flow_unit)?;
        if lower < 0.0 { return Err(NetworkSchedulingError::validation("硬下界不能为负 / hard lower bounds must be nonnegative")); }
        let load = flow.aggregation.capacity_expression(&arc.id)?;
        let mut lower_terms = load.clone();
        let mut lower_rhs = lower;
        if let Some(variables) = construction.service_variables(&arc.id) {
            let service = construction.problem().services().iter().find(|item| item.arc == arc.id)
                .ok_or_else(|| NetworkSchedulingError::contract("缺少已注册通道 / missing registered service"))?;
            let mut capacity_terms = load.clone();
            for (level, index) in service.levels.iter().zip(variables) {
                capacity_terms.push((*index, -adapter.normalize(&level.capacity, &nominal.units().flow_unit)?));
                lower_terms.push((*index, -lower));
            }
            lower_rhs = 0.0;
            rows.push(register_row(model, capacity_terms, ConstraintRelation::LessEqual, 0.0,
                &format!("{name}_{}_built", arc.id))?);
        }
        rows.push(register_row(model, lower_terms, ConstraintRelation::GreaterEqual, lower_rhs,
            &format!("{name}_{}_hard_lower", arc.id))?);
        // Flow 自身仍保留场景上界；建设等级从不替换它。 / Flow retains its own scenario upper bound; construction levels never replace it.
        for node in [&arc.from, &arc.to] {
            if let Some(index) = construction.facility_variable(node) {
                let mut terms = load.clone();
                terms.push((index, -upper));
                rows.push(register_row(model, terms, ConstraintRelation::LessEqual, 0.0,
                    &format!("{name}_{}_facility_{node}", arc.id))?);
            }
        }
    }
    for resource in declared.resources() {
        let load = flow.aggregation.resource_expression(&resource.id)?;
        let lower = adapter.to_solver(&resource.capacity.lower.value)?;
        let upper = adapter.to_solver(&resource.capacity.upper.value)?;
        let owners = construction.problem().facilities().iter().filter(|item| item.capacity_rows.contains(&resource.id)).collect::<Vec<_>>();
        if owners.is_empty() {
            rows.push(register_row(model, load, ConstraintRelation::GreaterEqual, lower,
                &format!("{name}_resource_{}_lower", resource.id))?);
        } else {
            for facility in owners {
                let index = construction.facility_variable(&facility.node)
                    .ok_or_else(|| NetworkSchedulingError::contract("设施尚未注册 / facility is not registered"))?;
                for (bound, relation, suffix) in [(lower, ConstraintRelation::GreaterEqual, "lower"), (upper, ConstraintRelation::LessEqual, "upper")] {
                    let mut terms = load.clone();
                    terms.push((index, -bound));
                    rows.push(register_row(model, terms, relation, 0.0,
                        &format!("{name}_resource_{}_{}_{suffix}", resource.id, facility.node))?);
                }
            }
        }
    }
    Ok(rows)
}
