/// 表面密度 / Surface density (对齐 Kotlin SurfaceDensity)
#[derive(Debug, Clone)]
pub struct SurfaceDensity {
    pub weight: f64,
    pub area: f64,
}

impl SurfaceDensity {
    pub fn density(&self) -> f64 {
        if self.area > 0.0 { self.weight / self.area } else { 0.0 }
    }
}
