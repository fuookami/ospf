//! CLIM 限制 / CLIM limits
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// CLIM 限制 / CLIM limit
/// 对齐 Kotlin CLIMLimit: 纵向力矩上下限约束
pub fn apply_clim_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 纵向力矩下限
    model.add_linear_constraint(
        &aggregation.envelope_longitudinal_moment_coefficients,
        ConstraintRelation::GreaterEqual,
        context.request.envelope_longitudinal_moment_min,
        &format!("airworthiness_security_clim_min_{}", mode_name(context.mode)),
    )?;
    // 纵向力矩上限
    model.add_linear_constraint(
        &aggregation.envelope_longitudinal_moment_coefficients,
        ConstraintRelation::LessEqual,
        context.request.envelope_longitudinal_moment_max,
        &format!("airworthiness_security_clim_max_{}", mode_name(context.mode)),
    )?;
    Ok(())
}
