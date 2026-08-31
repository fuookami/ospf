use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::stowage::StowageVariables;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// ELD 邻接限制: 电子设备不能与特定货物相邻
/// 对齐 Kotlin ELDAdjacentLimit
///
/// 使用 requires_separation 作为电子设备的代理标识
/// 电子设备货物不能与其他 requires_separation 货物在同一舱位
pub fn apply_eld_adjacent_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    stowage_vars: &StowageVariables,
) -> Result<(), Box<dyn Error>> {
    // 使用 requires_separation 作为电子设备标识
    let eld_cargos: Vec<usize> = (0..context.request.cargos.len())
        .filter(|c| context.request.cargos[*c].requires_separation)
        .collect();

    // 电子设备货物之间不能在同一舱位
    for i in 0..eld_cargos.len() {
        for j in (i + 1)..eld_cargos.len() {
            let c1 = eld_cargos[i];
            let c2 = eld_cargos[j];
            for p in 0..context.request.positions.len() {
                model.add_linear_constraint(
                    &[
                        (stowage_vars.stowage[c1][p], 1.0),
                        (stowage_vars.stowage[c2][p], 1.0),
                    ],
                    ConstraintRelation::LessEqual,
                    1.0,
                    &format!(
                        "stowage_eld_adjacent_{}_{}_{}_{}",
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
