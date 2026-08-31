use time::{Duration, OffsetDateTime};
use super::aircraft::Aircraft;
use super::airport::Airport;
use super::flight_task::{FlightTaskAssignment, FlightTaskStatus};

/// 维护类别 / Maintenance category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaintenanceCategory {
    ACheck,
    BCheck,
    CCheck,
    DCheck,
}

/// 维护计划 / Maintenance plan
#[derive(Debug, Clone)]
pub struct MaintenancePlan {
    pub id: String,
    pub name: String,
    pub category: MaintenanceCategory,
    pub aircraft: Aircraft,
    pub dep: Airport,
    pub time: (OffsetDateTime, OffsetDateTime),
    pub flight_task_status: Vec<FlightTaskStatus>,
}

/// 维护任务 / Maintenance task
#[derive(Debug, Clone)]
pub struct Maintenance {
    pub plan: MaintenancePlan,
    pub recovery_aircraft: Option<Aircraft>,
    pub recovery_time: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl Maintenance {
    pub fn new(plan: MaintenancePlan) -> Self {
        Self { plan, recovery_aircraft: None, recovery_time: None }
    }

    pub fn aircraft(&self) -> &Aircraft {
        self.recovery_aircraft.as_ref().unwrap_or(&self.plan.aircraft)
    }

    pub fn recovered(&self) -> bool {
        self.recovery_aircraft.is_some() || self.recovery_time.is_some()
    }
}
