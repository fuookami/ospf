//! 相邻位置载荷间隙限制 / Adjacent position load gap limits
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 相邻位置载荷间隙限制
/// 对齐 Kotlin AdjacentGapLimit
///
/// Kotlin: `(load.estimateLoadWeight[p] - load.estimateLoadWeight[p+1]) leq maxGap`
/// Rust: 使用已注册的 estimate_load_weight 符号索引。
pub fn apply_adjacent_gap_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    _aggregation: &AirworthinessAggregation,
    estimate_load_weight_idx: &[usize],
    _estimate_loaded_idx: &[usize],
) -> Result<(), Box<dyn Error>> {
    let pos_count = context.request.positions.len();
    for p in 0..pos_count.saturating_sub(1) {
        // estimateLoadWeight[p] - estimateLoadWeight[p+1] <= maxGap
        model.add_linear_constraint(
            &[
                (estimate_load_weight_idx[p], 1.0),
                (estimate_load_weight_idx[p + 1], -1.0),
            ],
            ConstraintRelation::LessEqual,
            context.request.max_adjacent_load_gap,
            &format!(
                "airworthiness_security_adjacent_gap_pos_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
        // estimateLoadWeight[p+1] - estimateLoadWeight[p] <= maxGap
        model.add_linear_constraint(
            &[
                (estimate_load_weight_idx[p + 1], 1.0),
                (estimate_load_weight_idx[p], -1.0),
            ],
            ConstraintRelation::LessEqual,
            context.request.max_adjacent_load_gap,
            &format!(
                "airworthiness_security_adjacent_gap_neg_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }
    Ok(())
}
