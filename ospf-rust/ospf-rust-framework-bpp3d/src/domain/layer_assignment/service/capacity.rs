// Capacity - 容量模型 / Capacity model
// ============================================================================

/// 容量模型 / Capacity model
///
/// 管理载重、体积、深度和装载率表达式。
/// Manages load weight, volume, depth, and loading rate expressions.
#[derive(Debug, Clone)]
pub struct Capacity {
    /// 载重表达式 / Load weight expressions
    pub load_weight: IndexedLinearExpressionSymbols1<usize>,
    /// 载体积表达式 / Load volume expressions
    pub load_volume: IndexedLinearExpressionSymbols1<usize>,
    /// 载深表达式 / Load depth expressions
    pub load_depth: IndexedLinearExpressionSymbols1<usize>,
}

impl Capacity {
    /// 创建容量模型 / Create a capacity model
    pub fn new() -> Self {
        Self {
            load_weight: IndexedLinearExpressionSymbols1::new("loadWeight", &[], Self::empty_combination("loadWeight")),
            load_volume: IndexedLinearExpressionSymbols1::new("loadVolume", &[], Self::empty_combination("loadVolume")),
            load_depth: IndexedLinearExpressionSymbols1::new("loadDepth", &[], Self::empty_combination("loadDepth")),
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

impl Default for Capacity {
    fn default() -> Self {
        Self::new()
    }
}

/// 精确负载容量 / Precise load capacity
///
/// Final MILP 阶段的容量约束数据，包含每个箱的载重、体积和深度上界。
/// Capacity constraint data for the final MILP phase, containing per-bin
/// load weight, volume, and depth upper bounds.
#[derive(Debug, Clone)]
pub struct PreciseLoadCapacity {
    /// 载重上界 / Weight upper bound
    pub weight_capacity: f64,
    /// 体积上界 / Volume upper bound
    pub volume_capacity: f64,
    /// 深度上界 / Depth upper bound
    pub depth_capacity: f64,
}

