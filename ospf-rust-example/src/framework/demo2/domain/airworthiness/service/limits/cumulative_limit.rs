use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework_demo::demo2::domain::airworthiness::aggregation::AirworthinessAggregation;
use crate::framework_demo::demo2::domain::airworthiness::context::AirworthinessContext;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::mode_name;

pub fn apply_cumulative_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    _aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    for p in 0..context.request.positions.len() {
        let mut prefix_coefficients: Vec<(usize, f64)> = Vec::new();
        for pos in 0..=p {
            for c in 0..context.request.cargos.len() {
                prefix_coefficients.push((context.x_idx[c][pos], context.request.cargos[c].weight));
            }
        }
        model.add_linear_constraint(
            &prefix_coefficients,
            ConstraintRelation::LessEqual,
            context.request.max_cumulative_forward_load,
            &format!(
                "airworthiness_cumulative_forward_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }

    for p in 0..context.request.positions.len() {
        let mut suffix_coefficients: Vec<(usize, f64)> = Vec::new();
        for pos in p..context.request.positions.len() {
            for c in 0..context.request.cargos.len() {
                suffix_coefficients.push((context.x_idx[c][pos], context.request.cargos[c].weight));
            }
        }
        model.add_linear_constraint(
            &suffix_coefficients,
            ConstraintRelation::LessEqual,
            context.request.max_cumulative_backward_load,
            &format!(
                "airworthiness_cumulative_backward_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }
    Ok(())
}
