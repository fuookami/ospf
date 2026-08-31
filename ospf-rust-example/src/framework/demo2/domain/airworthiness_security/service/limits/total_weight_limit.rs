//! 总重限制 / Total weight limits
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 总重限制 / Total weight limit
/// 对齐 Kotlin TotalWeightLimit: 每个舱位的载荷不超过最大载荷限制
pub fn apply_total_weight_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    for p in 0..context.request.positions.len() {
        if aggregation.per_position_weight_coefficients[p].is_empty() {
            continue;
        }
        model.add_linear_constraint(
            &aggregation.per_position_weight_coefficients[p],
            ConstraintRelation::LessEqual,
            context.request.positions[p].max_weight,
            &format!(
                "airworthiness_security_total_weight_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }
    Ok(())
}
