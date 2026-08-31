//! 装载限制 / Stowage limits
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::stowage::StowageVariables;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 装载限制: 不兼容的 item-position 对强制 stowage[i][j] = 0
/// 对齐 Kotlin StowageLimit
pub fn apply_stowage_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    stowage_vars: &StowageVariables,
) -> Result<(), Box<dyn Error>> {
    for c in 0..context.request.cargos.len() {
        for p in 0..context.request.positions.len() {
            let cargo_weight = context.request.cargos[c].weight;
            let position_max = context.request.positions[p].max_weight;
            // 如果单个货物超过舱位最大重量，禁止装载
            if cargo_weight > position_max + 1e-9 {
                model.add_linear_constraint(
                    &[(stowage_vars.x[c][p], 1.0)],
                    ConstraintRelation::Equal,
                    0.0,
                    &format!(
                        "stowage_forbidden_{}_{}_{}",
                        mode_name(context.mode),
                        c,
                        p
                    ),
                )?;
            }
        }
    }
    Ok(())
}
