use std::error::Error;
use ospf_rust_core::model::{MetaModel, LinearObjectiveInput};
use crate::framework::demo2::domain::loading_effectiveness::model::TrailerChangeVariables;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 拖车更换限制: 最小化拖车更换次数
/// 对齐 Kotlin TrailerChangeLimit
///
/// Kotlin 语义:
///   model.minimize(sum(orderedTrailers.flatMapIndexed { p1, (trailer1, trailer2) ->
///       adjacentPositions.mapIndexed { p2, (position1, position2) ->
///           coefficient(position2 to trailer1, position1 to trailer2) * loading.trailerChange[p1, p2]
///       }
///   }))
///
/// 其中 trailerChange[p1, p2] 是 IfFunction 符号:
///   condition = loadAmountOf(position1){trailer2.items} + loadAmountOf(position2){trailer1.items} - 2
///   当两个拖车的物品在相邻位置发生交叉装载时值为 1。
///
/// Rust 实现: 使用已注册的 trailerChange IfFunction 符号，最小化其总和。
pub fn apply_trailer_change_limits(
    model: &mut MetaModel<f64>,
    trailer_change_vars: &TrailerChangeVariables,
    mode_name_str: &str,
) -> Result<(), Box<dyn Error>> {
    let mut objective_terms: Vec<(usize, f64)> = Vec::new();

    for change_row in &trailer_change_vars.trailer_change {
        for &var_idx in change_row {
            objective_terms.push((var_idx, 1.0));
        }
    }

    if !objective_terms.is_empty() {
        let obj_input = LinearObjectiveInput::minimize(
            &format!("loading_trailer_change_{}", mode_name_str),
        )
        .terms(objective_terms.iter().copied());
        model.add_linear_objective_input(obj_input);
    }

    Ok(())
}
