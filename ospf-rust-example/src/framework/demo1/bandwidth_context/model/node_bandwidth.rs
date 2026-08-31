//! 节点带宽模型：定义节点维度的带宽变量与符号 / Node bandwidth model: defines node-dimension bandwidth variables and symbols

use ospf_rust_core::symbol::{LinearExpressionSymbol, SymbolCombination};
use ospf_rust_multiarray::Shape;

/// 一维线性表达式符号组合类型别名 / 1D linear expression symbol combination type alias
type Symbols1D = SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>;

/// 节点带宽模型 / Node bandwidth model
///
/// 跟踪每个节点的带宽需求，每个字段为显式中间符号组合。
/// Tracks per-node bandwidth demand. Each field is an explicit intermediate symbol combination.
pub struct NodeBandwidth {
    /// 入度符号：sum(in_degree[node][s]) across services / In-degree symbol: sum across services
    pub in_degree: Symbols1D,
    /// 出度符号：sum(out_degree[node][s]) across services / Out-degree symbol: sum across services
    pub out_degree: Symbols1D,
    /// 出流符号：sum(out_flow[node][s]) across services / Out-flow symbol: sum across services
    pub out_flow: Symbols1D,
}

impl NodeBandwidth {
    /// 创建新的节点带宽模型 / Create a new node bandwidth model
    pub fn new() -> Self {
        let dummy = SymbolCombination::new(Shape::new([0]), "dummy", |_, _| {
            LinearExpressionSymbol::new(0, "dummy", vec![], 0.0)
        });
        Self {
            in_degree: dummy.clone(),
            out_degree: dummy.clone(),
            out_flow: dummy,
        }
    }
}
