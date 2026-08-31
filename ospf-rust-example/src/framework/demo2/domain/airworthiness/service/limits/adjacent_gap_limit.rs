use std::error::Error;

use ospf_rust_core::model::{ConstraintRelation, MetaModel};

use crate::framework_demo::demo2::domain::airworthiness::aggregation::AirworthinessAggregation;
use crate::framework_demo::demo2::domain::airworthiness::context::AirworthinessContext;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::mode_name;

pub fn apply_adjacent_gap_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    _aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    for p in 0..context.request.positions.len().saturating_sub(1) {
        let mut current_minus_next: Vec<(usize, f64)> = Vec::new();
        let mut next_minus_current: Vec<(usize, f64)> = Vec::new();
        for c in 0..context.request.cargos.len() {
            let weight = context.request.cargos[c].weight;
            current_minus_next.push((context.x_idx[c][p], weight));
            current_minus_next.push((context.x_idx[c][p + 1], -weight));
            next_minus_current.push((context.x_idx[c][p], -weight));
            next_minus_current.push((context.x_idx[c][p + 1], weight));
        }
        model.add_linear_constraint(
            &current_minus_next,
            ConstraintRelation::LessEqual,
            context.request.max_adjacent_load_gap,
            &format!(
                "airworthiness_adjacent_gap_pos_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
        model.add_linear_constraint(
            &next_minus_current,
            ConstraintRelation::LessEqual,
            context.request.max_adjacent_load_gap,
            &format!(
                "airworthiness_adjacent_gap_neg_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;
    }
    Ok(())
}
