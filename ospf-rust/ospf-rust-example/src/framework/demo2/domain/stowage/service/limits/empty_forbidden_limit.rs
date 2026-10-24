//! 空禁限制 / Empty forbidden limits
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 显式空禁约束整体占位，三种模式一致，允许合法空预留。
/// Explicit empty-forbidden constrains whole occupancy in all modes, including legal empty reservations.
pub fn apply_empty_forbidden_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    _loaded_idx: &[usize],
    estimate_loaded_idx: &[usize],
) -> Result<(), Box<dyn Error>> {
    for p in 0..context.request.positions.len() {
        if !context.request.loading.positions[&context.request.positions[p].name].empty_forbidden { continue; }
        model.add_linear_constraint(
            &[(estimate_loaded_idx[p], 1.0)],
            ConstraintRelation::GreaterEqual,
            1.0,
            &format!("stowage_empty_forbidden_{}_{}", mode_name(context.mode), p),
        )?;
    }
    Ok(())
}
