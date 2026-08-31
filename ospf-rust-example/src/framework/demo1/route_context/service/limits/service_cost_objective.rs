//! 服务成本目标 / Service cost objective

use crate::framework::demo1::route_context::model::{Assignment, Service};
use ospf_rust_core::model::MetaModel;
use std::error::Error;

/// 服务成本目标函数 / Service cost objective function
///
/// 从 node_assignment[node] 符号组合取多项式：
/// node_assignment[node] = sum(x[node, s] for all s)
/// 每个单项式的系数乘以 service.cost
pub fn apply_service_cost_objective(
    model: &mut MetaModel<f64>,
    assignment: &Assignment,
    services: &[Service],
) -> Result<(), Box<dyn Error>> {
    let mut objective: Vec<(usize, f64)> = Vec::new();

    for (row, _) in assignment.normal_node_indices.iter().enumerate() {
        // node_assignment[node] 的多项式 = sum(x[node, s])
        let poly = assignment.node_assignment.symbol_polynomial(row);
        for mono in poly.monomials() {
            // 通过 var_index 在 x_idx 中反查 service 索引
            let s = find_service_for_x_var(mono.var_index(), row, assignment);
            if let Some(s) = s {
                if s < services.len() {
                    objective.push((mono.var_index(), services[s].cost));
                }
            }
        }
    }

    model.add_linear_objective(&objective, "service_cost");
    Ok(())
}

/// 通过 x_idx 反查指定 var_index 对应的 service 索引 / Reverse-lookup service index for specified var_index via x_idx
fn find_service_for_x_var(var_index: usize, row: usize, assignment: &Assignment) -> Option<usize> {
    let service_count = assignment.x_idx.shape[1];
    for s in 0..service_count {
        if assignment.x_idx[&[row, s]] == var_index {
            return Some(s);
        }
    }
    None
}
