use super::flight_phase::FlightPhase;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 燃油常数 / Fuel constant
#[derive(Debug, Clone)]
pub struct FuelConstant {
    pub weight: Quantity<f64, Unit>,
    pub arm: Quantity<f64, Unit>,
}

/// 燃油箱 / Fuel tank
#[derive(Debug, Clone)]
pub struct FuelTank {
    pub name: String,
    pub capacity: Quantity<f64, Unit>,
}
