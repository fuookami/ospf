//! 需求约束模块 / Demand constraint module

use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol};
use ospf_rust_multiarray::{MultiArray, Shape};
use crate::framework::demo1::route_context::model::{Edge, Node, Service};

/// 一维线性表达式符号组合类型别名 / 1D linear expression symbol combination type alias
type Symbols1D = SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>;

/// 需求约束：对每个客户节点，入边带宽之和 >= 需求 / Demand constraint: for each client node, sum of incoming edge bandwidth >= demand
///
/// 对齐 Kotlin DemandConstraint:
/// 对每个 client 节点，sum(y[e][s] for incoming edges) >= demand
///
/// 从符号组合取多项式构建约束：
/// - bandwidth[e] 的多项式包含所有 y[e, s] 的单项式
/// - 只取 edge.to == client_node 的入边
pub fn apply_demand_constraints(
    model: &mut MetaModel<f64>,
    nodes: &[Node],
    edges: &[Edge],
    services: &[Service],
    bandwidth: &Symbols1D,
) -> Result<(), Box<dyn Error>> {
    for (node_idx, node) in nodes.iter().enumerate() {
        if !node.is_client() {
            continue;
        }

        // 收集所有入边的 bandwidth 多项式中的单项式
        let mut coefficients: Vec<(usize, f64)> = Vec::new();
        for (e, edge) in edges.iter().enumerate() {
            if edge.to == node_idx {
                // bandwidth[e] 的多项式：sum(y[e, s] for all s)
                let poly = bandwidth.symbol_polynomial(e);
                for mono in poly.monomials() {
                    coefficients.push((mono.var_index(), 1.0));
                }
            }
        }

        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::GreaterEqual,
            node.demand,
            &format!("demand_{}", node.id),
        )?;
    }
    Ok(())
}
