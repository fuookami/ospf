//! 任务领域模块 / Task domain module.
/// 任务模型模块 / Task model module
pub mod model;

use model::*;
use time::Duration;

/// 任务领域聚合 / Task domain aggregation
/// 对齐 Kotlin task Aggregation
#[derive(Debug)]
pub struct Aggregation {
    /// 机场列表 / List of airports
    pub airports: Vec<Airport>,
    /// 飞机列表 / List of aircraft
    pub aircrafts: Vec<Aircraft>,
    /// 飞机可用性列表 / List of aircraft usability pairs
    pub aircraft_usability: Vec<(Aircraft, AircraftUsability)>,
    /// 航段列表 / List of flight legs
    pub legs: Vec<FlightLeg>,
    /// 维护任务列表 / List of maintenance tasks
    pub maintenances: Vec<Maintenance>,
    /// AOG（飞机停场）列表 / List of AOG (Aircraft on Ground) entries
    pub aogs: Vec<Aog>,
    /// 调机航班列表 / List of transfer flights
    pub transfer_flights: Vec<Transfer>,
    /// 原始飞行任务束列表 / List of original flight task bunches
    pub origin_bunches: Vec<FlightTaskBunch>,
}

impl Aggregation {
    /// 创建新的聚合 / Create a new aggregation
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

    /// 判断飞机在指定时间段是否可用 / Check if an aircraft is enabled within the given time range
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
    /// 任务领域聚合 / Task domain aggregation
    pub aggregation: Option<Aggregation>,
}

impl FlightTaskContext {
    /// 创建新的飞行任务上下文 / Create a new flight task context
    pub fn new() -> Self {
        Self { aggregation: None }
    }
}
