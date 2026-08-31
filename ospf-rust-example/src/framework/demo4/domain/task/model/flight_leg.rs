use time::{Duration, OffsetDateTime};
use crate::framework::demo4::infrastructure::AircraftRegisterNumber;
use super::aircraft::Aircraft;
use super::airport::Airport;
use super::flight_task::{FlightTaskAssignment, FlightTaskStatus, FlightTaskType, FlightTaskCategory};

/// 航班计划 / Flight leg plan
#[derive(Debug, Clone)]
pub struct FlightLegPlan {
    pub actual_id: String,
    pub no: String,
    pub flight_type: super::flight_type::FlightType,
    pub aircraft: Aircraft,
    pub enabled_aircrafts: Vec<Aircraft>,
    pub dep: Airport,
    pub arr: Airport,
    pub scheduled_time: (OffsetDateTime, OffsetDateTime),
    pub estimated_time: Option<(OffsetDateTime, OffsetDateTime)>,
    pub actual_time: Option<(OffsetDateTime, OffsetDateTime)>,
    pub out_time: Option<OffsetDateTime>,
    pub flight_task_status: Vec<FlightTaskStatus>,
    pub weight: f64,
}

impl FlightLegPlan {
    pub fn id(&self) -> String {
        format!("f_{}", self.actual_id)
    }

    pub fn name(&self) -> String {
        format!("{}_{}", self.no, self.actual_id)
    }

    pub fn time(&self) -> Option<(OffsetDateTime, OffsetDateTime)> {
        self.actual_time.or(self.estimated_time)
    }

    pub fn recovery_enabled(&self) -> bool {
        self.actual_time.is_none() && self.out_time.is_none()
    }
}

/// 航班 / Flight leg
#[derive(Debug, Clone)]
pub struct FlightLeg {
    pub plan: FlightLegPlan,
    pub recovery_aircraft: Option<Aircraft>,
    pub recovery_time: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl FlightLeg {
    pub fn new(plan: FlightLegPlan) -> Self {
        Self {
            plan,
            recovery_aircraft: None,
            recovery_time: None,
        }
    }

    pub fn aircraft(&self) -> &Aircraft {
        self.recovery_aircraft.as_ref().unwrap_or(&self.plan.aircraft)
    }

    pub fn time(&self) -> Option<(OffsetDateTime, OffsetDateTime)> {
        self.recovery_time.or(self.plan.time())
    }

    pub fn dep(&self) -> &Airport {
        &self.plan.dep
    }

    pub fn arr(&self) -> &Airport {
        &self.plan.arr
    }

    pub fn recovered(&self) -> bool {
        self.recovery_aircraft.is_some() || self.recovery_time.is_some()
    }
}
