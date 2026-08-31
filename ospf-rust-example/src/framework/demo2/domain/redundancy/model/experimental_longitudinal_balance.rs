/// 实验性纵向平衡 / Experimental longitudinal balance (对齐 Kotlin ExperimentalLongitudinalBalance)
#[derive(Debug, Clone)]
pub struct ExperimentalLongitudinalBalance {
    pub mac_value: f64,
    pub min_mac: f64,
    pub max_mac: f64,
}

impl ExperimentalLongitudinalBalance {
    pub fn in_range(&self) -> bool {
        self.mac_value >= self.min_mac && self.mac_value <= self.max_mac
    }
}
