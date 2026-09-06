//! 航班类型模块 / Flight type module

use super::airport::AirportType;

/// 航班类型 / Flight type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FlightType {
    /// 国内航班 / Domestic flight
    Domestic,
    /// 区域航班 / Regional flight
    Regional,
    /// 国际航班 / International flight
    International,
}

impl FlightType {
    /// 是否为国内航班 / Check if domestic flight
    pub fn is_domain_type(&self) -> bool {
        matches!(self, FlightType::Domestic)
    }

    /// 根据出发和到达机场类型推断航班类型 / Infer flight type from departure and arrival airport types
    pub fn from_airport_types(dep: AirportType, arr: AirportType) -> Self {
        match dep.max(arr) {
            AirportType::Domestic => FlightType::Domestic,
            AirportType::Regional => FlightType::Regional,
            AirportType::International => FlightType::International,
        }
    }
}
