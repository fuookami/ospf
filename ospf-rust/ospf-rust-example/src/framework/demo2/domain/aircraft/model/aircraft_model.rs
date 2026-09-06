//! 飞机型号定义 / Aircraft model definitions
use std::collections::HashMap;
/// 飞机类型枚举 / Aircraft type enum (对齐 Kotlin AircraftType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AircraftType {
    /// 波音 737 / Boeing 737
    B737,
    /// 波音 757 / Boeing 757
    B757,
    /// 波音 767 / Boeing 767
    B767,
    /// 波音 747 / Boeing 747
    B747,
}

impl AircraftType {
    /// 是否需要压舱物 / Whether ballast is needed
    pub fn ballast_needed(&self) -> bool {
        matches!(self, AircraftType::B737 | AircraftType::B757)
    }

    /// 主甲板舱门是否偏好空置 / Whether main deck door empty is preferred
    pub fn main_deck_door_empty_prefer(&self) -> bool {
        matches!(self, AircraftType::B747)
    }
}

/// 飞机子型号 / Aircraft minor model
#[derive(Debug, Clone)]
pub struct AircraftMinorModel {
    /// 子型号名称 / Minor model name
    pub name: String,
    /// 所属飞机类型 / Aircraft type
    pub aircraft_type: AircraftType,
}

/// 飞机型号 / Aircraft model (对齐 Kotlin AircraftModel)
#[derive(Debug, Clone)]
pub struct AircraftModel {
    /// 型号名称 / Model name
    pub name: String,
    /// 飞机类型 / Aircraft type
    pub aircraft_type: AircraftType,
    /// 子型号 / Minor model
    pub minor_model: AircraftMinorModel,
    /// 是否为宽体机 / Whether wide-body aircraft
    pub wide_body: bool,
}

impl AircraftModel {
    /// 计算重力 / Calculate gravity force
    pub fn gravity(&self, weight: f64) -> f64 {
        weight * 9.80665 // 标准重力加速度
    }
}
