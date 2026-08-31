use time::{Duration, OffsetDateTime};
use super::aircraft::Aircraft;
use super::airport::Airport;
use super::flight_task::{FlightTaskAssignment, FlightTaskStatus};

/// AOG 计划 / AOG plan
#[derive(Debug, Clone)]
pub struct AogPlan {
    pub id: String,
    pub name: String,
    pub aircraft: Aircraft,
    pub dep: Airport,
    pub time: (OffsetDateTime, OffsetDateTime),
    pub flight_task_status: Vec<FlightTaskStatus>,
}

/// AOG (飞机停场) / Aircraft on Ground
#[derive(Debug, Clone)]
pub struct Aog {
    pub plan: AogPlan,
    pub recovery_aircraft: Option<Aircraft>,
    pub recovery_time: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl Aog {
    pub fn new(plan: AogPlan) -> Self {
        Self { plan, recovery_aircraft: None, recovery_time: None }
    }

    pub fn aircraft(&self) -> &Aircraft {
        self.recovery_aircraft.as_ref().unwrap_or(&self.plan.aircraft)
    }

    pub fn recovered(&self) -> bool {
        self.recovery_aircraft.is_some() || self.recovery_time.is_some()
    }
}
