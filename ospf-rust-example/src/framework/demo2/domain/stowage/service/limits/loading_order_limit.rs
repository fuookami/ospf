//! 装载顺序限制 / Loading order limits
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::stowage::StowageVariables;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 装载顺序限制: 装载顺序靠前的舱位应优先装载
/// 对齐 Kotlin LoadingOrderLimit
///
/// Kotlin 语义: 对于相邻位置对 (j1, j2)，如果 j1 在 j2 之前装载，
///   则 actualLoaded[j1] >= actualLoaded[j2]（硬约束）。
///   跳过 unavailable 或 bulk 位置。
/// Rust 实现: 使用相邻位置对，对每个位置对中 j1 的装载量 >= j2 的装载量。
///   装载量 = sum(x[c][j] for all cargos c)。
///   注意: Rust 当前未检查 unavailable/bulk 位置（StowageContext 无此信息）。
pub fn apply_loading_order_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    _stowage_vars: &StowageVariables,
) -> Result<(), Box<dyn Error>> {
    let pos_count = context.request.positions.len();
    let cargo_count = context.request.cargos.len();

    // 对于相邻位置对，添加装载顺序约束: sum(x[c][j1]) >= sum(x[c][j2])
    for adj in &context.request.adjacent_positions {
        let j1 = adj.first;
        let j2 = adj.second;
        if j1 >= pos_count || j2 >= pos_count {
            continue;
        }

        // sum(x[c][j1] for all c) - sum(x[c][j2] for all c) >= 0
        let mut coefficients: Vec<(usize, f64)> = Vec::new();
        for c in 0..cargo_count {
            if c < context.x_idx.len() && j1 < context.x_idx[c].len() {
                coefficients.push((context.x_idx[c][j1], 1.0));
            }
            if c < context.x_idx.len() && j2 < context.x_idx[c].len() {
                coefficients.push((context.x_idx[c][j2], -1.0));
            }
        }

        if !coefficients.is_empty() {
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::GreaterEqual,
                0.0,
                &format!("{}_loading_order_{}_{}", mode_name(context.mode), j1, j2),
            )?;
        }
    }

    Ok(())
}
