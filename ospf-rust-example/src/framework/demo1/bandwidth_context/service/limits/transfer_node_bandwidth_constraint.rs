use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo1::route_context::model::{Edge, Node, Service};

/// 计算节点的最大出度带宽 / Compute max outgoing bandwidth of a node
fn max_out_degree(node_idx: usize, edges: &[Edge]) -> f64 {
    edges
        .iter()
        .filter(|e| e.from == node_idx)
        .map(|e| e.max_bandwidth)
        .sum()
}

/// 中转节点带宽约束 / Transfer node bandwidth constraint
/// 对齐 Kotlin TransferNodeBandwidthConstraint:
/// maxOutDegree * (1 - nodeAssignment[node]) + outFlow[node] <= maxOutDegree
/// 即: outFlow[node] <= nodeAssignment[node] * maxOutDegree
pub fn apply_transfer_node_bandwidth_constraints(
    model: &mut MetaModel<f64>,
    nodes: &[Node],
    edges: &[Edge],
    services: &[Service],
    y_idx: &[Vec<usize>],
    x_idx: &[Vec<usize>],
    normal_node_indices: &[usize],
) -> Result<(), Box<dyn Error>> {
    for (row, &node_idx) in normal_node_indices.iter().enumerate() {
        let max_out = max_out_degree(node_idx, edges);
        if max_out <= 0.0 {
            continue;
        }

        for s in 0..services.len() {
            // outFlow[node][s] = 出边带宽和 - 入边带宽和
            let mut coefficients: Vec<(usize, f64)> = Vec::new();

            // 出边: +y[e][s]
            for (e, edge) in edges.iter().enumerate() {
                if edge.from == node_idx && !nodes[edge.to].is_client() {
                    coefficients.push((y_idx[e][s], 1.0));
                }
            }

            // 入边: -y[e][s]
            for (e, edge) in edges.iter().enumerate() {
                if edge.to == node_idx && !nodes[edge.from].is_client() {
                    coefficients.push((y_idx[e][s], -1.0));
                }
            }

            // -maxOut * x[node][s] (约束: outFlow <= x * maxOut)
            coefficients.push((x_idx[row][s], -max_out));

            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                0.0,
                &format!("transfer_node_bandwidth_{}_{}", node_idx, s),
            )?;
        }
    }
    Ok(())
}
