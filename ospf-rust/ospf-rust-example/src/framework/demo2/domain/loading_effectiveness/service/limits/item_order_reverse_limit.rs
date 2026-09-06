//! 物品排序反向限制 / Item order reverse limits
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 物品顺序反转限制: 高优先级货物不能在低优先级之后装载 / Item order reverse limit: higher-priority cargos cannot load after lower-priority ones
/// 对齐 Kotlin ItemOrderReverseLimit
///
/// 简化实现: 对于同一目的地的货物，高优先级必须在低优先级之前装载
pub fn apply_item_order_reverse_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    _aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 按目的地分组，然后按优先级排序
    let mut cargos_by_destination: std::collections::BTreeMap<String, Vec<(usize, u8)>> =
        std::collections::BTreeMap::new();
    for c in 0..context.request.cargos.len() {
        cargos_by_destination
            .entry(context.request.cargos[c].destination.clone())
            .or_default()
            .push((c, context.request.cargos[c].priority));
    }

    // 对于同一目的地的货物对，如果优先级不同，高优先级必须先装载
    for (_dest, cargos) in &cargos_by_destination {
        for i in 0..cargos.len() {
            for j in (i + 1)..cargos.len() {
                let (c1, p1) = cargos[i];
                let (c2, p2) = cargos[j];
                if p1 > p2 {
                    // c1 优先级高于 c2，c1 必须在 c2 之前装载
                    // 简化实现: 如果 c2 装载了，c1 也必须装载
                    for pos in 0..context.request.positions.len() {
                        model.add_linear_constraint(
                            &[
                                (context.x_idx[c2][pos], 1.0),
                                (context.x_idx[c1][pos], -1.0),
                            ],
                            ConstraintRelation::LessEqual,
                            0.0,
                            &format!(
                                "loading_order_reverse_{}_{}_{}_{}",
                                mode_name(context.mode),
                                c1,
                                c2,
                                pos
                            ),
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}
