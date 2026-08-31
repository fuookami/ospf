use time::{Duration, OffsetDateTime};
use ospf_rust_framework_gantt_scheduling::domain::task::ExecutorTrait;
use crate::framework::demo4::infrastructure::{AircraftRegisterNumber, PassengerClass};
use super::aircraft_type::AircraftMinorType;
use super::airport::Airport;
use super::flight_cycle::FlightCyclePeriod;

/// 飞机容量 / Aircraft capacity
#[derive(Debug, Clone)]
pub enum AircraftCapacity {
    Passenger { capacities: Vec<(PassengerClass, u64)> },
    Cargo { max_weight: f64 },
}

/// 飞机 / Aircraft
#[derive(Debug, Clone)]
pub struct Aircraft {
    pub reg_no: AircraftRegisterNumber,
    pub minor_type: AircraftMinorType,
    pub capacity: AircraftCapacity,
}

impl Aircraft {
    pub fn type_code(&self) -> &super::aircraft_type::AircraftType {
        &self.minor_type.aircraft_type
    }

    pub fn cost_per_hour(&self) -> f64 {
        self.minor_type.cost_per_hour
    }
}

impl PartialEq for Aircraft {
    fn eq(&self, other: &Self) -> bool {
        self.reg_no == other.reg_no
    }
}
impl Eq for Aircraft {}

impl std::hash::Hash for Aircraft {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.reg_no.hash(state);
    }
}

/// 实现 ExecutorTrait for Aircraft
impl ExecutorTrait for Aircraft {
    fn id(&self) -> &str {
        &self.reg_no.0
    }

    fn name(&self) -> &str {
        &self.reg_no.0
    }
}

/// 飞机可用性 / Aircraft usability
#[derive(Debug, Clone)]
pub struct AircraftUsability {
    pub location: Airport,
    pub enabled_time: OffsetDateTime,
    pub flight_cycle_periods: Vec<FlightCyclePeriod>,
}
