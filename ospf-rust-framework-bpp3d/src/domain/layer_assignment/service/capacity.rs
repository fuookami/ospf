// Capacity - 容量模型 / Capacity model
// ============================================================================

/// 容量模型 / Capacity model
///
/// 管理载重、体积、深度和装载率表达式。
/// Manages load weight, volume, depth, and loading rate expressions.
#[derive(Debug)]
pub struct Capacity {
    /// 载重表达式 / Load weight expressions
    pub load_weight: ExpressionArray1<usize>,
    /// 载体积表达式 / Load volume expressions
    pub load_volume: ExpressionArray1<usize>,
    /// 载深表达式 / Load depth expressions
    pub load_depth: ExpressionArray1<usize>,
}

impl Capacity {
    /// 创建容量模型 / Create a capacity model
    pub fn new() -> Self {
        Self {
            load_weight: ExpressionArray1::new("loadWeight"),
            load_volume: ExpressionArray1::new("loadVolume"),
            load_depth: ExpressionArray1::new("loadDepth"),
        }
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

