use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 主甲板舱门空载限制: 主甲板舱门附近舱位必须装载
/// 对齐 Kotlin MainDeckDoorEmptyLimit
///
/// 简化实现: 第一个和最后一个舱位必须装载至少一个货物
pub fn apply_main_deck_door_empty_limits(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    _aggregation: &SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>> {
    // 主甲板舱门位置（第一个和最后一个舱位）必须装载
    let door_positions = vec![0, context.request.positions.len().saturating_sub(1)];
    for &p in &door_positions {
        if p < context.request.positions.len() {
            let coefficients: Vec<(usize, f64)> = (0..context.request.cargos.len())
                .map(|c| (context.x_idx[c][p], 1.0))
                .collect();
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::GreaterEqual,
                1.0,
                &format!(
                    "soft_security_main_deck_door_{}_{}",
                    mode_name(context.mode),
                    p
                ),
            )?;
        }
    }
    Ok(())
}
