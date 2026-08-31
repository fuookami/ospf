use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol};
use ospf_rust_multiarray::Shape;

/// 一维线性表达式符号组合类型别名 / 1D linear expression symbol combination type alias
type Symbols1D = SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>;

/// NodeBandwidth tracks per-node bandwidth demand.
/// Each field is an explicit intermediate symbol combination.
pub struct NodeBandwidth {
    /// 入度符号：sum(in_degree[node][s]) across services
    pub in_degree: Symbols1D,
    /// 出度符号：sum(out_degree[node][s]) across services
    pub out_degree: Symbols1D,
    /// 出流符号：sum(out_flow[node][s]) across services
    pub out_flow: Symbols1D,
}

impl NodeBandwidth {
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
