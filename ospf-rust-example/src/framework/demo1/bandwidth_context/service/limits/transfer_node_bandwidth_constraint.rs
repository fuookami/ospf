use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol};
use ospf_rust_multiarray::{MultiArray, Shape};
use crate::framework::demo1::route_context::model::{Edge, Node, Service};

type Symbols1D = SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>;

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
/// outFlow[node] <= nodeAssignment[node] * maxOutDegree
///
/// 从符号组合取多项式构建约束：
/// - bandwidth[e] 的多项式包含所有 y[e, s] 的单项式
/// - node_assignment[node] 的多项式包含所有 x[node, s] 的单项式
pub fn apply_transfer_node_bandwidth_constraints(
    model: &mut MetaModel<f64>,
    nodes: &[Node],
    edges: &[Edge],
    services: &[Service],
    bandwidth: &Symbols1D,
    node_assignment: &Symbols1D,
    y_idx: &MultiArray<usize, Shape<2>>,
    normal_node_indices: &[usize],
) -> Result<(), Box<dyn Error>> {
    for (row, &node_idx) in normal_node_indices.iter().enumerate() {
        let max_out = max_out_degree(node_idx, edges);
        if max_out <= 0.0 {
            continue;
        }

        for s in 0..services.len() {
            let mut coefficients: Vec<(usize, f64)> = Vec::new();

            // 出边: +y[e][s] — 从 bandwidth[e] 的多项式提取
            for (e, edge) in edges.iter().enumerate() {
                if edge.from == node_idx && !nodes[edge.to].is_client() {
                    if let Some(coeff) = extract_service_coeff(bandwidth, e, s, y_idx) {
                        coefficients.push((y_idx[&[e, s]], coeff));
                    }
                }
            }

            // 入边: -y[e][s]
            for (e, edge) in edges.iter().enumerate() {
                if edge.to == node_idx && !nodes[edge.from].is_client() {
                    if let Some(coeff) = extract_service_coeff(bandwidth, e, s, y_idx) {
                        coefficients.push((y_idx[&[e, s]], -coeff));
                    }
                }
            }

            // -maxOut * x[node][s] — 从 node_assignment[node] 的多项式提取
            let na_poly = node_assignment[row].to_linear_polynomial();
            for mono in na_poly.monomials() {
                coefficients.push((mono.var_index(), -max_out));
            }

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

/// 从 bandwidth[e] 的多项式中提取指定 service s 的系数
fn extract_service_coeff(
    bandwidth: &Symbols1D,
    edge_index: usize,
    service_index: usize,
    y_idx: &MultiArray<usize, Shape<2>>,
) -> Option<f64> {
    let target_var = y_idx[&[edge_index, service_index]];
    let poly = bandwidth[edge_index].to_linear_polynomial();
    poly.monomials()
        .iter()
        .find(|m| m.var_index() == target_var)
        .map(|m| *m.coefficient())
}
