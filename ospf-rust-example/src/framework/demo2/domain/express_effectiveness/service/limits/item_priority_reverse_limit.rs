use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::express_effectiveness::aggregation::ExpressEffectivenessAggregation;
use crate::framework::demo2::domain::express_effectiveness::context::ExpressEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 物品优先级反转限制: 低优先级货物不能在高优先级之前装载
/// 对齐 Kotlin ItemPriorityReverseLimit
///
/// 简化实现: 对于同一目的地的货物，如果高优先级未装载，低优先级也不能装载
pub fn apply_item_priority_reverse_limits(
    model: &mut MetaModel<f64>,
    context: &ExpressEffectivenessContext<'_>,
    aggregation: &ExpressEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 对于非 must_ship 的货物，如果同目的地的 must_ship 货物未装载，则也不能装载
    let non_must_ship: Vec<usize> = (0..context.request.cargos.len())
        .filter(|c| !aggregation.must_ship_indices.contains(c))
        .collect();

    for &c_low in &non_must_ship {
        for &c_high in &aggregation.must_ship_indices {
            if context.request.cargos[c_low].destination
                == context.request.cargos[c_high].destination
            {
                // 如果 c_low 装载了，c_high 也必须装载
                for p in 0..context.request.positions.len() {
                    model.add_linear_constraint(
                        &[
                            (context.x_idx[c_low][p], 1.0),
                            (context.x_idx[c_high][p], -1.0),
                        ],
                        ConstraintRelation::LessEqual,
                        0.0,
                        &format!(
                            "express_priority_reverse_{}_{}_{}_{}",
                            mode_name(context.mode),
                            c_low,
                            c_high,
                            p
                        ),
                    )?;
                }
            }
        }
    }
    Ok(())
}
