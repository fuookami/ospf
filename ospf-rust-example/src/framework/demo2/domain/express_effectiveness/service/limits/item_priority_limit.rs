use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::express_effectiveness::aggregation::ExpressEffectivenessAggregation;
use crate::framework::demo2::domain::express_effectiveness::context::ExpressEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 物品优先级限制: 高优先级货物优先装载
/// 对齐 Kotlin ItemPriorityLimit
pub fn apply_item_priority_limits(
    model: &mut MetaModel<f64>,
    context: &ExpressEffectivenessContext<'_>,
    aggregation: &ExpressEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 高优先级货物 (priority >= 8) 必须装载
    for &c in &aggregation.must_ship_indices {
        let coefficients: Vec<(usize, f64)> = (0..context.request.positions.len())
            .map(|p| (context.x_idx[c][p], 1.0))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::Equal,
            1.0,
            &format!(
                "express_item_priority_{}_{}",
                mode_name(context.mode),
                c
            ),
        )?;
    }
    Ok(())
}
