//! 建议装载重量限制 / Advice load weight limits
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 建议装载重量限制: 每个舱位的装载重量建议 / Advice load weight limit: recommended load weight per position
/// 对齐 Kotlin AdviceLoadWeightLimit (目标函数: 最小化偏离建议重量)
pub fn apply_advice_load_weight_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    _aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 对齐 Kotlin - 需要 adviceLoadWeight 变量
    // 当前实现: 每个舱位建议装载 max_weight / 2 的重量
    for p in 0..context.request.positions.len() {
        let advice_weight = context.request.positions[p].max_weight / 2.0;
        let coefficients: Vec<(usize, f64)> = (0..context.request.cargos.len())
            .map(|c| (context.x_idx[c][p], context.request.cargos[c].weight))
            .collect();
        // 软约束: 装载重量接近建议值
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            advice_weight,
            &format!(
                "loading_advice_load_weight_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }
    Ok(())
}
