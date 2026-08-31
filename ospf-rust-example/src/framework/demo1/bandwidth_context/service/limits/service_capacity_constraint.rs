use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo1::route_context::model::{Edge, Node, Service};

/// 对齐 Kotlin ServiceCapacityConstraint:
/// 逐节点、逐服务的流出约束: outFlow[node, service] <= x[node, service] * capacity
/// 其中 outFlow = 出度 - 入度 (仅计算 normal node 之间的边)
pub fn apply_service_capacity_constraints(
    model: &mut MetaModel<f64>,
    nodes: &[Node],
    edges: &[Edge],
    services: &[Service],
    y_idx: &[Vec<usize>],
    x_idx: &[Vec<usize>],
    normal_node_indices: &[usize],
) -> Result<(), Box<dyn Error>> {
    for (row, &node_idx) in normal_node_indices.iter().enumerate() {
        for s in 0..services.len() {
            // outFlow[node][s] = sum(y[e][s] for e where edge.from == node)
            //                   - sum(y[e][s] for e where edge.to == node)
            // outFlow[node][s] <= x[node][s] * capacity
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

            // -x[node][s] * capacity (移到右边)
            coefficients.push((x_idx[row][s], -services[s].capacity));

            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                0.0,
                &format!("service_capacity_{}_{}", node_idx, s),
            )?;
        }
    }
    Ok(())
}
