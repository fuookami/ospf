use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 线密度限制 / Linear density limit
/// 对齐 Kotlin LinearDensityLimit: loadWeight[j] <= position.length * maxLinearDensity
pub fn apply_linear_density_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 每个舱位的载荷重量 / 长度 <= 最大线密度
    for p in 0..context.request.positions.len() {
        let length = context.request.positions[p].length;
        if length > 0.0 && !aggregation.per_position_weight_coefficients[p].is_empty() {
            // 线密度约束: 总重量 <= length * maxLinearDensity
            // maxLinearDensity 默认为 500 kg/m
            let max_linear_density = 500.0;
            model.add_linear_constraint(
                &aggregation.per_position_weight_coefficients[p],
                ConstraintRelation::LessEqual,
                length * max_linear_density,
                &format!(
                    "airworthiness_security_linear_density_{}_{}",
                    mode_name(context.mode),
                    p
                ),
            )?;
        }
    }
    Ok(())
}
