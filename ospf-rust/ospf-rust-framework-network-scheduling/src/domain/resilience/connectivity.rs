//! 结构辅助流，不借用商品供需或故障场景容量证明。 / Structural auxiliary flows independent of commodity demand and failure-capacity proofs.

use std::collections::BTreeMap;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::variable::{Continuous, VariableRange};
use ospf_rust_quantities::unit::UnitConversionValue;
use crate::domain::design::NetworkConstruction;
use crate::domain::design::pipeline::{LinearRow, register_row, validate_rows};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkSchedulingSolverValueAdapter};
use super::model::{ConnectivityKind, ConnectivityRequirement};

#[derive(Debug, Clone, Default)]
pub(super) struct ConnectivityFlow {
    variables: BTreeMap<NetworkArcId, usize>,
    rows: Vec<LinearRow>,
    upper: f64,
}

impl ConnectivityFlow {
    pub(super) fn register<V>(
        model: &mut MetaModel<f64>,
        construction: &NetworkConstruction<V>,
        requirement: &ConnectivityRequirement,
        name: &str,
    ) -> Result<Self>
    where
        V: SolveValue + UnitConversionValue,
    {
        let problem = construction.problem().flow();
        let amount = requirement.paths as f64;
        let upper = if requirement.kind == ConnectivityKind::Reachable { amount } else { 1.0 };
        let mut witness = Self { upper, ..Default::default() };
        for arc in problem.arcs().iter().filter(|arc| arc.from != arc.to) {
            let index = model.register_auto_variable_with_range::<Continuous>(
                &format!("{name}_{}", arc.id), VariableRange::bounded(0.0, upper),
            ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
            witness.variables.insert(arc.id.clone(), index);
            if let Some(levels) = construction.service_variables(&arc.id) {
                let mut terms = vec![(index, 1.0)];
                terms.extend(levels.iter().map(|level| (*level, -upper)));
                witness.rows.push(register_row(model, terms, ConstraintRelation::LessEqual, 0.0,
                    &format!("{name}_{}_service", arc.id))?);
            }
            for node in [&arc.from, &arc.to] {
                if let Some(facility) = construction.facility_variable(node) {
                    witness.rows.push(register_row(model, vec![(index, 1.0), (facility, -upper)], ConstraintRelation::LessEqual, 0.0,
                        &format!("{name}_{}_{node}_facility", arc.id))?);
                }
            }
        }
        for node in problem.nodes() {
            let mut balance = Vec::new();
            let mut incoming = Vec::new();
            for arc in problem.arcs().iter().filter(|arc| arc.from != arc.to) {
                let index = witness.variables[&arc.id];
                if arc.from == node.id { balance.push((index, 1.0)); }
                if arc.to == node.id { balance.push((index, -1.0)); incoming.push((index, 1.0)); }
            }
            let rhs = if node.id == requirement.source { amount } else if node.id == requirement.target { -amount } else { 0.0 };
            witness.rows.push(register_row(model, balance, ConstraintRelation::Equal, rhs, &format!("{name}_balance_{}", node.id))?);
            if requirement.kind == ConnectivityKind::NodeDisjoint && node.id != requirement.source && node.id != requirement.target {
                witness.rows.push(register_row(model, incoming, ConstraintRelation::LessEqual, 1.0, &format!("{name}_node_{}", node.id))?);
            }
        }
        Ok(witness)
    }

    pub(super) fn extract<V: SolveValue + UnitConversionValue>(&self, model: &MetaModel<f64>) -> Result<BTreeMap<NetworkArcId, V>> {
        validate_rows(model, &self.rows)?;
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        self.variables.iter().map(|(id, index)| {
            let value = model.tokens().get(*index).and_then(|token| token.get_result())
                .ok_or_else(|| NetworkSchedulingError::contract("缺少连通见证结果 / missing connectivity witness result"))?;
            if !value.is_finite() || value < 0.0 || value > self.upper + 1e-7 {
                return Err(NetworkSchedulingError::contract("连通辅助流结果越界 / connectivity flow result out of bounds"));
            }
            Ok((id.clone(), adapter.from_solver(value)?))
        }).collect()
    }
}
