use time::{Duration, OffsetDateTime};
use super::aircraft::Aircraft;
use super::airport::Airport;
use super::flight_task::{FlightTaskAssignment, FlightTaskStatus};

/// 调机计划 / Transfer plan
#[derive(Debug, Clone)]
pub struct TransferPlan {
    pub id: String,
    pub name: String,
    pub aircraft: Aircraft,
    pub dep: Airport,
    pub arr: Airport,
    pub time: (OffsetDateTime, OffsetDateTime),
    pub flight_task_status: Vec<FlightTaskStatus>,
}

/// 调机任务 / Transfer flight
#[derive(Debug, Clone)]
pub struct Transfer {
    pub plan: TransferPlan,
    pub recovery_aircraft: Option<Aircraft>,
    pub recovery_time: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl Transfer {
    pub fn new(plan: TransferPlan) -> Self {
        Self { plan, recovery_aircraft: None, recovery_time: None }
    }

    pub fn aircraft(&self) -> &Aircraft {
        self.recovery_aircraft.as_ref().unwrap_or(&self.plan.aircraft)
    }

    pub fn dep(&self) -> &Airport { &self.plan.dep }
    pub fn arr(&self) -> &Airport { &self.plan.arr }

    pub fn recovered(&self) -> bool {
        self.recovery_aircraft.is_some() || self.recovery_time.is_some()
    }
}
