use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::mac_optimization::aggregation::MacOptimizationAggregation;
use crate::framework::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 水平安定面限制: MAC 值必须在安全范围内
/// 对齐 Kotlin mac_optimization HorizontalStabilizerLimit
///
/// 使用纵向力矩约束作为代理，限制 MAC 在安全范围内
pub fn apply_horizontal_stabilizer_limits(
    model: &mut MetaModel<f64>,
    context: &MacOptimizationContext<'_>,
    aggregation: &MacOptimizationAggregation,
) -> Result<(), Box<dyn Error>> {
    // 水平安定面约束: 纵向力矩必须在 envelope 范围内
    // 使用 target_longitudinal_moment ± max_longitudinal_moment_deviation 作为限制
    let target = context.request.target_longitudinal_moment;
    let max_dev = context.request.max_longitudinal_moment_deviation;

    // 纵向力矩上限
    model.add_linear_constraint(
        &aggregation.long_moment,
        ConstraintRelation::LessEqual,
        target + max_dev,
        &format!(
            "mac_horizontal_stabilizer_upper_{}",
            mode_name(context.mode)
        ),
    )?;
    // 纵向力矩下限
    model.add_linear_constraint(
        &aggregation.neg_long_moment,
        ConstraintRelation::LessEqual,
        -target + max_dev,
        &format!(
            "mac_horizontal_stabilizer_lower_{}",
            mode_name(context.mode)
        ),
    )?;
    Ok(())
}
