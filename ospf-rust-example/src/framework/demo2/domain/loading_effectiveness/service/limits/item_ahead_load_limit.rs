use std::error::Error;
use ospf_rust_core::model::{LinearObjectiveInput, MetaModel};
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 物品提前装载限制: 优先级高的货物应优先装载
/// 对齐 Kotlin ItemAheadLoadLimit
///
/// Kotlin 使用 model.minimize(sum(coefficient(item) * stowage.loaded[i])) 最小化装载状态。
/// Kotlin 的 predicates 根据处理阶段（到店/称重/复查）过滤物品。
///
/// 简化实现: 对高优先级货物 (priority >= 8) 使用 add_linear_objective_input
/// 添加最小化目标，鼓励这些货物被装载。
pub fn apply_item_ahead_load_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    _aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 简化的 predicate: 高优先级货物 (priority >= 8)
    let high_priority_cargos: Vec<usize> = (0..context.request.cargos.len())
        .filter(|c| context.request.cargos[*c].priority >= 8)
        .collect();

    if high_priority_cargos.is_empty() {
        return Ok(());
    }

    // 构建目标项: sum(high_priority_cargos[c].weight * sum_p x[c][p])
    // 对齐 Kotlin: sum(coefficient(item) * stowage.loaded[i])
    // 使用货物权重作为系数，跨所有位置求和
    let mut objective_terms: Vec<(usize, f64)> = Vec::new();
    for &c in &high_priority_cargos {
        let coefficient = context.request.cargos[c].weight;
        for p in 0..context.request.positions.len() {
            objective_terms.push((context.x_idx[c][p], coefficient));
        }
    }

    if !objective_terms.is_empty() {
        let obj_input = LinearObjectiveInput::minimize(&format!(
            "loading_effectiveness_item_ahead_load_{}",
            mode_name(context.mode)
        ))
        .terms(objective_terms.iter().copied());
        model.add_linear_objective_input(obj_input);
    }

    Ok(())
}
