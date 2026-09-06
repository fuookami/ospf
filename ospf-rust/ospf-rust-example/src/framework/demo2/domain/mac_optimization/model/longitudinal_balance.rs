//! 纵向平衡模型 / Longitudinal balance model
/// 纵向平衡 / Longitudinal balance (对齐 Kotlin LongitudinalBalance)
/// MAC 值为无量纲百分比，保持 f64
#[derive(Debug, Clone)]
pub struct LongitudinalBalance {
    /// MAC 值 / MAC value
    pub mac_value: f64,
    /// 最小 MAC 限制 / Minimum MAC limit
    pub min_mac: f64,
    /// 最大 MAC 限制 / Maximum MAC limit
    pub max_mac: f64,
}

impl LongitudinalBalance {
    /// 判断 MAC 值是否在安全范围内 / Check whether MAC value is within safe range
    pub fn in_range(&self) -> bool {
        self.mac_value >= self.min_mac && self.mac_value <= self.max_mac
    }
}
