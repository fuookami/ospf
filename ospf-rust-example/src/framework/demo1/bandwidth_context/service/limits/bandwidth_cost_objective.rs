use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo1::route_context::model::{Edge, Node, Service};

pub fn apply_bandwidth_cost_objective(
    model: &mut MetaModel<f64>,
    edges: &[Edge],
    nodes: &[Node],
    services: &[Service],
    y_idx: &[Vec<usize>],
) -> Result<(), Box<dyn Error>> {
    let objective: Vec<(usize, f64)> = edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| nodes[edge.from].is_normal())
        .flat_map(|(e, edge)| {
            (0..services.len()).map(move |s| (y_idx[e][s], edge.cost_per_bandwidth))
        })
        .collect();
    model.add_linear_objective(&objective, "bandwidth_cost");
    Ok(())
}
