use std::error::Error;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol};
use ospf_rust_multiarray::Shape;
use crate::framework::demo1::route_context::model::{Edge, Node, Service};

type Symbols1D = SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>;

/// 带宽成本目标函数
///
/// 从符号组合取多项式构建目标：
/// - bandwidth[e] 的多项式包含所有 y[e, s] 的单项式
/// - 每个单项式的系数乘以 edge.cost_per_bandwidth
pub fn apply_bandwidth_cost_objective(
    model: &mut MetaModel<f64>,
    edges: &[Edge],
    nodes: &[Node],
    services: &[Service],
    bandwidth: &Symbols1D,
) -> Result<(), Box<dyn Error>> {
    let mut objective: Vec<(usize, f64)> = Vec::new();

    for (e, edge) in edges.iter().enumerate() {
        if !nodes[edge.from].is_normal() {
            continue;
        }

        // bandwidth[e] 的多项式：sum(y[e, s] for all s)
        let poly = bandwidth.symbol_polynomial(e);
        for mono in poly.monomials() {
            objective.push((mono.var_index(), edge.cost_per_bandwidth));
        }
    }

    model.add_linear_objective(&objective, "bandwidth_cost");
    Ok(())
}
