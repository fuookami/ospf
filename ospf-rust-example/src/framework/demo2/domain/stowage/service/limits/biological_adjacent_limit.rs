//! 生物邻接限制 / Biological adjacent limits
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::stowage::StowageVariables;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 生物邻接限制: 活体动物不能与特定货物在同一舱位
/// 对齐 Kotlin BiologicalAdjacentLimit
pub fn apply_biological_adjacent_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    stowage_vars: &StowageVariables,
) -> Result<(), Box<dyn Error>> {
    // 使用 requires_separation 作为生物货物的代理标识
    let bio_cargos: Vec<usize> = (0..context.request.cargos.len())
        .filter(|c| context.request.cargos[*c].requires_separation)
        .collect();

    // 生物货物不能与其他 requires_separation 货物在同一舱位
    for &c1 in &bio_cargos {
        for c2 in 0..context.request.cargos.len() {
            if c1 == c2 || !context.request.cargos[c2].requires_separation {
                continue;
            }
            for p in 0..context.request.positions.len() {
                model.add_linear_constraint(
                    &[
                        (stowage_vars.stowage[c1][p], 1.0),
                        (stowage_vars.stowage[c2][p], 1.0),
                    ],
                    ConstraintRelation::LessEqual,
                    1.0,
                    &format!(
                        "stowage_bio_adjacent_{}_{}_{}_{}",
                        mode_name(context.mode),
                        c1,
                        c2,
                        p
                    ),
                )?;
            }
        }
    }
    Ok(())
}
