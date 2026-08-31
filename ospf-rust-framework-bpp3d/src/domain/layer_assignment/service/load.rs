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
#[derive(Debug)]
pub struct Load {
    /// 需求条目 / Demand entries
    pub demand_entries: Vec<Bpp3dDemandEntry>,
    /// 负载表达式 load[layer] / Load expressions load[layer]
    pub load: ExpressionArray1<usize>,
    /// 过载表达式 overLoad[layer] / Overload expressions
    pub over_load: ExpressionArray1<usize>,
    /// 欠载表达式 lessLoad[layer] / Less-load expressions
    pub less_load: ExpressionArray1<usize>,
}

impl Load {
    /// 创建负载模型 / Create a load model
    pub fn new(demand_entries: Vec<Bpp3dDemandEntry>) -> Self {
        Self {
            demand_entries,
            load: ExpressionArray1::new("load"),
            over_load: ExpressionArray1::new("overLoad"),
            less_load: ExpressionArray1::new("lessLoad"),
        }
    }
}

// ============================================================================
