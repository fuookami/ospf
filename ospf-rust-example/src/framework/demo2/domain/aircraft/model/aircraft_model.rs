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
