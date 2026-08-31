use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::mode_name;
use crate::framework_demo::demo2::domain::stowage::aggregation::StowageAggregation;
use crate::framework_demo::demo2::domain::stowage::context::StowageContext;

pub fn apply_assignment_limits(
    model: &mut MetaModel<f64>,
    context: &StowageContext<'_>,
    aggregation: &StowageAggregation,
) -> Result<(), Box<dyn Error>> {
    for c in 0..context.request.cargos.len() {
        let coefficients: Vec<(usize, f64)> = (0..context.request.positions.len())
            .map(|p| (context.x_idx[c][p], 1.0))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            aggregation.assignment_relation,
            aggregation.assignment_rhs,
            &format!("stowage_assignment_{}_{}", mode_name(context.mode), c),
        )?;
    }
    Ok(())
}
