/// 纵向平衡 / Longitudinal balance (对齐 Kotlin LongitudinalBalance)
/// MAC 值为无量纲百分比，保持 f64
#[derive(Debug, Clone)]
pub struct LongitudinalBalance {
    pub mac_value: f64,
    pub min_mac: f64,
    pub max_mac: f64,
}

impl LongitudinalBalance {
    pub fn in_range(&self) -> bool {
        self.mac_value >= self.min_mac && self.mac_value <= self.max_mac
    }
}
