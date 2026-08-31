//! 带宽成本目标函数模块 / Bandwidth cost objective function module

use crate::framework::demo1::route_context::model::{Edge, Node, Service};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::{LinearExpressionSymbol, SymbolCombination};
use ospf_rust_multiarray::Shape;
use std::error::Error;

/// 一维线性表达式符号组合类型别名 / 1D linear expression symbol combination type alias
type Symbols1D = SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>;

/// 带宽成本目标函数 / Bandwidth cost objective function
///
/// 从符号组合取多项式构建目标：
/// - bandwidth[e] 的多项式包含所有 y[e, s] 的单项式
/// - 每个单项式的系数乘以 edge.cost_per_bandwidth
///
/// Builds objective from symbol combination polynomials:
/// - bandwidth[e] polynomial contains all y[e, s] monomials
/// - Each monomial coefficient is multiplied by edge.cost_per_bandwidth
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
