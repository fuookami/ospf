//! 拖车绕行限制 / Trailer circling limits
use std::error::Error;
use std::collections::BTreeSet;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 拖车循环限制: 最小化拖车循环 / Trailer circling limit: minimize trailer circling
/// 对齐 Kotlin TrailerCirclingLimit
///
/// Kotlin 语义: model.minimize(sum of trailerCircling intermediate symbols for adjacent positions)
/// 简化实现: 同一目的地的货物应装载在相邻舱位，减少拖车循环
///
/// 对于同一目的地的每对货物 (c1, c2)，如果都装载了，它们应在同一或相邻舱位:
///   x[c1][p1] + x[c2][p2] <= 1  (对所有 p1, p2 不相邻且 p1 != p2)
pub fn apply_trailer_circling_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    _aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    // 按目的地分组
    let mut cargos_by_destination: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
    for c in 0..context.request.cargos.len() {
        cargos_by_destination
            .entry(context.request.cargos[c].destination.clone())
            .or_default()
            .push(c);
    }

    // 构建相邻位置对集合 (用于快速查找)
    // Build adjacent position pair set for fast lookup
    let adjacent_set: BTreeSet<(usize, usize)> = context
        .request
        .adjacent_positions
        .iter()
        .flat_map(|pair| vec![(pair.first, pair.second), (pair.second, pair.first)])
        .collect();

    let pos_count = context.request.positions.len();

    // 对于同一目的地的货物对，约束它们必须在同一或相邻舱位
    // For same-destination cargo pairs, constrain them to same or adjacent positions
    for (_dest, cargos) in &cargos_by_destination {
        if cargos.len() <= 1 {
            continue;
        }
        for i in 0..cargos.len() {
            for j in (i + 1)..cargos.len() {
                let c1 = cargos[i];
                let c2 = cargos[j];
                for p1 in 0..pos_count {
                    for p2 in 0..pos_count {
                        if p1 == p2 {
                            continue;
                        }
                        // 跳过相邻位置对 (允许在相邻位置)
                        // Skip adjacent position pairs (allowed to be adjacent)
                        if adjacent_set.contains(&(p1, p2)) {
                            continue;
                        }
                        // 非相邻位置: 不允许同时装载
                        // Non-adjacent positions: not allowed to load both
                        model.add_linear_constraint(
                            &[
                                (context.x_idx[c1][p1], 1.0),
                                (context.x_idx[c2][p2], 1.0),
                            ],
                            ConstraintRelation::LessEqual,
                            1.0,
                            &format!(
                                "loading_trailer_circling_{}_{}_{}_{}_{}",
                                mode_name(context.mode),
                                c1, c2, p1, p2
                            ),
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}
