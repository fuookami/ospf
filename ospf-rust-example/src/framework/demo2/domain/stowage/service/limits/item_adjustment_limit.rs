//! 物品调整限制 / Item adjustment limits
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::stowage::StowageVariables;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 物品调整限制: 已装载的物品，调整变量必须为 0
/// 对齐 Kotlin ItemAdjustmentLimit
pub fn apply_item_adjustment_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    stowage_vars: &StowageVariables,
) -> Result<(), Box<dyn Error>> {
    for c in 0..context.request.cargos.len() {
        for p in 0..context.request.positions.len() {
            // 如果物品已装载到此舱位，调整变量必须为 0
            if context.request.positions[p]
                .loaded_items
                .contains(&context.request.cargos[c].name)
            {
                model.add_linear_constraint(
                    &[(stowage_vars.u[c][p], 1.0)],
                    ConstraintRelation::Equal,
                    0.0,
                    &format!(
                        "stowage_item_adjustment_{}_{}_{}",
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
