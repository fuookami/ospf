//! 解提取逻辑 / Solution extraction logic

use std::collections::BTreeMap;

use ospf_rust_core::solver::SolveValue;

use crate::domain::material::{
    from_f64, shadow_price_unit_symbol, to_f64, Csp1dQuantity,
    ProductDemand, ProductId,
};
use crate::domain::r#yield::{
    ModeledOverProduction, ModeledUnderProduction, ProductOutput, YieldAnalysis,
};

use super::Produce;

/// 分析产出 yield / Analyze yield from produce
pub fn analyze_yield<V: SolveValue>(
    produce: &Produce<V>,
    demands: &[ProductDemand<V>],
) -> YieldAnalysis<V> {
    let mut supplied = BTreeMap::<(ProductId, String), (ProductDemand<V>, f64)>::new();
    for usage in &produce.cutting_plans {
        for contribution in &usage.plan.demand_contributions {
            let key = (
                contribution.product.id.clone(),
                shadow_price_unit_symbol(&contribution.quantity.unit),
            );
            let Some(value) = to_f64(&contribution.quantity.value) else {
                continue;
            };
            let demand_like = ProductDemand {
                product: contribution.product.clone(),
                quantity: contribution.quantity.clone(),
                mode: None,
            };
            supplied
                .entry(key)
                .and_modify(|(_, total)| *total += value * usage.amount as f64)
                .or_insert((demand_like, value * usage.amount as f64));
        }
    }
    let mut under_productions = Vec::new();
    let mut over_productions = Vec::new();
    let mut outputs = Vec::new();
    for demand in demands {
        let key = (
            demand.product.id.clone(),
            shadow_price_unit_symbol(&demand.quantity.unit),
        );
        let supplied_value = supplied.get(&key).map(|(_, total)| *total).unwrap_or(0.0);
        let required = to_f64(&demand.quantity.value).unwrap_or(f64::INFINITY);
        if supplied_value > 0.0 {
            if let Some(value) = from_f64(supplied_value) {
                outputs.push(ProductOutput {
                    product: demand.product.clone(),
                    total_quantity: Csp1dQuantity {
                        value,
                        unit: demand.quantity.unit.clone(),
                    },
                    mode: demand.mode,
                });
            }
        }
        if supplied_value + f64::EPSILON < required {
            if let Some(value) = from_f64(required - supplied_value) {
                under_productions.push(ModeledUnderProduction {
                    demand: demand.clone(),
                    shortfall: Csp1dQuantity {
                        value,
                        unit: demand.quantity.unit.clone(),
                    },
                });
            }
        } else if supplied_value > required + f64::EPSILON {
            if let Some(value) = from_f64(supplied_value - required) {
                over_productions.push(ModeledOverProduction {
                    demand: demand.clone(),
                    surplus: Csp1dQuantity {
                        value,
                        unit: demand.quantity.unit.clone(),
                    },
                });
            }
        }
    }
    YieldAnalysis {
        under_productions,
        over_productions,
        outputs,
    }
}
