use std::collections::HashMap;

/// 飞机类型枚举 / Aircraft type enum (对齐 Kotlin AircraftType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AircraftType {
    B737,
    B757,
    B767,
    B747,
}

impl AircraftType {
    pub fn ballast_needed(&self) -> bool {
        matches!(self, AircraftType::B737 | AircraftType::B757)
    }

    pub fn main_deck_door_empty_prefer(&self) -> bool {
        matches!(self, AircraftType::B747)
    }
}

/// 飞机子型号 / Aircraft minor model
#[derive(Debug, Clone)]
pub struct AircraftMinorModel {
    pub name: String,
    pub aircraft_type: AircraftType,
}

/// 飞机型号 / Aircraft model (对齐 Kotlin AircraftModel)
#[derive(Debug, Clone)]
pub struct AircraftModel {
    pub name: String,
    pub aircraft_type: AircraftType,
    pub minor_model: AircraftMinorModel,
    pub wide_body: bool,
}

impl AircraftModel {
    pub fn gravity(&self, weight: f64) -> f64 {
        weight * 9.80665 // 标准重力加速度
    }
}

/// 甲板位置 / Deck location (对齐 Kotlin DeckLocation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckLocation {
    Main,
    LowForward,
    LowAft,
}

/// 甲板 / Deck (对齐 Kotlin Deck)
#[derive(Debug, Clone)]
pub struct Deck {
    pub name: String,
    pub location: DeckLocation,
    pub positions: Vec<Position>,
}

/// 飞行阶段 / Flight phase (对齐 Kotlin FlightPhase)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightPhase {
    ZeroFuel,
    TakeOff,
    Landing,
}

/// 公式 / Formula (对齐 Kotlin Formula)
#[derive(Debug, Clone)]
pub struct Formula {
    pub lip: f64,
    pub chord: f64,
    pub standard_datum: f64,
    pub force_distance_coefficient: f64,
    pub doi_correction: f64,
}

impl Formula {
    pub fn balanced_arm(&self, dow: f64, doi: f64, liferaft_weight: f64, liferaft_arm: f64) -> f64 {
        let numerator = dow * self.standard_datum + doi * self.chord + liferaft_weight * liferaft_arm;
        let denominator = dow + doi + liferaft_weight;
        if denominator > 0.0 { numerator / denominator } else { 0.0 }
    }

    pub fn arm(&self, _weight: f64, arm_distance: f64) -> f64 {
        arm_distance - self.standard_datum
    }

    pub fn index(&self, weight: f64, arm_distance: f64) -> f64 {
        let mac_arm = self.arm(weight, arm_distance);
        mac_arm * weight * self.force_distance_coefficient + self.doi_correction
    }

    pub fn mac(&self, weight: f64, arm_distance: f64) -> f64 {
        // MAC 百分比计算
        let index = self.index(weight, arm_distance);
        index / self.chord * 100.0
    }
}

/// 燃油常数 / Fuel constant
#[derive(Debug, Clone)]
pub struct FuelConstant {
    pub weight: f64,
    pub arm: f64,
}

/// 燃油箱 / Fuel tank
#[derive(Debug, Clone)]
pub struct FuelTank {
    pub name: String,
    pub capacity: f64,
}

/// 救生筏 / Liferaft (对齐 Kotlin Liferaft)
#[derive(Debug, Clone)]
pub struct Liferaft {
    pub weight: f64,
    pub index: f64,
}

/// 机身 / Fuselage (对齐 Kotlin Fuselage)
#[derive(Debug, Clone)]
pub struct Fuselage {
    pub liferaft: Option<Liferaft>,
    pub dow: f64,       // Dry Operating Weight
    pub doi: f64,       // Dry Operating Index
    pub balanced_arm: f64,
}

/// 舱门 / Hatch door (对齐 Kotlin HatchDoor)
#[derive(Debug, Clone)]
pub struct HatchDoor {
    pub name: String,
    pub location: DeckLocation,
    pub beside_bulk: bool,
    pub nose_door: bool,
    pub lateral_arm: f64,
    pub front_arm: f64,
    pub back_arm: f64,
}

/// 装载顺序 / Loading order (对齐 Kotlin LoadingOrder)
#[derive(Debug, Clone)]
pub struct LoadingOrder {
    pub position: String,
    pub order: u32,
    pub direct_prec: Option<String>,
    pub direct_succ: Option<String>,
}

/// 邻接类型 / Neighbour type (对齐 Kotlin NeighbourType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NeighbourType {
    Physics,
    IndirectPhysics,
    LinearLoadingOrder,
    TopologicalLoadingOrder,
}

impl NeighbourType {
    pub fn ordered(&self) -> bool {
        matches!(self, NeighbourType::LinearLoadingOrder | NeighbourType::TopologicalLoadingOrder)
    }
}

/// 邻接关系 / Neighbour
#[derive(Debug, Clone)]
pub struct Neighbour {
    pub from: String,
    pub to: String,
    pub neighbour_type: NeighbourType,
}

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
    pub front_arm: f64,
    pub back_arm: f64,
    pub left_arm: f64,
    pub right_arm: f64,
    pub offsets: HashMap<String, f64>, // ULDCode -> offset
}

impl PositionCoordinate {
    pub fn longitudinal_arm(&self) -> f64 {
        (self.front_arm + self.back_arm) / 2.0
    }

    pub fn lateral_arm(&self) -> f64 {
        (self.left_arm + self.right_arm) / 2.0
    }

    pub fn transverse(&self) -> bool {
        (self.left_arm - self.right_arm).abs() > 1e-6
    }
}

/// 舱位形状 / Position shape
#[derive(Debug, Clone)]
pub struct PositionShape {
    pub width: f64,
    pub length: f64,
    pub height: f64,
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

/// ULD 分类 / ULD category (对齐 Kotlin ULDCategory)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UldCategory {
    Pallet,
    Container,
}

/// ULD 代码 / ULD code (对齐 Kotlin ULDCode enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UldCode {
    PAG, PAJ, PMC, PMD, PLA, PLB, PLP, PGE, PGA, PQP, PQF, FQA,
    AKE, DPE, P6P, LAY, ALF, AMA, AMD,
}

impl UldCode {
    pub fn category(&self) -> UldCategory {
        match self {
            UldCode::PAG | UldCode::PAJ | UldCode::PMC | UldCode::PMD
            | UldCode::PLA | UldCode::PLB | UldCode::PLP | UldCode::PGE
            | UldCode::PGA | UldCode::PQP | UldCode::PQF | UldCode::FQA => UldCategory::Pallet,
            UldCode::AKE | UldCode::DPE | UldCode::P6P | UldCode::LAY
            | UldCode::ALF | UldCode::AMA | UldCode::AMD => UldCategory::Container,
        }
    }
}
