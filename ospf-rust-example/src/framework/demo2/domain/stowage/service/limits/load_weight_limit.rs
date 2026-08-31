//! 装载重量限制 / Load weight limits
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::load::LoadVariables;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 装载重量限制: actualLoadWeight[j] <= position.max_weight
/// 对齐 Kotlin LoadWeightLimit
pub fn apply_load_weight_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    load_vars: &LoadVariables,
) -> Result<(), Box<dyn Error>> {
    for p in 0..context.request.positions.len() {
        // actualLoadWeight[j] <= max_weight
        model.add_linear_constraint(
            &[(load_vars.actual_load_weight[p], 1.0)],
            ConstraintRelation::LessEqual,
            context.request.positions[p].max_weight,
            &format!("stowage_load_weight_{}_{}", mode_name(context.mode), p),
        )?;
    }
    Ok(())
}
