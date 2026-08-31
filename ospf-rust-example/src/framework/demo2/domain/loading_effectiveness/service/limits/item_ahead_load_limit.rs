use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 物品提前装载限制: 优先级高的货物应优先装载
/// 对齐 Kotlin ItemAheadLoadLimit
///
/// 简化实现: 高优先级货物 (priority >= 8) 必须装载
pub fn apply_item_ahead_load_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    _aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 高优先级货物必须装载
    for c in 0..context.request.cargos.len() {
        if context.request.cargos[c].priority >= 8 {
            let coefficients: Vec<(usize, f64)> = (0..context.request.positions.len())
                .map(|p| (context.x_idx[c][p], 1.0))
                .collect();
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::Equal,
                1.0,
                &format!(
                    "loading_item_ahead_{}_{}",
                    mode_name(context.mode),
                    c
                ),
            )?;
        }
    }
    Ok(())
}
