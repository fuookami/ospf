use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 物品重新称重限制: 需要重新称重的物品必须装载到指定舱位
/// 对齐 Kotlin ItemReweighNeededLimit
///
/// 简化实现: 高重量货物 (weight >= 8) 需要重新称重，限制其装载数量
pub fn apply_item_reweigh_needed_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    _aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 高重量货物需要重新称重，限制每个舱位最多装载一个
    let heavy_cargos: Vec<usize> = (0..context.request.cargos.len())
        .filter(|c| context.request.cargos[*c].weight >= 8.0)
        .collect();

    for p in 0..context.request.positions.len() {
        let coefficients: Vec<(usize, f64)> = heavy_cargos
            .iter()
            .map(|&c| (context.x_idx[c][p], 1.0))
            .collect();
        if !coefficients.is_empty() {
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                1.0,
                &format!(
                    "loading_reweigh_needed_{}_{}",
                    mode_name(context.mode),
                    p
                ),
            )?;
        }
    }
    Ok(())
}
