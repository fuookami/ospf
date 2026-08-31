/// 冗余 / Redundancy (对齐 Kotlin Redundancy)
/// 计算主甲板的备用容量并注册松弛变量
#[derive(Debug, Clone)]
pub struct Redundancy {
    pub main_deck_capacity: f64,
    pub actual_load: f64,
    pub slack: f64,
}

impl Redundancy {
    pub fn redundancy(&self) -> f64 {
        self.main_deck_capacity - self.actual_load
    }
}

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
