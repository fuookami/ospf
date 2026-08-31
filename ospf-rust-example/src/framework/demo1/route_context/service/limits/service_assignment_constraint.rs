use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo1::route_context::model::Assignment;

pub fn apply_service_assignment_constraints(
    model: &mut MetaModel<f64>,
    assignment: &Assignment,
    service_count: usize,
) -> Result<(), Box<dyn Error>> {
    for s in 0..service_count {
        let coefficients: Vec<(usize, f64)> = assignment
            .normal_node_indices
            .iter()
            .enumerate()
            .map(|(row, _)| (assignment.x_idx[row][s], 1.0))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            1.0,
            &format!("service_assignment_{}", s),
        )?;
    }
    Ok(())
}
