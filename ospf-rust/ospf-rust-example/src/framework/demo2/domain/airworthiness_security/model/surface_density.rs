//! 表面密度模型 / Surface density model
/// 表面密度 / Surface density (对齐 Kotlin SurfaceDensity)
#[derive(Debug, Clone)]
pub struct SurfaceDensity {
    /// 重量 / Weight
    pub weight: f64,
    /// 面积 / Area
    pub area: f64,
}

impl SurfaceDensity {
    /// 计算表面密度（重量/面积） / Compute surface density (weight / area)
    pub fn density(&self) -> f64 {
        if self.area > 0.0 {
            self.weight / self.area
        } else {
            0.0
        }
    }
}
