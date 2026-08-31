use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 非对称线密度限制 / Unsymmetrical linear density limit
/// 对齐 Kotlin UnsymmetricalLinearDensityLimit: 横向力矩上下限
pub fn apply_unsymmetrical_linear_density_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    if context.request.max_lateral_imbalance > 0.0 {
        // 横向力矩上限
        model.add_linear_constraint(
            &aggregation.lateral_moment_coefficients,
            ConstraintRelation::LessEqual,
            context.request.max_lateral_imbalance,
            &format!(
                "airworthiness_security_lateral_imbalance_upper_{}",
                mode_name(context.mode)
            ),
        )?;
        // 横向力矩下限
        model.add_linear_constraint(
            &aggregation.lateral_moment_coefficients,
            ConstraintRelation::GreaterEqual,
            -context.request.max_lateral_imbalance,
            &format!(
                "airworthiness_security_lateral_imbalance_lower_{}",
                mode_name(context.mode)
            ),
        )?;
    }
    Ok(())
}
