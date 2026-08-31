//! 机场模型模块 / Airport model module

use std::collections::HashMap;
use time::Duration;
use crate::framework::demo4::infrastructure::Icao;

/// 机场类型 / Airport type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AirportType {
    /// 国内机场 / Domestic airport
    Domestic,
    /// 区域机场 / Regional airport
    Regional,
    /// 国际机场 / International airport
    International,
}

impl AirportType {
    /// 是否为国内机场 / Check if domestic airport
    pub fn is_domain_type(&self) -> bool {
        matches!(self, AirportType::Domestic)
    }
}

/// 机场 / Airport
#[derive(Debug, Clone)]
pub struct Airport {
    /// ICAO 代码 / ICAO code
    pub icao: Icao,
    /// 机场类型 / Airport type
    pub airport_type: AirportType,
    /// 旅客过站时间 / Passenger transfer time
    pub passenger_transfer_time: Duration,
    /// 货物过站时间 / Cargo transfer time
    pub cargo_transfer_time: Duration,
    /// 是否为基地机场 / Whether this is a base airport
    pub base: bool,
}

impl PartialEq for Airport {
    fn eq(&self, other: &Self) -> bool {
        self.icao == other.icao
    }
}
impl Eq for Airport {}

impl std::hash::Hash for Airport {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.icao.hash(state);
    }
}

/// 航线 / Route
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Route {
    /// 出发机场 / Departure airport
    pub dep: Airport,
    /// 到达机场 / Arrival airport
    pub arr: Airport,
}
