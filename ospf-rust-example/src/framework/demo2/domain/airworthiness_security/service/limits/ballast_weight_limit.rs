//! 压舱物重量限制 / Ballast weight limits
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 压舱物重量限制: ballastWeight >= minBallastWeight
/// 对齐 Kotlin BallastWeightLimit
pub fn apply_ballast_weight_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    _aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    if let Some(ballast_idx) = context.ballast_weight_idx {
        // 压舱物重量下限
        let min_ballast = 0.0; // 默认最小压舱物重量为 0
        model.add_linear_constraint(
            &[(ballast_idx, 1.0)],
            ConstraintRelation::GreaterEqual,
            min_ballast,
            &format!(
                "airworthiness_security_ballast_weight_min_{}",
                mode_name(context.mode)
            ),
        )?;
    }
    Ok(())
}
