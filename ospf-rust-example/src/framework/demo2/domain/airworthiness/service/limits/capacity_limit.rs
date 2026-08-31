use std::error::Error;

use ospf_rust_core::model::{ConstraintRelation, MetaModel};

use crate::framework::demo2::domain::airworthiness::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

pub fn apply_capacity_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    _aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    for p in 0..context.request.positions.len() {
        let coefficients: Vec<(usize, f64)> = (0..context.request.cargos.len())
            .map(|c| (context.x_idx[c][p], context.request.cargos[c].weight))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            context.request.positions[p].max_weight,
            &format!("airworthiness_capacity_{}_{}", mode_name(context.mode), p),
        )?;
    }
    Ok(())
}

