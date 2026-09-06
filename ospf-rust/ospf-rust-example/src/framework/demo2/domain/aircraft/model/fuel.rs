//! 燃油模型定义 / Fuel model definitions
use super::flight_phase::FlightPhase;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 燃油常数 / Fuel constant
#[derive(Debug, Clone)]
pub struct FuelConstant {
    /// 燃油重量 / Fuel weight
    pub weight: Quantity<f64, Unit>,
    /// 燃油力臂 / Fuel arm
    pub arm: Quantity<f64, Unit>,
}

/// 燃油箱 / Fuel tank
#[derive(Debug, Clone)]
pub struct FuelTank {
    /// 油箱名称 / Tank name
    pub name: String,
    /// 油箱容量 / Tank capacity
    pub capacity: Quantity<f64, Unit>,
}
