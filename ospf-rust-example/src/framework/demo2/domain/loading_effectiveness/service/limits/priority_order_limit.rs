use std::error::Error;

use ospf_rust_core::model::{ConstraintRelation, MetaModel};

use crate::framework_demo::demo2::domain::loading_effectiveness::aggregation::LoadingEffectivenessAggregation;
use crate::framework_demo::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::mode_name;

pub fn apply_priority_order_limits(
    model: &mut MetaModel<f64>,
    context: &LoadingEffectivenessContext<'_>,
    aggregation: &LoadingEffectivenessAggregation,
) -> Result<(), Box<dyn Error>> {
    for i in 0..context.request.cargos.len() {
        for j in 0..context.request.cargos.len() {
            if context.request.cargos[i].priority <= context.request.cargos[j].priority {
                continue;
            }
            let mut coefficients: Vec<(usize, f64)> = Vec::new();
            for p in 0..context.request.positions.len() {
                coefficients.push((context.x_idx[i][p], p as f64 + aggregation.big_m));
                coefficients.push((context.x_idx[j][p], -(p as f64) + aggregation.big_m));
            }
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                2.0 * aggregation.big_m,
                &format!(
                    "loading_priority_order_{}_{}_{}_{}",
                    mode_name(context.mode),
                    i,
                    j,
                    context.request.cargos[i].priority
                ),
            )?;
        }
    }
    Ok(())
}
