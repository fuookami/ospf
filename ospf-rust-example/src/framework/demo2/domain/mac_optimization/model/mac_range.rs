//! MAC 范围模型 / MAC range model
/// MAC 范围 / MAC range (对齐 Kotlin MACRange)
#[derive(Debug, Clone)]
pub struct MacRange {
    /// 最小 MAC 值 / Minimum MAC value
    pub min: f64,
    /// 最大 MAC 值 / Maximum MAC value
    pub max: f64,
}
