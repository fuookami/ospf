use std::error::Error;

use ospf_rust_core::model::{ConstraintRelation, MetaModel};

use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;

pub fn apply_adjacent_separation_limits(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    aggregation: &SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>> {
    if context.request.positions.len() <= 1 {
        return Ok(());
    }
    for i in 0..aggregation.separated.len() {
        for j in (i + 1)..aggregation.separated.len() {
            let left_cargo = aggregation.separated[i];
            let right_cargo = aggregation.separated[j];
            for p in 0..(context.request.positions.len() - 1) {
                model.add_linear_constraint(
                    &[(context.x_idx[left_cargo][p], 1.0), (context.x_idx[right_cargo][p + 1], 1.0)],
                    ConstraintRelation::LessEqual,
                    1.0,
                    &format!(
                        "soft_security_adjacent_separation_{}_{}_{}_{}_lr",
                        mode_name(context.mode),
                        p,
                        left_cargo,
                        right_cargo
                    ),
                )?;
                model.add_linear_constraint(
                    &[(context.x_idx[right_cargo][p], 1.0), (context.x_idx[left_cargo][p + 1], 1.0)],
                    ConstraintRelation::LessEqual,
                    1.0,
                    &format!(
                        "soft_security_adjacent_separation_{}_{}_{}_{}_rl",
                        mode_name(context.mode),
                        p,
                        left_cargo,
                        right_cargo
                    ),
                )?;
            }
        }
    }
    Ok(())
}
