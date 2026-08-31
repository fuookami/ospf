use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo1::route_context::model::{Edge, Node, Service};

pub fn apply_demand_constraints(
    model: &mut MetaModel<f64>,
    nodes: &[Node],
    edges: &[Edge],
    services: &[Service],
    y_idx: &[Vec<usize>],
) -> Result<(), Box<dyn Error>> {
    for (node_idx, node) in nodes.iter().enumerate() {
        if !node.is_client() {
            continue;
        }
        let coefficients: Vec<(usize, f64)> = edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.to == node_idx)
            .flat_map(|(e, _)| {
                (0..services.len()).map(move |s| (y_idx[e][s], 1.0))
            })
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::GreaterEqual,
            node.demand,
            &format!("demand_{}", node.id),
        )?;
    }
    Ok(())
}
