//! 分配限制 / Assignment limits
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use ospf_rust_core::model::MetaModel;
use std::error::Error;

/// 物品分配限制
/// 对齐 Kotlin ItemAssignmentLimit
///
/// Kotlin: `stowage.loaded[i] eq true` (预分配) 或 `stowage.loaded[i] leq true` (可选)
/// Rust: 使用已注册的 loaded[c] 符号索引。
pub fn apply_assignment_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    aggregation: &StowageAggregation,
    loaded_idx: &[usize],
    _estimate_loaded_idx: &[usize],
) -> Result<(), Box<dyn Error>> {
    for c in 0..context.request.cargos.len() {
        // loaded[c] <relation> rhs
        // 对齐 Kotlin: loaded[i] eq/leq true
        model.add_linear_constraint(
            &[(loaded_idx[c], 1.0)],
            aggregation.assignment_relation,
            aggregation.assignment_rhs,
            &format!("stowage_assignment_{}_{}", mode_name(context.mode), c),
        )?;
    }
    Ok(())
}
