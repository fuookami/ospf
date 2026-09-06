//! 配载模式定义 / Stowage mode definitions
/// 配载模式 / Stowage mode (对齐 Kotlin StowageMode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StowageMode {
    /// 预配载 / Predistribution
    Predistribution,
    /// 全配载 / Full load
    FullLoad,
    /// 建议打板 / Weight recommendation
    WeightRecommendation,
}

impl StowageMode {
    /// 是否启用 MAC 优化 / Whether MAC optimization is enabled
    pub fn with_mac_optimization(&self) -> bool {
        match self {
            StowageMode::WeightRecommendation => false,
            _ => true,
        }
    }

    /// 是否启用软性安全约束 / Whether soft security constraints are enabled
    pub fn with_soft_security(&self) -> bool {
        match self {
            StowageMode::WeightRecommendation => false,
            _ => true,
        }
    }

    /// 是否启用载重最大化 / Whether payload maximization is enabled
    pub fn with_payload_maximization(&self) -> bool {
        matches!(self, StowageMode::WeightRecommendation)
    }
}
