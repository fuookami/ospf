use std::collections::HashMap;
use time::Duration;
use crate::framework::demo4::infrastructure::Icao;

/// 机场类型 / Airport type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AirportType {
    Domestic,
    Regional,
    International,
}

impl AirportType {
    pub fn is_domain_type(&self) -> bool {
        matches!(self, AirportType::Domestic)
    }
}

/// 机场 / Airport
#[derive(Debug, Clone)]
pub struct Airport {
    pub icao: Icao,
    pub airport_type: AirportType,
    pub passenger_transfer_time: Duration,
    pub cargo_transfer_time: Duration,
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
    pub dep: Airport,
    pub arr: Airport,
}
