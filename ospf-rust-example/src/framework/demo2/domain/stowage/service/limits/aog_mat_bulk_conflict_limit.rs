//! AOG/MAT 散货冲突限制 / AOG/MAT bulk conflict limits
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::stowage::StowageVariables;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// AOG/MAT 散货冲突限制: 航材与普通散货不能混装在同一舱位
/// 对齐 Kotlin AOGMATBulkConflictLimit
///
/// 使用 requires_separation 作为航材的代理标识
pub fn apply_aog_mat_bulk_conflict_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    stowage_vars: &StowageVariables,
) -> Result<(), Box<dyn Error>> {
    // requires_separation 的货物不能与非 requires_separation 的货物在同一舱位
    let separation_cargos: Vec<usize> = (0..context.request.cargos.len())
        .filter(|c| context.request.cargos[*c].requires_separation)
        .collect();
    let normal_cargos: Vec<usize> = (0..context.request.cargos.len())
        .filter(|c| !context.request.cargos[*c].requires_separation)
        .collect();

    for &c1 in &separation_cargos {
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
                        "stowage_aog_mat_conflict_{}_{}_{}_{}",
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
