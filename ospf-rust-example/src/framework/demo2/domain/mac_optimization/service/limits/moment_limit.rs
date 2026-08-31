use std::error::Error;

use ospf_rust_core::model::{ConstraintRelation, MetaModel};

use crate::framework::demo2::domain::mac_optimization::aggregation::MacOptimizationAggregation;
use crate::framework::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

pub fn apply_moment_limits(
    model: &mut MetaModel<f64>,
    context: &MacOptimizationContext<'_>,
    aggregation: &MacOptimizationAggregation,
) -> Result<(), Box<dyn Error>> {
    let mut long_upper = aggregation.long_moment.clone();
    let mut long_lower = aggregation.neg_long_moment.clone();
    if let Some(z_idx) = context.z {
        long_upper.push((z_idx, -aggregation.max_arm_abs));
        long_lower.push((z_idx, -aggregation.max_arm_abs));
    }
    model.add_linear_constraint(
        &long_upper,
        ConstraintRelation::LessEqual,
        context.request.target_longitudinal_moment + context.request.max_longitudinal_moment_deviation,
        &format!("mac_longitudinal_upper_{}", mode_name(context.mode)),
    )?;
    model.add_linear_constraint(
        &long_lower,
        ConstraintRelation::LessEqual,
        -context.request.target_longitudinal_moment + context.request.max_longitudinal_moment_deviation,
        &format!("mac_longitudinal_lower_{}", mode_name(context.mode)),
    )?;

    model.add_linear_constraint(
        &aggregation.lat_moment,
        ConstraintRelation::LessEqual,
        context.request.max_lateral_imbalance,
        &format!("mac_lateral_upper_{}", mode_name(context.mode)),
    )?;
    model.add_linear_constraint(
        &aggregation.neg_lat_moment,
        ConstraintRelation::LessEqual,
        context.request.max_lateral_imbalance,
        &format!("mac_lateral_lower_{}", mode_name(context.mode)),
    )?;

    Ok(())
}

