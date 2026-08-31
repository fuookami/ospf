//! 包络线限制 / Envelope limits
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 包络线限制 / Envelope limit
///
/// 对齐 Kotlin EnvelopeLimit: 纵向力矩在包络线范围内。
/// Aligns with Kotlin EnvelopeLimit: longitudinal moment within envelope bounds.
pub fn apply_envelope_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
    _estimate_load_weight_idx: &[usize],
    _estimate_loaded_idx: &[usize],
) -> Result<(), Box<dyn Error>> {
    model.add_linear_constraint(
        &aggregation.envelope_longitudinal_moment_coefficients,
        ConstraintRelation::GreaterEqual,
        context.request.envelope_longitudinal_moment_min,
        &format!(
            "airworthiness_security_envelope_longitudinal_min_{}",
            mode_name(context.mode)
        ),
    )?;
    model.add_linear_constraint(
        &aggregation.envelope_longitudinal_moment_coefficients,
        ConstraintRelation::LessEqual,
        context.request.envelope_longitudinal_moment_max,
        &format!(
            "airworthiness_security_envelope_longitudinal_max_{}",
            mode_name(context.mode)
        ),
    )?;
    Ok(())
}
