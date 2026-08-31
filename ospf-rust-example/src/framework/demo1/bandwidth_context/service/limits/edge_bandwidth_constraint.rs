use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo1::route_context::model::{Edge, Node, Service};

/// 对齐 Kotlin EdgeBandwidthConstraint:
/// (1 - serviceAssignment[service]) * maxBandwidth + y[edge, service] <= maxBandwidth
/// 等价于: y[edge, service] <= serviceAssignment[service] * maxBandwidth
/// 其中 serviceAssignment[service] = sum(x[n, service] for all normal nodes)
pub fn apply_edge_bandwidth_constraints(
    model: &mut MetaModel<f64>,
    edges: &[Edge],
    services: &[Service],
    nodes: &[Node],
    y_idx: &[Vec<usize>],
    x_idx: &[Vec<usize>],
    normal_node_indices: &[usize],
) -> Result<(), Box<dyn Error>> {
    for (e, edge) in edges.iter().enumerate() {
        if !nodes[edge.from].is_normal() {
            continue;
        }
        for s in 0..services.len() {
            // y[e][s] <= sum(x[n][s]) * maxBandwidth
            let mut coefficients: Vec<(usize, f64)> = vec![(y_idx[e][s], 1.0)];
            for (row, _) in normal_node_indices.iter().enumerate() {
                coefficients.push((x_idx[row][s], -edge.max_bandwidth));
            }
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                0.0,
                &format!("service_gate_{}_{}", e, s),
            )?;
        }
    }
    Ok(())
}
