use std::error::Error;

use ospf_rust_core::model::{ConstraintRelation, MetaModel};

use crate::framework::demo2::domain::airworthiness::aggregation::AirworthinessAggregation;
use crate::framework::demo2::domain::airworthiness::context::AirworthinessContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

pub fn apply_payload_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    let total_cargo_weight: f64 = context.request.cargos.iter().map(|cargo| cargo.weight).sum();
    let min_payload = context
        .request
        .payload_upper_bound
        .min(total_cargo_weight)
        * context.request.min_payload_ratio;
    model.add_linear_constraint(
        &aggregation.total_payload_coefficients,
        ConstraintRelation::LessEqual,
        context.request.payload_upper_bound,
        &format!("airworthiness_payload_upper_{}", mode_name(context.mode)),
    )?;
    model.add_linear_constraint(
        &aggregation.total_payload_coefficients,
        ConstraintRelation::GreaterEqual,
        min_payload,
        &format!("airworthiness_payload_lower_{}", mode_name(context.mode)),
    )?;
    Ok(())
}

