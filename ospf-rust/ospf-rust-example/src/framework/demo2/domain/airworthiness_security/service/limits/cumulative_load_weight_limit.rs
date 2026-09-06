//! 累积载荷重量限制 / Cumulative load weight limits
use crate::framework::demo2::domain::airworthiness_security::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 累积载荷重量限制
/// 对齐 Kotlin CumulativeLoadWeightLimit
///
/// Kotlin: `sum(estimateLoadWeight[j] for j in checkPoint.parts) leq maxSum`
/// Rust: 使用已注册的 estimate_load_weight 符号索引构建前缀/后缀累积约束。
pub fn apply_cumulative_load_weight_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    _aggregation: &AirworthinessAggregation,
    estimate_load_weight_idx: &[usize],
    _estimate_loaded_idx: &[usize],
) -> Result<(), Box<dyn Error>> {
    let pos_count = context.request.positions.len();

    // 前缀累积: sum(estimateLoadWeight[0..=p]) <= maxCumulativeForwardLoad
    for p in 0..pos_count {
        let prefix_terms: Vec<(usize, f64)> = (0..=p)
            .map(|pos| (estimate_load_weight_idx[pos], 1.0))
            .collect();
        model.add_linear_constraint(
            &prefix_terms,
            ConstraintRelation::LessEqual,
            context.request.max_cumulative_forward_load,
            &format!(
                "airworthiness_security_cumulative_forward_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }

    // 后缀累积: sum(estimateLoadWeight[p..end]) <= maxCumulativeBackwardLoad
    for p in 0..pos_count {
        let suffix_terms: Vec<(usize, f64)> = (p..pos_count)
            .map(|pos| (estimate_load_weight_idx[pos], 1.0))
            .collect();
        model.add_linear_constraint(
            &suffix_terms,
            ConstraintRelation::LessEqual,
            context.request.max_cumulative_backward_load,
            &format!(
                "airworthiness_security_cumulative_backward_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }
    Ok(())
}
