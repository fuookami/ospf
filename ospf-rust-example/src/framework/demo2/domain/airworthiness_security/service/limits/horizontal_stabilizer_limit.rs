//! 水平安定面限制 / Horizontal stabilizer limits
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 水平安定面限制: 纵向力矩在 envelope 范围内
/// 对齐 Kotlin HorizontalStabilizerLimit
pub fn apply_horizontal_stabilizer_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 水平安定面约束: 纵向力矩在 envelope 范围内
    // 使用 envelope_longitudinal_moment_min/max 作为限制
    model.add_linear_constraint(
        &aggregation.envelope_longitudinal_moment_coefficients,
        ConstraintRelation::GreaterEqual,
        context.request.envelope_longitudinal_moment_min,
        &format!(
            "airworthiness_security_horizontal_stabilizer_min_{}",
            mode_name(context.mode)
        ),
    )?;
    model.add_linear_constraint(
        &aggregation.envelope_longitudinal_moment_coefficients,
        ConstraintRelation::LessEqual,
        context.request.envelope_longitudinal_moment_max,
        &format!(
            "airworthiness_security_horizontal_stabilizer_max_{}",
            mode_name(context.mode)
        ),
    )?;
    Ok(())
}
