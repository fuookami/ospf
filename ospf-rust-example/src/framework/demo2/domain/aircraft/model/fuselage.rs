use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 救生筏 / Liferaft (对齐 Kotlin Liferaft)
#[derive(Debug, Clone)]
pub struct Liferaft {
    pub weight: Quantity<f64, Unit>,
    pub index: f64,
}

/// 机身 / Fuselage (对齐 Kotlin Fuselage)
#[derive(Debug, Clone)]
pub struct Fuselage {
    pub liferaft: Option<Liferaft>,
    pub dow: Quantity<f64, Unit>,       // Dry Operating Weight
    pub doi: f64,                       // Dry Operating Index
    pub balanced_arm: Quantity<f64, Unit>,
}
