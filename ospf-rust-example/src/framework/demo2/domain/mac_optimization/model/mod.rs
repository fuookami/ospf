/// MAC 范围 / MAC range (对齐 Kotlin mac_optimization MACRange)
#[derive(Debug, Clone)]
pub struct MacRange {
    pub min: f64,
    pub max: f64,
}

/// 纵向平衡 / Longitudinal balance (对齐 Kotlin mac_optimization LongitudinalBalance)
#[derive(Debug, Clone)]
pub struct LongitudinalBalance {
    pub mac_value: f64,
    pub mac_range: MacRange,
    pub in_range: bool,
}

/// 横向平衡 / Lateral balance (对齐 Kotlin mac_optimization LateralBalance)
#[derive(Debug, Clone)]
pub struct LateralBalance {
    pub moment: f64,
    pub max_imbalance: f64,
    pub in_range: bool,
}
