use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::stowage::StowageVariables;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 生物散货冲突限制: 活体动物不能与散货在同一舱位
/// 对齐 Kotlin BiologicalBulkConflictLimit
///
/// 使用 requires_separation 作为生物货物的代理标识
/// 生物货物不能与普通货物在同一舱位
pub fn apply_biological_bulk_conflict_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    stowage_vars: &StowageVariables,
) -> Result<(), Box<dyn Error>> {
    // 生物货物 (requires_separation) 不能与普通货物在同一舱位
    let bio_cargos: Vec<usize> = (0..context.request.cargos.len())
        .filter(|c| context.request.cargos[*c].requires_separation)
        .collect();
    let normal_cargos: Vec<usize> = (0..context.request.cargos.len())
        .filter(|c| !context.request.cargos[*c].requires_separation)
        .collect();

    for &c1 in &bio_cargos {
        for &c2 in &normal_cargos {
            for p in 0..context.request.positions.len() {
                model.add_linear_constraint(
                    &[
                        (stowage_vars.stowage[c1][p], 1.0),
                        (stowage_vars.stowage[c2][p], 1.0),
                    ],
                    ConstraintRelation::LessEqual,
                    1.0,
                    &format!(
                        "stowage_bio_bulk_conflict_{}_{}_{}_{}",
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
