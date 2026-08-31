//! 横向平衡限制 / Lateral balance limits
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::mac_optimization::aggregation::MacOptimizationAggregation;
use crate::framework::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 横向平衡限制 / Lateral balance limit
/// 对齐 Kotlin LateralBalanceLimit
pub fn apply_lateral_balance_limits(
    model: &mut MetaModel<f64>,
    context: &MacOptimizationContext<'_>,
    aggregation: &MacOptimizationAggregation,
) -> Result<(), Box<dyn Error>> {
    // 横向力矩约束（即使 max_lateral_imbalance = 0 也要添加，确保横向力矩为 0）
    model.add_linear_constraint(
        &aggregation.lat_moment,
        ConstraintRelation::LessEqual,
        context.request.max_lateral_imbalance,
        &format!("mac_lateral_upper_{}", mode_name(context.mode)),
    )?;
    model.add_linear_constraint(
        &aggregation.neg_lat_moment,
        ConstraintRelation::LessEqual,
        context.request.max_lateral_imbalance,
        &format!("mac_lateral_lower_{}", mode_name(context.mode)),
    )?;
    Ok(())
}
