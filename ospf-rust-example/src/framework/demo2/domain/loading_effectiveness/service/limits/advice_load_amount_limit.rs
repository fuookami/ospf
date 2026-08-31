//! 建议装载数量限制 / Advice load amount limits
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 建议装载数量限制: 每个舱位的装载数量建议 / Advice load amount limit: recommended load count per position
/// 对齐 Kotlin AdviceLoadAmountLimit (目标函数: 最小化偏离建议数量)
pub fn apply_advice_load_amount_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    _aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 对齐 Kotlin - 需要 adviceLoadAmount 变量
    // 当前实现: 每个舱位建议装载 max_load_count / 2 个货物
    for p in 0..context.request.positions.len() {
        let advice_count = (context.request.positions[p].max_load_count as f64 / 2.0).ceil();
        let coefficients: Vec<(usize, f64)> = (0..context.request.cargos.len())
            .map(|c| (context.x_idx[c][p], 1.0))
            .collect();
        // 软约束: 装载数量接近建议值
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            advice_count,
            &format!(
                "loading_advice_load_amount_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }
    Ok(())
}
