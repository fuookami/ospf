use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo1::route_context::model::{Assignment, Service};

pub fn apply_service_cost_objective(
    model: &mut MetaModel<f64>,
    assignment: &Assignment,
    services: &[Service],
) -> Result<(), Box<dyn Error>> {
    let objective: Vec<(usize, f64)> = assignment
        .normal_node_indices
        .iter()
        .enumerate()
        .flat_map(|(row, _)| {
            services
                .iter()
                .enumerate()
                .map(move |(s, service)| (assignment.x_idx[row][s], service.cost))
        })
        .collect();
    model.add_linear_objective(&objective, "service_cost");
    Ok(())
}
