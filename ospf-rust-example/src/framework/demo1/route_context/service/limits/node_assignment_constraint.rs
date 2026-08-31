use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo1::route_context::model::Assignment;

pub fn apply_node_assignment_constraints(
    model: &mut MetaModel<f64>,
    assignment: &Assignment,
    _node_count: usize,
) -> Result<(), Box<dyn Error>> {
    for (row, _) in assignment.normal_node_indices.iter().enumerate() {
        let coefficients: Vec<(usize, f64)> = (0..assignment.x_idx[row].len())
            .map(|s| (assignment.x_idx[row][s], 1.0))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            1.0,
            &format!("node_assignment_{}", row),
        )?;
    }
    Ok(())
}
