//! CLIM 限制 / CLIM limits
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// CLIM 限制 / CLIM limit
/// 对齐 Kotlin CLIMLimit: 纵向力矩上下限约束
pub fn apply_clim_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
    _weights: &[usize],
    _occupied: &[usize],
) -> Result<(), Box<dyn Error>> {
    let Some(max_clim) = context.request.loading.max_clim else { return Ok(()); };
    // 纵向力矩下限
    model.add_linear_constraint(
        &aggregation.envelope_longitudinal_moment_coefficients,
        ConstraintRelation::GreaterEqual,
        -max_clim,
        &format!(
            "airworthiness_security_clim_min_{}",
            mode_name(context.mode)
        ),
    )?;
    // 纵向力矩上限
    model.add_linear_constraint(
        &aggregation.envelope_longitudinal_moment_coefficients,
        ConstraintRelation::LessEqual,
        max_clim,
        &format!(
            "airworthiness_security_clim_max_{}",
            mode_name(context.mode)
        ),
    )?;
    Ok(())
}
