use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;
use std::collections::HashMap;
use super::aircraft_model::AircraftModel;
use super::deck::DeckLocation;
/// 舱位位置标签 / Position location tag (对齐 Kotlin PositionLocationTag)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositionLocationTag {
    Main,
    Low,
    Bulk,
    Head,
    Tail,
    Forward,
    Aft,
}

/// 舱位位置 / Position location (对齐 Kotlin PositionLocation)
#[derive(Debug, Clone)]
pub struct PositionLocation {
    pub tags: Vec<PositionLocationTag>,
}

impl PositionLocation {
    pub fn main(&self) -> bool { self.tags.contains(&PositionLocationTag::Main) }
    pub fn low(&self) -> bool { self.tags.contains(&PositionLocationTag::Low) }
    pub fn bulk(&self) -> bool { self.tags.contains(&PositionLocationTag::Bulk) }
    pub fn head(&self) -> bool { self.tags.contains(&PositionLocationTag::Head) }
    pub fn tail(&self) -> bool { self.tags.contains(&PositionLocationTag::Tail) }

    pub fn deck_location(&self) -> DeckLocation {
        if self.tags.contains(&PositionLocationTag::Main) {
            DeckLocation::Main
        } else if self.tags.contains(&PositionLocationTag::Forward) {
            DeckLocation::LowForward
        } else {
            DeckLocation::LowAft
        }
    }
}

/// 舱位坐标 / Position coordinate (对齐 Kotlin PositionCoordinate)
#[derive(Debug, Clone)]
pub struct PositionCoordinate {
    pub front_arm: Quantity<f64, Unit>,
    pub back_arm: Quantity<f64, Unit>,
    pub left_arm: Quantity<f64, Unit>,
    pub right_arm: Quantity<f64, Unit>,
    pub offsets: HashMap<String, f64>, // ULDCode -> offset
}

impl PositionCoordinate {
    pub fn longitudinal_arm(&self) -> f64 {
        (self.front_arm.value + self.back_arm.value) / 2.0
    }

    pub fn lateral_arm(&self) -> f64 {
        (self.left_arm.value + self.right_arm.value) / 2.0
    }

    pub fn transverse(&self) -> bool {
        (self.left_arm.value - self.right_arm.value).abs() > 1e-6
    }
}

/// 舱位形状 / Position shape
#[derive(Debug, Clone)]
pub struct PositionShape {
    pub width: Quantity<f64, Unit>,
    pub length: Quantity<f64, Unit>,
    pub height: Quantity<f64, Unit>,
}

/// 舱位类型代码 / Position type code (对齐 Kotlin PositionTypeCode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositionTypeCode {
    EmptyForbidden,
    EmptyHated,
    EmptyItemForbidden,
    CrushItemForbidden,
    StiffCargoForbidden,
    AOGMATAppointed,
    NormalBulkAppointed,
}

/// 舱位类型 / Position type
#[derive(Debug, Clone)]
pub struct PositionType {
    pub codes: Vec<PositionTypeCode>,
}

impl PositionType {
    pub fn contains(&self, code: &PositionTypeCode) -> bool {
        self.codes.contains(code)
    }
}

/// 舱位 / Position (aircraft domain, 对齐 Kotlin aircraft Position)
#[derive(Debug, Clone)]
pub struct Position {
    pub id: String,
    pub space_name: String,
    pub alpha_space_name: String,
    pub size_code: String,
    pub loading_order: u32,
    pub coordinate: PositionCoordinate,
    pub shape: PositionShape,
    pub location: PositionLocation,
}

/// 舱位对 / Position pair
pub type PositionPair = (Position, Position);
