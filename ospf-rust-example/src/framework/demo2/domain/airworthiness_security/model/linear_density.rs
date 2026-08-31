/// 线密度 / Linear density (对齐 Kotlin LinearDensity)
#[derive(Debug, Clone)]
pub struct LinearDensity {
    pub weight: f64,
    pub length: f64,
}

impl LinearDensity {
    pub fn density(&self) -> f64 {
        if self.length > 0.0 { self.weight / self.length } else { 0.0 }
    }
}
