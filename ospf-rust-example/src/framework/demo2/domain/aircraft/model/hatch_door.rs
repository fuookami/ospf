use super::deck::DeckLocation;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 舱门 / Hatch door (对齐 Kotlin HatchDoor)
#[derive(Debug, Clone)]
pub struct HatchDoor {
    pub name: String,
    pub location: DeckLocation,
    pub beside_bulk: bool,
    pub nose_door: bool,
    pub lateral_arm: Quantity<f64, Unit>,
    pub front_arm: Quantity<f64, Unit>,
    pub back_arm: Quantity<f64, Unit>,
}
