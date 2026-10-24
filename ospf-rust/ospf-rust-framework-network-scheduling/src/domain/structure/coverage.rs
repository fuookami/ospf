//! 合格设施分区归属，不隐含运输或连通规则。 / Eligible facility ownership without implicit transportation or connectivity rules.

use std::collections::BTreeMap;
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::variable::{Binary, VariableRange};
use crate::domain::design::NetworkConstruction;
use crate::domain::design::pipeline::{LinearRow, register_row, validate_rows};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::NetworkNodeId;
use super::model::NetworkStructureProblem;
use super::tree::binary;

/// 分区覆盖及设施数量配额组件。 / Partition coverage and facility assignment-count quota component.
#[derive(Debug, Clone, Default)]
pub struct FacilityCoverageComponent {
    assignments: BTreeMap<(String, NetworkNodeId), usize>,
    rows: Vec<LinearRow>,
    binding: Option<u64>,
}

impl FacilityCoverageComponent {
    /// 合格分区-设施组合的二元变量。 / Binary variables for eligible partition-facility pairs.
    pub fn assignment_variables(&self) -> &BTreeMap<(String, NetworkNodeId), usize> { &self.assignments }

    /// 已注册的合格归属表达式；不合格组合返回错误。 / Registered eligible-assignment expression; ineligible pairs return an error.
    pub fn assignment_expression(&self, partition: &str, facility: &NetworkNodeId) -> Result<Vec<(usize, f64)>> {
        let index = self.assignments.get(&(partition.to_owned(), facility.clone()))
            .ok_or_else(|| NetworkSchedulingError::structure("分区-设施组合不合格或尚未注册 / ineligible or unregistered partition-facility pair"))?;
        Ok(vec![(*index, 1.0)])
    }

    pub(super) fn register<V>(
        &mut self,
        model: &mut MetaModel<f64>,
        construction: &NetworkConstruction<V>,
        problem: &NetworkStructureProblem<V>,
        name: &str,
    ) -> Result<()>
    where
        V: SolveValue + UnitConversionValue,
    {
        for partition in problem.partitions() {
            let mut terms = Vec::new();
            for facility in &partition.eligible_facilities {
                let index = model.register_auto_variable_with_range::<Binary>(
                    &format!("{name}_partition_{}_facility_{facility}", partition.id), VariableRange::bounded(0.0, 1.0),
                ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                self.assignments.insert((partition.id.clone(), facility.clone()), index);
                terms.push((index, 1.0));
                if let Some(open) = construction.facility_variable(facility) {
                    self.rows.push(register_row(model, vec![(index, 1.0), (open, -1.0)], ConstraintRelation::LessEqual,
                        0.0, &format!("{name}_partition_{}_open_{facility}", partition.id))?);
                }
            }
            self.rows.push(register_row(model, terms,
                if partition.required { ConstraintRelation::Equal } else { ConstraintRelation::LessEqual },
                1.0, &format!("{name}_partition_{}_coverage", partition.id))?);
        }
        for quota in problem.quotas() {
            let terms = self.assignments.iter().filter(|((_, facility), _)| facility == &quota.facility)
                .map(|(_, index)| (*index, 1.0)).collect::<Vec<_>>();
            self.rows.push(register_row(model, terms.clone(), ConstraintRelation::GreaterEqual, quota.minimum as f64,
                &format!("{name}_quota_{}_minimum", quota.facility))?);
            if let Some(maximum) = quota.maximum {
                self.rows.push(register_row(model, terms, ConstraintRelation::LessEqual, maximum as f64,
                    &format!("{name}_quota_{}_maximum", quota.facility))?);
            }
        }
        self.binding = Some(model.model_identity());
        Ok(())
    }

    pub(super) fn extract<V>(
        &self,
        model: &MetaModel<f64>,
        problem: &NetworkStructureProblem<V>,
    ) -> Result<BTreeMap<String, NetworkNodeId>>
    where
        V: SolveValue + UnitConversionValue,
    {
        if self.binding != Some(model.model_identity()) {
            return Err(NetworkSchedulingError::contract("设施覆盖未注册或模型不匹配 / unregistered facility coverage or model mismatch"));
        }
        validate_rows(model, &self.rows)?;
        let mut owners = BTreeMap::new();
        for ((partition, facility), index) in &self.assignments {
            if binary(model, *index)? && owners.insert(partition.clone(), facility.clone()).is_some() {
                return Err(NetworkSchedulingError::contract("分区有多个所有者 / partition has multiple owners"));
            }
        }
        for partition in problem.partitions() {
            if partition.required && !owners.contains_key(&partition.id) {
                return Err(NetworkSchedulingError::contract("必需分区没有所有者 / required partition has no owner"));
            }
        }
        for quota in problem.quotas() {
            let count = owners.values().filter(|owner| *owner == &quota.facility).count();
            if count < quota.minimum || quota.maximum.is_some_and(|maximum| count > maximum) {
                return Err(NetworkSchedulingError::contract("分区归属违反设施数量配额 / partition ownership violates a facility assignment-count quota"));
            }
        }
        Ok(owners)
    }
}
