//! 隔离限制 / Separation limits
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use std::error::Error;

/// 分离限制 / Separation limit
///
/// 需要分离的货物不能装载在同一个舱位。
/// Cargos requiring separation cannot be loaded in the same position.
pub fn apply_separation_limits(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    aggregation: &mut SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>> {
    for p in 0..context.request.positions.len() {
        for i in 0..aggregation.separated.len() {
            for j in (i + 1)..aggregation.separated.len() {
                model.add_linear_constraint(
                    &[
                        (context.x_idx[aggregation.separated[i]][p], 1.0),
                        (context.x_idx[aggregation.separated[j]][p], 1.0),
                    ],
                    ConstraintRelation::LessEqual,
                    1.0,
                    &format!(
                        "soft_security_separation_{}_{}_{}_{}",
                        mode_name(context.mode),
                        p,
                        aggregation.separated[i],
                        aggregation.separated[j]
                    ),
                )?;
            }
        }
    }
    Ok(())
}
