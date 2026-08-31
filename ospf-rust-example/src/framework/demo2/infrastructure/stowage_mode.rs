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
    pub fn with_mac_optimization(&self) -> bool {
        match self {
            StowageMode::WeightRecommendation => false,
            _ => true,
        }
    }

    pub fn with_soft_security(&self) -> bool {
        match self {
            StowageMode::WeightRecommendation => false,
            _ => true,
        }
    }

    pub fn with_payload_maximization(&self) -> bool {
        matches!(self, StowageMode::WeightRecommendation)
    }
}
