//! 表面密度限制 / Surface density limits
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 表面密度限制 / Surface density limit
/// 对齐 Kotlin SurfaceDensityLimit: loadWeight[j] <= position.area * maxSurfaceDensity
pub fn apply_surface_density_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 每个舱位的载荷重量 / 面积 <= 最大表面密度
    // 使用 per_position_weight_coefficients 和 position.area
    for p in 0..context.request.positions.len() {
        let area = context.request.positions[p].area;
        if area > 0.0 && !aggregation.per_position_weight_coefficients[p].is_empty() {
            // 表面密度约束: 总重量 <= area * maxSurfaceDensity
            // maxSurfaceDensity 默认为 1000 kg/m²
            let max_surface_density = 1000.0;
            model.add_linear_constraint(
                &aggregation.per_position_weight_coefficients[p],
                ConstraintRelation::LessEqual,
                area * max_surface_density,
                &format!(
                    "airworthiness_security_surface_density_{}_{}",
                    mode_name(context.mode),
                    p
                ),
            )?;
        }
    }
    Ok(())
}
