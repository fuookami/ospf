//! 装载数量限制 / Load amount limits
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::load::LoadVariables;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 装载数量限制: loadAmount[j] <= position.max_load_count
/// 对齐 Kotlin LoadAmountLimit
pub fn apply_load_amount_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    load_vars: &LoadVariables,
) -> Result<(), Box<dyn Error>> {
    for p in 0..context.request.positions.len() {
        let max_count = context.request.positions[p].max_load_count as f64;
        if max_count > 0.0 {
            // loadAmount[j] <= max_load_count
            model.add_linear_constraint(
                &[(load_vars.load_amount[p], 1.0)],
                ConstraintRelation::LessEqual,
                max_count,
                &format!("stowage_load_amount_{}_{}", mode_name(context.mode), p),
            )?;
        }
    }
    Ok(())
}
