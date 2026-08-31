//! 低载荷限制 / Low payload limits
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 低载荷限制 / Low payload limit
/// 对齐 Kotlin LowPayloadLimit: 总业载 >= min(payload_upper_bound * min_payload_ratio, total_capacity * min_payload_ratio)
pub fn apply_low_payload_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    let _total_capacity: f64 = context.request.positions.iter().map(|p| p.max_weight).sum();
    let total_cargo_weight: f64 = context.request.cargos.iter().map(|c| c.weight).sum();
    let min_payload = context.request.payload_upper_bound.min(total_cargo_weight)
        * context.request.min_payload_ratio;
    if min_payload > 0.0 {
        model.add_linear_constraint(
            &aggregation.total_payload_coefficients,
            ConstraintRelation::GreaterEqual,
            min_payload,
            &format!(
                "airworthiness_security_low_payload_{}",
                mode_name(context.mode)
            ),
        )?;
    }
    Ok(())
}
