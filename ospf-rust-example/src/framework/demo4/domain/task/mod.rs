pub mod model;

use model::*;
use time::Duration;

/// 任务领域聚合 / Task domain aggregation
/// 对齐 Kotlin task Aggregation
#[derive(Debug)]
pub struct Aggregation {
    pub airports: Vec<Airport>,
    pub aircrafts: Vec<Aircraft>,
    pub aircraft_usability: Vec<(Aircraft, AircraftUsability)>,
    pub legs: Vec<FlightLeg>,
    pub maintenances: Vec<Maintenance>,
    pub aogs: Vec<Aog>,
    pub transfer_flights: Vec<Transfer>,
    pub origin_bunches: Vec<FlightTaskBunch>,
}

impl Aggregation {
    pub fn new(
        airports: Vec<Airport>,
        aircrafts: Vec<Aircraft>,
        aircraft_usability: Vec<(Aircraft, AircraftUsability)>,
        legs: Vec<FlightLeg>,
        maintenances: Vec<Maintenance>,
        aogs: Vec<Aog>,
        transfer_flights: Vec<Transfer>,
        origin_bunches: Vec<FlightTaskBunch>,
    ) -> Self {
        Self {
            airports,
            aircrafts,
            aircraft_usability,
            legs,
            maintenances,
            aogs,
            transfer_flights,
            origin_bunches,
        }
    }

    /// 获取所有飞行任务 / Get all flight tasks
    pub fn flight_tasks(&self) -> Vec<&str> {
        let mut tasks: Vec<&str> = Vec::new();
        for leg in &self.legs {
            tasks.push(&leg.plan.actual_id);
        }
        for m in &self.maintenances {
            tasks.push(&m.plan.id);
        }
        for aog in &self.aogs {
            tasks.push(&aog.plan.id);
        }
        for t in &self.transfer_flights {
            tasks.push(&t.plan.id);
        }
        tasks
    }

    pub fn enabled(&self, aircraft: &Aircraft, time: &(time::OffsetDateTime, time::OffsetDateTime)) -> bool {
        self.aircraft_usability
            .iter()
            .find(|(a, _)| a == aircraft)
            .map(|(_, usability)| usability.enabled_time <= time.1)
            .unwrap_or(false)
    }
}

/// 飞行任务上下文 / Flight task context
/// 对齐 Kotlin FlightTaskContext
#[derive(Debug)]
pub struct FlightTaskContext {
    pub aggregation: Option<Aggregation>,
}

impl FlightTaskContext {
    pub fn new() -> Self {
        Self { aggregation: None }
    }
}
