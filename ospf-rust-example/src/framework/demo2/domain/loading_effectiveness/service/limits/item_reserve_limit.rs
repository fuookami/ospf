//! 物品预留限制 / Item reserve limits
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{LinearObjectiveInput, MetaModel};
use std::error::Error;

/// 物品保留限制: 最大化装载量，低优先级物品不强制装载 / Item reserve limit: maximize loading, low-priority items not forced
/// 对齐 Kotlin ItemReserveLimit
///
/// Kotlin 语义: model.minimize(sum(coefficient - coefficient * loaded[i]))
///   = minimize(constant - sum(coefficient * loaded[i]))
///   = maximize(sum(coefficient * loaded[i]))
/// 简化实现: 通过最小化目标(-loaded变量)鼓励装载所有物品，低优先级物品不强制
pub fn apply_item_reserve_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    _aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 最大化装载量: minimize(sum(-1.0 * loaded[i][p])) = maximize(sum(loaded[i][p]))
    // Maximize loading: minimize negative loaded variables
    let objective_terms: Vec<(usize, f64)> = (0..context.request.cargos.len())
        .flat_map(|i| {
            (0..context.request.positions.len()).map(move |p| (context.x_idx[i][p], -1.0))
        })
        .collect();

    if !objective_terms.is_empty() {
        let obj_input =
            LinearObjectiveInput::minimize(&format!("item_reserve_{}", mode_name(context.mode)))
                .terms(objective_terms.iter().copied());
        model.add_linear_objective_input(obj_input);
    }

    Ok(())
}
