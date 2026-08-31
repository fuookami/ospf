//! 载荷限制 / Payload limits
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 业载限制 / Payload limit
///
/// 对齐 Kotlin PayloadLimit: 总业载在上下限范围内。
/// Aligns with Kotlin PayloadLimit: total payload within upper and lower bounds.
pub fn apply_payload_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
    _estimate_load_weight_idx: &[usize],
    _estimate_loaded_idx: &[usize],
) -> Result<(), Box<dyn Error>> {
    let total_cargo_weight: f64 = context
        .request
        .cargos
        .iter()
        .map(|cargo| cargo.weight)
        .sum();
    let min_payload = context.request.payload_upper_bound.min(total_cargo_weight)
        * context.request.min_payload_ratio;
    model.add_linear_constraint(
        &aggregation.total_payload_coefficients,
        ConstraintRelation::LessEqual,
        context.request.payload_upper_bound,
        &format!(
            "airworthiness_security_payload_upper_{}",
            mode_name(context.mode)
        ),
    )?;
    model.add_linear_constraint(
        &aggregation.total_payload_coefficients,
        ConstraintRelation::GreaterEqual,
        min_payload,
        &format!(
            "airworthiness_security_payload_lower_{}",
            mode_name(context.mode)
        ),
    )?;
    Ok(())
}
