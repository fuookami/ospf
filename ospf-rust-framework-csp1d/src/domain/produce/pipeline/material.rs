//! 默认物料约束管线 / Default material constraint pipeline

use std::sync::Arc;

use ospf_rust_core::model::{ConstraintGroup, ConstraintRelation, MetaModel};
use ospf_rust_core::solver::SolveValue;
use ospf_rust_framework::model::Pipeline;

use crate::domain::material::{
    Csp1dShadowPriceKey, MaterialUsageShadowPriceKey, shadow_price_key_to_string,
};

use super::super::aggregation::ProduceAggregation;
use super::{Csp1dCGPipeline, Csp1dShadowPriceExtractor};

/// 默认物料约束管线 / Default material constraint pipeline
#[derive(Debug, Clone)]
pub struct MaterialConstraintPipeline<V: SolveValue> {
    name: String,
    group: Option<ConstraintGroup>,
    produce: ProduceAggregation<V>,
}

impl<V: SolveValue> MaterialConstraintPipeline<V> {
    /// 创建物料约束管线 / Create material constraint pipeline
    pub fn new(produce: ProduceAggregation<V>) -> Self {
        Self {
            name: "material_constraint".to_string(),
            group: Some(ConstraintGroup::new(10_002, "csp1d_material_constraint")),
            produce,
        }
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for MaterialConstraintPipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        let Some(symbols) = self.produce.batch_symbols() else {
            log::warn!("Skip material constraints: batch symbols not registered");
            return;
        };
        for (material_index, material) in self.produce.materials.iter().enumerate() {
            if material.available_batches == u64::MAX {
                continue;
            }
            let terms = symbols.material_terms(&material.id);
            let key = Csp1dShadowPriceKey::MaterialUsage(MaterialUsageShadowPriceKey {
                material_id: material.id.clone(),
            });
            if let Err(error) = model.add_linear_constraint_with_metadata(
                &terms,
                ConstraintRelation::LessEqual,
                material.available_batches as f64,
                &format!("material_{material_index}"),
                self.group.clone().map(Arc::new),
                false,
                0,
                Some(shadow_price_key_to_string(&key)),
            ) {
                log::warn!(
                    "Failed to register material constraint {}: {:?}",
                    material_index,
                    error
                );
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

impl<V: SolveValue> Csp1dCGPipeline<V> for MaterialConstraintPipeline<V> {
    fn shadow_price_extractor(&self) -> Option<Csp1dShadowPriceExtractor<V>> {
        if self.produce.materials.is_empty() {
            return None;
        }
        Some(Arc::new(move |map, plan| {
            let key = Csp1dShadowPriceKey::MaterialUsage(MaterialUsageShadowPriceKey {
                material_id: plan.material.id.clone(),
            });
            map.get(&key).unwrap_or(0.0)
        }))
    }
}
