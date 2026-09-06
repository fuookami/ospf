//! 线密度模型 / Linear density model
/// 线密度 / Linear density (对齐 Kotlin LinearDensity)
#[derive(Debug, Clone)]
pub struct LinearDensity {
    /// 重量 / Weight
    pub weight: f64,
    /// 长度 / Length
    pub length: f64,
}

impl LinearDensity {
    /// 计算线密度（重量/长度） / Compute linear density (weight / length)
    pub fn density(&self) -> f64 {
        if self.length > 0.0 {
            self.weight / self.length
        } else {
            0.0
        }
    }
}
