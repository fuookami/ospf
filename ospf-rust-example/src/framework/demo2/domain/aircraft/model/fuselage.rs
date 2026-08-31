//! 机身模型定义 / Fuselage model definitions
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 救生筏 / Liferaft (对齐 Kotlin Liferaft)
#[derive(Debug, Clone)]
pub struct Liferaft {
    /// 救生筏重量 / Liferaft weight
    pub weight: Quantity<f64, Unit>,
    /// 救生筏指数 / Liferaft index
    pub index: f64,
}

/// 机身 / Fuselage (对齐 Kotlin Fuselage)
#[derive(Debug, Clone)]
pub struct Fuselage {
    /// 救生筏 / Liferaft
    pub liferaft: Option<Liferaft>,
    /// 干运行重量 / Dry operating weight
    pub dow: Quantity<f64, Unit>,
    /// 干运行指数 / Dry operating index
    pub doi: f64,
    /// 平衡力臂 / Balanced arm
    pub balanced_arm: Quantity<f64, Unit>,
}
