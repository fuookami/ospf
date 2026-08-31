use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 区域载荷重量限制 / Zone load weight limit
/// 对齐 Kotlin ZoneLoadWeightLimit: 每个舱位载荷 <= 该舱位最大载荷
pub fn apply_zone_load_weight_limits(
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
                "airworthiness_security_zone_load_weight_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }
    Ok(())
}
