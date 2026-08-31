//! 飞机类型模块 / Aircraft type module

use std::collections::HashMap;
use time::Duration;
use crate::framework::demo4::infrastructure::{AircraftTypeCode, AircraftMinorTypeCode};
use super::airport::{Airport, Route};

/// 飞机类型 / Aircraft type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AircraftType {
    /// 机型代码 / Aircraft type code
    pub code: AircraftTypeCode,
}

/// 飞机子类型 / Aircraft minor type
#[derive(Debug, Clone)]
pub struct AircraftMinorType {
    /// 所属飞机类型 / Parent aircraft type
    pub aircraft_type: AircraftType,
    /// 子类型代码 / Minor type code
    pub code: AircraftMinorTypeCode,
    /// 每小时成本 / Cost per flight hour
    pub cost_per_hour: f64,
    /// 各航线飞行时间 / Flight time per route
    pub route_fly_time: HashMap<Route, Duration>,
    /// 各机场过站时间 / Connection time per airport
    pub connection_time: HashMap<Airport, Duration>,
    /// 最大飞行时间 / Maximum flight time
    pub max_fly_time: Option<Duration>,
}

impl AircraftMinorType {
    /// 获取最长航线飞行时间 / Get maximum route flight time
    pub fn max_route_fly_time(&self) -> Duration {
        self.route_fly_time.values().copied().max().unwrap_or(Duration::ZERO)
    }

    /// 获取最长过站时间 / Get maximum connection time
    pub fn max_connection_time(&self) -> Duration {
        self.connection_time.values().copied().max().unwrap_or(Duration::ZERO)
    }
}

impl PartialEq for AircraftMinorType {
    fn eq(&self, other: &Self) -> bool {
        self.aircraft_type == other.aircraft_type && self.code == other.code
    }
}
impl Eq for AircraftMinorType {}

impl std::hash::Hash for AircraftMinorType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.aircraft_type.hash(state);
        self.code.hash(state);
    }
}
