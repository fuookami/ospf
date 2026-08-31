//! 约束与目标管线 / Constraint and objective pipelines

pub mod demand;
pub mod length;
pub mod machine;
pub mod material;
pub mod r#yield;

use std::sync::Arc;

use ospf_rust_core::model::{ConstraintGroup, MetaModel};
use ospf_rust_core::solver::SolveValue;
use ospf_rust_framework::model::Pipeline;

use crate::domain::material::{CuttingPlan, shadow_price_key_from_string, to_f64};
use crate::domain::wasting_minimization::RestMaterialMeasure;

use super::shadow_price::Csp1dDefaultShadowPriceMap;

pub use demand::*;
pub use length::*;
pub use machine::*;
pub use material::*;
pub use r#yield::*;

/// CSP1D 影子价格提取器 / CSP1D shadow price extractor
pub type Csp1dShadowPriceExtractor<V> =
    Arc<dyn Fn(&Csp1dDefaultShadowPriceMap, &CuttingPlan<V>) -> f64 + Send + Sync>;

/// CSP1D 列生成管线 / CSP1D column-generation pipeline
pub trait Csp1dCGPipeline<V: SolveValue>: Pipeline<MetaModel<f64>> {
    /// 刷新影子价格 / Refresh shadow prices
    fn refresh_shadow_price(
        &self,
        shadow_price_map: &mut Csp1dDefaultShadowPriceMap,
        model: &MetaModel<f64>,
        shadow_prices: &[f64],
    ) -> crate::Csp1dResult<()> {
        refresh_shadow_price_by_key_as_args(
            self.constraint_group(),
            shadow_price_map,
            model,
            shadow_prices,
        )
    }

    /// 获取方案级影子价格提取器 / Get plan-level shadow price extractor
    fn shadow_price_extractor(&self) -> Option<Csp1dShadowPriceExtractor<V>> {
        None
    }
}

fn refresh_shadow_price_by_key_as_args(
    group: Option<&ConstraintGroup>,
    shadow_price_map: &mut Csp1dDefaultShadowPriceMap,
    model: &MetaModel<f64>,
    shadow_prices: &[f64],
) -> crate::Csp1dResult<()> {
    for (constraint_index, constraint) in model.constraints().iter().enumerate() {
        if let Some(group) = group {
            if constraint.group.as_ref().map(|value| value.id) != Some(group.id) {
                continue;
            }
        }
        let Some(args) = constraint.args.as_deref() else {
            continue;
        };
        let Some(key) = shadow_price_key_from_string(args) else {
            continue;
        };
        let Some(value) = shadow_prices.get(constraint_index).copied() else {
            continue;
        };
        shadow_price_map.put_or_add(key, value);
    }
    Ok(())
}

/// 增量扩展管线 / Incremental extension pipeline
pub trait Csp1dIncrementalPipeline<V: SolveValue>: Pipeline<MetaModel<f64>> {
    /// 向模型添加新列并更新增量约束 / Add new columns to the model and update incremental constraints
    fn add_columns(
        &self,
        _context: &dyn super::Csp1dModelingContext<V>,
        _iteration: u64,
        new_plans: Vec<CuttingPlan<V>>,
        _model: &mut MetaModel<f64>,
    ) -> crate::Csp1dResult<Vec<CuttingPlan<V>>> {
        Ok(new_plans)
    }
}

/// 计算剩余物料值 / Compute rest material value
pub fn rest_material_value<V: SolveValue>(
    plan: &CuttingPlan<V>,
    measure: RestMaterialMeasure,
) -> Option<f64> {
    match measure {
        RestMaterialMeasure::RestWidthByMaterialLengthProxy => {
            let rest_width = plan.rest_width().and_then(|width| to_f64(&width.value))?;
            let material_length = plan
                .material
                .length
                .as_ref()
                .and_then(|length| to_f64(&length.value))?;
            Some(rest_width * material_length)
        }
    }
}
