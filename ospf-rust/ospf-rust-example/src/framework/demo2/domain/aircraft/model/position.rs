//! 舱位模型定义 / Position model definitions
use super::aircraft_model::AircraftModel;
use super::deck::DeckLocation;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;
use std::collections::HashMap;
/// 舱位位置标签 / Position location tag (对齐 Kotlin PositionLocationTag)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositionLocationTag {
    /// 主舱 / Main compartment
    Main,
    /// 下舱 / Lower compartment
    Low,
    /// 散货舱 / Bulk compartment
    Bulk,
    /// 机头 / Head
    Head,
    /// 机尾 / Tail
    Tail,
    /// 前部 / Forward
    Forward,
    /// 后部 / Aft
    Aft,
}

/// 舱位位置 / Position location (对齐 Kotlin PositionLocation)
#[derive(Debug, Clone)]
pub struct PositionLocation {
    /// 位置标签列表 / Location tag list
    pub tags: Vec<PositionLocationTag>,
}

impl PositionLocation {
    /// 是否为主舱位置 / Whether main compartment
    pub fn main(&self) -> bool {
        self.tags.contains(&PositionLocationTag::Main)
    }
    /// 是否为下舱位置 / Whether lower compartment
    pub fn low(&self) -> bool {
        self.tags.contains(&PositionLocationTag::Low)
    }
    /// 是否为散货舱位置 / Whether bulk compartment
    pub fn bulk(&self) -> bool {
        self.tags.contains(&PositionLocationTag::Bulk)
    }
    /// 是否为机头位置 / Whether head position
    pub fn head(&self) -> bool {
        self.tags.contains(&PositionLocationTag::Head)
    }
    /// 是否为机尾位置 / Whether tail position
    pub fn tail(&self) -> bool {
        self.tags.contains(&PositionLocationTag::Tail)
    }

    /// 推断甲板位置 / Infer deck location from tags
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
    /// 前向力臂 / Forward arm
    pub front_arm: Quantity<f64, Unit>,
    /// 后向力臂 / Aft arm
    pub back_arm: Quantity<f64, Unit>,
    /// 左侧力臂 / Left arm
    pub left_arm: Quantity<f64, Unit>,
    /// 右侧力臂 / Right arm
    pub right_arm: Quantity<f64, Unit>,
    /// ULD 代码到偏移量的映射 / ULD code to offset mapping
    pub offsets: HashMap<String, f64>,
}

impl PositionCoordinate {
    /// 计算纵向力臂（前后均值） / Calculate longitudinal arm (average of front and back)
    pub fn longitudinal_arm(&self) -> f64 {
        (self.front_arm.value + self.back_arm.value) / 2.0
    }

    /// 计算横向力臂（左右均值） / Calculate lateral arm (average of left and right)
    pub fn lateral_arm(&self) -> f64 {
        (self.left_arm.value + self.right_arm.value) / 2.0
    }

    /// 是否为横向位置 / Whether transverse position
    pub fn transverse(&self) -> bool {
        (self.left_arm.value - self.right_arm.value).abs() > 1e-6
    }
}

/// 舱位形状 / Position shape
#[derive(Debug, Clone)]
pub struct PositionShape {
    /// 宽度 / Width
    pub width: Quantity<f64, Unit>,
    /// 长度 / Length
    pub length: Quantity<f64, Unit>,
    /// 高度 / Height
    pub height: Quantity<f64, Unit>,
}

/// 舱位类型代码 / Position type code (对齐 Kotlin PositionTypeCode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositionTypeCode {
    /// 禁止空舱 / Empty forbidden
    EmptyForbidden,
    /// 不宜空舱 / Empty hated
    EmptyHated,
    /// 空舱禁放特定货物 / Empty item forbidden
    EmptyItemForbidden,
    /// 禁放易碎货物 / Crush item forbidden
    CrushItemForbidden,
    /// 禁放刚性货物 / Stiff cargo forbidden
    StiffCargoForbidden,
    /// AOG/MAT 指定舱位 / AOG/MAT appointed
    AOGMATAppointed,
    /// 普通散货指定舱位 / Normal bulk appointed
    NormalBulkAppointed,
}

/// 舱位类型 / Position type
#[derive(Debug, Clone)]
pub struct PositionType {
    /// 类型代码列表 / Type code list
    pub codes: Vec<PositionTypeCode>,
}

impl PositionType {
    /// 是否包含指定类型代码 / Whether contains the specified type code
    pub fn contains(&self, code: &PositionTypeCode) -> bool {
        self.codes.contains(code)
    }
}

/// 舱位 / Position (aircraft domain, 对齐 Kotlin aircraft Position)
#[derive(Debug, Clone)]
pub struct Position {
    /// 舱位标识 / Position identifier
    pub id: String,
    /// 空间名称 / Space name
    pub space_name: String,
    /// 字母空间名称 / Alpha space name
    pub alpha_space_name: String,
    /// 尺寸代码 / Size code
    pub size_code: String,
    /// 装载顺序号 / Loading order number
    pub loading_order: u32,
    /// 坐标 / Coordinate
    pub coordinate: PositionCoordinate,
    /// 形状 / Shape
    pub shape: PositionShape,
    /// 位置 / Location
    pub location: PositionLocation,
}

/// 舱位对 / Position pair
pub type PositionPair = (Position, Position);
