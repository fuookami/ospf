/// 冗余 / Redundancy (对齐 Kotlin Redundancy)
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
