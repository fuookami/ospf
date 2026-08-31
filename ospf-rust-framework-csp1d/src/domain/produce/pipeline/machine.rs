//! 默认设备约束管线 / Default machine constraint pipeline

use std::sync::Arc;

use ospf_rust_core::model::{ConstraintGroup, ConstraintRelation, MetaModel};
use ospf_rust_core::solver::SolveValue;
use ospf_rust_framework::model::Pipeline;

use crate::domain::material::{
    Csp1dShadowPriceKey, MachineBatchShadowPriceKey, MachineCapacityShadowPriceKey,
    shadow_price_key_to_string, to_f64,
};

use super::super::aggregation::ProduceAggregation;
use super::{Csp1dCGPipeline, Csp1dShadowPriceExtractor};

/// 默认设备约束管线 / Default machine constraint pipeline
#[derive(Debug, Clone)]
pub struct MachineConstraintPipeline<V: SolveValue> {
    name: String,
    group: Option<ConstraintGroup>,
    produce: ProduceAggregation<V>,
}

impl<V: SolveValue> MachineConstraintPipeline<V> {
    /// 创建设备约束管线 / Create machine constraint pipeline
    pub fn new(produce: ProduceAggregation<V>) -> Self {
        Self {
            name: "machine_constraint".to_string(),
            group: Some(ConstraintGroup::new(10_003, "csp1d_machine_constraint")),
            produce,
        }
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for MachineConstraintPipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        let Some(symbols) = self.produce.batch_symbols() else {
            log::warn!("Skip machine constraints: batch symbols not registered");
            return;
        };
        for (machine_index, machine) in self.produce.machines.iter().enumerate() {
            if let Some(max_batch_count) = machine.max_batch_count {
                let terms = symbols.machine_batch_terms(&machine.id);
                let key = Csp1dShadowPriceKey::MachineBatch(MachineBatchShadowPriceKey {
                    machine_id: machine.id.clone(),
                });
                if let Err(error) = model.add_linear_constraint_with_metadata(
                    &terms,
                    ConstraintRelation::LessEqual,
                    max_batch_count as f64,
                    &format!("machine_batch_{machine_index}"),
                    self.group.clone().map(Arc::new),
                    false,
                    0,
                    Some(shadow_price_key_to_string(&key)),
                ) {
                    log::warn!(
                        "Failed to register machine batch constraint {}: {:?}",
                        machine_index,
                        error
                    );
                }
            }

            let Some(capacity) = &machine.capacity else {
                continue;
            };
            let Some(rhs) = to_f64(&capacity.value) else {
                continue;
            };
            let terms = symbols.machine_capacity_terms(&machine.id);
            if terms.is_empty() {
                continue;
            }
            let key = Csp1dShadowPriceKey::MachineCapacity(MachineCapacityShadowPriceKey {
                machine_id: machine.id.clone(),
            });
            if let Err(error) = model.add_linear_constraint_with_metadata(
                &terms,
                ConstraintRelation::LessEqual,
                rhs,
                &format!("machine_capacity_{machine_index}"),
                self.group.clone().map(Arc::new),
                false,
                0,
                Some(shadow_price_key_to_string(&key)),
            ) {
                log::warn!(
                    "Failed to register machine capacity constraint {}: {:?}",
                    machine_index,
                    error
                );
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

impl<V: SolveValue> Csp1dCGPipeline<V> for MachineConstraintPipeline<V> {
    fn shadow_price_extractor(&self) -> Option<Csp1dShadowPriceExtractor<V>> {
        if self.produce.machines.is_empty() {
            return None;
        }
        Some(Arc::new(move |map, plan| {
            let Some(machine_id) = plan.machine_id.as_deref() else {
                return 0.0;
            };
            let batch_key = Csp1dShadowPriceKey::MachineBatch(MachineBatchShadowPriceKey {
                machine_id: machine_id.into(),
            });
            let capacity_key =
                Csp1dShadowPriceKey::MachineCapacity(MachineCapacityShadowPriceKey {
                    machine_id: machine_id.into(),
                });
            let capacity_consumption = plan
                .capacity_consumption
                .as_ref()
                .and_then(|quantity| to_f64(&quantity.value))
                .unwrap_or(0.0);
            map.get(&batch_key).unwrap_or(0.0)
                + map.get(&capacity_key).unwrap_or(0.0) * capacity_consumption
        }))
    }
}
