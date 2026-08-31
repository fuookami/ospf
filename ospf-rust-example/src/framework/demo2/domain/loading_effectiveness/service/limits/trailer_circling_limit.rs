use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 拖车循环限制: 最小化拖车循环
/// 对齐 Kotlin TrailerCirclingLimit
///
/// 简化实现: 同一目的地的货物应装载在相邻舱位，减少拖车循环
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

    // 对于同一目的地的货物，限制每个舱位最多装载 2 个
    for (_dest, cargos) in &cargos_by_destination {
        if cargos.len() <= 1 {
            continue;
        }
        for p in 0..context.request.positions.len() {
            let coefficients: Vec<(usize, f64)> = cargos
                .iter()
                .map(|&c| (context.x_idx[c][p], 1.0))
                .collect();
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                2.0,
                &format!(
                    "loading_trailer_circling_{}_{}_{}",
                    mode_name(context.mode),
                    _dest,
                    p
                ),
            )?;
        }
    }
    Ok(())
}
