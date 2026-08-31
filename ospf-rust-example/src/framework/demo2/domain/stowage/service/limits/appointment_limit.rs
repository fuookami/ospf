//! 指定舱位限制 / Appointment limits
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::domain::stowage::model::stowage::StowageVariables;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 预约限制: 预约的 cargo-position 对强制 x[c][p] = 1
/// 对齐 Kotlin AppointmentLimit
///
/// 使用 loaded_items 作为预约代理
pub fn apply_appointment_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    _aggregation: &StowageAggregation,
    stowage_vars: &StowageVariables,
) -> Result<(), Box<dyn Error>> {
    // 对于已装载的物品 (loaded_items)，强制 x[c][p] = 1
    for p in 0..context.request.positions.len() {
        for loaded_name in &context.request.positions[p].loaded_items {
            if let Some(c) = context
                .request
                .cargos
                .iter()
                .position(|cargo| &cargo.name == loaded_name)
            {
                model.add_linear_constraint(
                    &[(stowage_vars.x[c][p], 1.0)],
                    ConstraintRelation::Equal,
                    1.0,
                    &format!(
                        "stowage_appointment_{}_{}_{}",
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
