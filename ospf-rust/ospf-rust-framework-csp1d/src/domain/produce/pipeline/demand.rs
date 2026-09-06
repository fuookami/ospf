//! 默认需求约束管线 / Default demand constraint pipeline

use std::sync::Arc;

use ospf_rust_core::model::{ConstraintGroup, ConstraintRelation, MetaModel};
use ospf_rust_core::solver::SolveValue;
use ospf_rust_framework::model::Pipeline;

use crate::domain::material::{
    Csp1dShadowPriceKey, ProductDemandShadowPriceKey, shadow_price_key_to_string,
    shadow_price_unit_symbol, to_f64,
};

use super::super::aggregation::ProduceAggregation;
use super::{Csp1dCGPipeline, Csp1dShadowPriceExtractor};

/// 默认需求约束管线 / Default demand constraint pipeline
#[derive(Debug, Clone)]
pub struct DemandConstraintPipeline<V: SolveValue> {
    name: String,
    group: Option<ConstraintGroup>,
    produce: ProduceAggregation<V>,
}

impl<V: SolveValue> DemandConstraintPipeline<V> {
    /// 创建需求约束管线 / Create demand constraint pipeline
    pub fn new(produce: ProduceAggregation<V>) -> Self {
        Self {
            name: "demand_constraint".to_string(),
            group: Some(ConstraintGroup::new(10_001, "csp1d_demand_constraint")),
            produce,
        }
    }
}

impl<V: SolveValue> Pipeline<MetaModel<f64>> for DemandConstraintPipeline<V> {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        self.group.as_ref()
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        let Some(symbols) = self.produce.batch_symbols() else {
            log::warn!("Skip demand constraints: batch symbols not registered");
            return;
        };
        for (demand_index, demand) in self.produce.demands.iter().enumerate() {
            let terms = symbols.demand_terms(&demand.product.id, &demand.quantity.unit.symbol());
            let Some(rhs) = to_f64(&demand.quantity.value) else {
                log::warn!(
                    "Skip demand constraint {} due to non-convertible rhs",
                    demand_index
                );
                continue;
            };
            let key = Csp1dShadowPriceKey::ProductDemand(ProductDemandShadowPriceKey {
                product_id: demand.product.id.clone(),
                unit_symbol: shadow_price_unit_symbol(&demand.quantity.unit),
            });
            if let Err(error) = model.add_linear_constraint_with_metadata(
                &terms,
                ConstraintRelation::GreaterEqual,
                rhs,
                &format!("demand_{demand_index}"),
                self.group.clone().map(Arc::new),
                false,
                0,
                Some(shadow_price_key_to_string(&key)),
            ) {
                log::warn!(
                    "Failed to register demand constraint {}: {:?}",
                    demand_index,
                    error
                );
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> ospf_rust_core::error::Result<()> {
        Ok(())
    }
}

impl<V: SolveValue> Csp1dCGPipeline<V> for DemandConstraintPipeline<V> {
    fn shadow_price_extractor(&self) -> Option<Csp1dShadowPriceExtractor<V>> {
        let demands = self.produce.demands.clone();
        if demands.is_empty() {
            return None;
        }
        Some(Arc::new(move |map, plan| {
            demands
                .iter()
                .map(|demand| {
                    let key = Csp1dShadowPriceKey::ProductDemand(ProductDemandShadowPriceKey {
                        product_id: demand.product.id.clone(),
                        unit_symbol: shadow_price_unit_symbol(&demand.quantity.unit),
                    });
                    let shadow_price = map.get(&key).unwrap_or(0.0);
                    let contribution = plan
                        .demand_contributions
                        .iter()
                        .filter(|contribution| {
                            contribution.product.id == demand.product.id
                                && contribution.quantity.unit == demand.quantity.unit
                        })
                        .filter_map(|contribution| to_f64(&contribution.quantity.value))
                        .sum::<f64>();
                    shadow_price * contribution
                })
                .sum()
        }))
    }
}
