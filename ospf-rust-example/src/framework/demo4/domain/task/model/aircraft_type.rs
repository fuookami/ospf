use std::collections::HashMap;
use time::Duration;
use crate::framework::demo4::infrastructure::{AircraftTypeCode, AircraftMinorTypeCode};
use super::airport::{Airport, Route};

/// 飞机类型 / Aircraft type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AircraftType {
    pub code: AircraftTypeCode,
}

/// 飞机子类型 / Aircraft minor type
#[derive(Debug, Clone)]
pub struct AircraftMinorType {
    pub aircraft_type: AircraftType,
    pub code: AircraftMinorTypeCode,
    pub cost_per_hour: f64,
    pub route_fly_time: HashMap<Route, Duration>,
    pub connection_time: HashMap<Airport, Duration>,
    pub max_fly_time: Option<Duration>,
}

impl AircraftMinorType {
    pub fn max_route_fly_time(&self) -> Duration {
        self.route_fly_time.values().copied().max().unwrap_or(Duration::ZERO)
    }

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
