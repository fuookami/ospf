use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework_demo::demo2::domain::airworthiness::aggregation::AirworthinessAggregation;
use crate::framework_demo::demo2::domain::airworthiness::context::AirworthinessContext;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::mode_name;

pub fn apply_envelope_limits(
    model: &mut MetaModel<f64>,
    context: &AirworthinessContext<'_>,
    aggregation: &AirworthinessAggregation,
) -> Result<(), Box<dyn Error>> {
    model.add_linear_constraint(
        &aggregation.envelope_longitudinal_moment_coefficients,
        ConstraintRelation::GreaterEqual,
        context.request.envelope_longitudinal_moment_min,
        &format!(
            "airworthiness_envelope_longitudinal_min_{}",
            mode_name(context.mode)
        ),
    )?;
    model.add_linear_constraint(
        &aggregation.envelope_longitudinal_moment_coefficients,
        ConstraintRelation::LessEqual,
        context.request.envelope_longitudinal_moment_max,
        &format!(
            "airworthiness_envelope_longitudinal_max_{}",
            mode_name(context.mode)
        ),
    )?;
    Ok(())
}
