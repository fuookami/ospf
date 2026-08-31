use std::error::Error;
use ospf_rust_core::model::{MetaModel, LinearObjectiveInput};
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 物品重新称重限制: 需要重新称重的物品应尽量不装载
/// 对齐 Kotlin ItemReweighNeededLimit
///
/// Kotlin 语义: model.minimize(sum(coefficient * loaded[i]) for items needing reweigh)
/// 简化实现: 高重量货物 (weight >= 8) 需要重新称重，通过最小化目标鼓励不装载
pub fn apply_item_reweigh_needed_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    _aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 高重量货物需要重新称重，最小化其装载量
    // Heavy items (weight >= 8) need reweighing, minimize their loading
    let objective_terms: Vec<(usize, f64)> = (0..context.request.cargos.len())
        .filter(|&c| context.request.cargos[c].weight >= 8.0)
        .flat_map(|c| {
            (0..context.request.positions.len())
                .map(move |p| (context.x_idx[c][p], 1.0))
        })
        .collect();

    if !objective_terms.is_empty() {
        let obj_input = LinearObjectiveInput::minimize(
            &format!("item_reweigh_needed_{}", mode_name(context.mode)),
        )
        .terms(objective_terms.iter().copied());
        model.add_linear_objective_input(obj_input);
    }

    Ok(())
}
