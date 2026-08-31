// ============================================================================
// Load - 负载模型 / Load model
// ============================================================================

/// 需求条目 / Demand entry
#[derive(Debug, Clone)]
pub struct Bpp3dDemandEntry {
    /// 需求模式 / Demand mode
    pub mode: Bpp3dDemandMode,
    /// 需求键 / Demand key
    pub key: Bpp3dDemandKey,
    /// 需求值 / Demand value
    pub demand: f64,
}

/// 负载模型 / Load model
///
/// 管理需求约束的中间表达式。
/// Manages intermediate expressions for demand constraints.
#[derive(Debug, Clone)]
pub struct Load {
    /// 需求条目 / Demand entries
    pub demand_entries: Vec<Bpp3dDemandEntry>,
    /// 负载表达式 load[layer] / Load expressions load[layer]
    pub load: IndexedLinearExpressionSymbols1<usize>,
    /// 过载表达式 overLoad[layer] / Overload expressions
    pub over_load: IndexedLinearExpressionSymbols1<usize>,
    /// 欠载表达式 lessLoad[layer] / Less-load expressions
    pub less_load: IndexedLinearExpressionSymbols1<usize>,
}

impl Load {
    /// 创建负载模型 / Create a load model
    pub fn new(demand_entries: Vec<Bpp3dDemandEntry>) -> Self {
        Self {
            demand_entries,
            load: IndexedLinearExpressionSymbols1::new("load", &[], Self::empty_combination("load")),
            over_load: IndexedLinearExpressionSymbols1::new("overLoad", &[], Self::empty_combination("overLoad")),
            less_load: IndexedLinearExpressionSymbols1::new("lessLoad", &[], Self::empty_combination("lessLoad")),
        }
    }

    fn empty_combination(prefix: &str) -> ospf_rust_core::symbol::SymbolCombination<f64, ospf_rust_core::symbol::LinearExpressionSymbol<f64>, ospf_rust_multiarray::Shape<1>> {
        use ospf_rust_core::symbol::SymbolCombination;
        use ospf_rust_core::symbol::LinearExpressionSymbol;
        use ospf_rust_multiarray::Shape;
        SymbolCombination::new(
            Shape::new([0]),
            prefix,
            |_index, _vector| {
                LinearExpressionSymbol::new(0, prefix, Vec::new(), 0.0)
            },
        )
    }
}

// ============================================================================
