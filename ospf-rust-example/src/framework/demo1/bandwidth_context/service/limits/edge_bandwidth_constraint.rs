use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol};
use ospf_rust_multiarray::{MultiArray, Shape};
use crate::framework::demo1::route_context::model::{Edge, Node, Service};

type Symbols1D = SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>;

/// 从多项式中提取指定 var_index 的系数，不存在则返回 0.0
fn coeff_for_var(poly: &ospf_rust_core::symbol::flatten::Linear<f64>, var_index: usize) -> f64 {
    poly.monomials()
        .iter()
        .find(|m| m.var_index() == var_index)
        .map(|m| *m.coefficient())
        .unwrap_or(0.0)
}

/// 对齐 Kotlin EdgeBandwidthConstraint:
/// y[edge, service] <= serviceAssignment[service] * maxBandwidth
///
/// 从符号组合取多项式构建约束：
/// - bandwidth[e] 的多项式包含所有 y[e, s] 的单项式
/// - service_assignment[s] 的多项式包含所有 x[n, s] 的单项式
pub fn apply_edge_bandwidth_constraints(
    model: &mut MetaModel<f64>,
    edges: &[Edge],
    services: &[Service],
    nodes: &[Node],
    bandwidth: &Symbols1D,
    service_assignment: &Symbols1D,
    y_idx: &MultiArray<usize, Shape<2>>,
) -> Result<(), Box<dyn Error>> {
    for (e, edge) in edges.iter().enumerate() {
        if !nodes[edge.from].is_normal() {
            continue;
        }

        // bandwidth[e] 的多项式：sum(y[e, s] for all s)
        let bw_poly = bandwidth.symbol_polynomial(e);

        // 遍历 bandwidth[e] 的每个单项式，每个对应一个 service s
        for mono in bw_poly.monomials() {
            let y_var_index = mono.var_index();

            // 通过 y_idx 反查 service 索引
            let s = find_service_for_var(y_var_index, e, y_idx, services.len());
            let s = match s {
                Some(s) => s,
                None => continue,
            };

            // 构建约束系数：y[e,s] - maxBandwidth * sum(x[n,s]) <= 0
            let mut coefficients: Vec<(usize, f64)> = vec![(y_var_index, 1.0)];

            // 从 service_assignment[s] 的多项式提取 x[n,s] 的单项式
            let sa_poly = service_assignment.symbol_polynomial(s);
            for sa_mono in sa_poly.monomials() {
                coefficients.push((sa_mono.var_index(), -edge.max_bandwidth));
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

/// 通过 y_idx 反查指定 var_index 对应的 service 索引
fn find_service_for_var(
    var_index: usize,
    edge_index: usize,
    y_idx: &MultiArray<usize, Shape<2>>,
    service_count: usize,
) -> Option<usize> {
    for s in 0..service_count {
        if y_idx[&[edge_index, s]] == var_index {
            return Some(s);
        }
    }
    None
}
