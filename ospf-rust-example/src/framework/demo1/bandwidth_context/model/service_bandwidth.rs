use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol};
use ospf_rust_multiarray::Shape;

/// 二维线性表达式符号组合类型别名 / 2D linear expression symbol combination type alias
type Symbols2D = SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<2>>;

/// ServiceBandwidth tracks per-service bandwidth usage.
/// Each field is an explicit intermediate symbol combination derived from EdgeBandwidth.
pub struct ServiceBandwidth {
    /// 入度符号：sum(y[e][s] for e where edge.to == node)
    pub in_degree: Symbols2D,
    /// 出度符号：sum(y[e][s] for e where edge.from == node)
    pub out_degree: Symbols2D,
    /// 出流符号：out_degree - in_degree
    pub out_flow: Symbols2D,
}

impl ServiceBandwidth {
    pub fn new() -> Self {
        let dummy = SymbolCombination::new(Shape::new([0, 0]), "dummy", |_, _| {
            LinearExpressionSymbol::new(0, "dummy", vec![], 0.0)
        });
        Self {
            in_degree: dummy.clone(),
            out_degree: dummy.clone(),
            out_flow: dummy,
        }
    }
}
