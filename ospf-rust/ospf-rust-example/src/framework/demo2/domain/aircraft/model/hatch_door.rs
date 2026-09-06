//! 舱门模型定义 / Hatch door model definitions
use super::deck::DeckLocation;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 舱门 / Hatch door (对齐 Kotlin HatchDoor)
#[derive(Debug, Clone)]
pub struct HatchDoor {
    /// 舱门名称 / Door name
    pub name: String,
    /// 舱门所在甲板位置 / Door deck location
    pub location: DeckLocation,
    /// 是否靠近散货舱 / Whether beside bulk compartment
    pub beside_bulk: bool,
    /// 是否为鼻门 / Whether nose door
    pub nose_door: bool,
    /// 横向力臂 / Lateral arm
    pub lateral_arm: Quantity<f64, Unit>,
    /// 前向力臂 / Forward arm
    pub front_arm: Quantity<f64, Unit>,
    /// 后向力臂 / Aft arm
    pub back_arm: Quantity<f64, Unit>,
}
