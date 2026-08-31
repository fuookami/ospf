//! 主舱门空舱限制 / Main deck door empty limits
use std::error::Error;
use ospf_rust_core::model::{LinearObjectiveInput, MetaModel};
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 主甲板舱门空载限制: 主甲板舱门附近舱位必须装载 / Main deck door empty limit: positions near main deck door must be loaded
/// 对齐 Kotlin MainDeckDoorEmptyLimit
///
/// Kotlin 使用 model.minimize(sum(stowage[i,j] for door positions)) 添加软目标。
/// 简化实现: 对第一个和最后一个舱位添加最小化目标，鼓励在这些位置装载货物。
pub fn apply_main_deck_door_empty_limits(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    _aggregation: &SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>> {
    let pos_count = context.request.positions.len();
    if pos_count == 0 {
        return Ok(());
    }

    // 主甲板舱门位置（第一个和最后一个舱位）
    let door_positions = [0, pos_count - 1];
    let mut objective_terms: Vec<(usize, f64)> = Vec::new();

    for &p in &door_positions {
        for c in 0..context.request.cargos.len() {
            objective_terms.push((context.x_idx[c][p], 1.0));
        }
    }

    if !objective_terms.is_empty() {
        let obj_input = LinearObjectiveInput::minimize(
            &format!(
                "soft_security_main_deck_door_empty_{}",
                mode_name(context.mode)
            ),
        )
        .terms(objective_terms.iter().copied());
        model.add_linear_objective_input(obj_input);
    }

    Ok(())
}
