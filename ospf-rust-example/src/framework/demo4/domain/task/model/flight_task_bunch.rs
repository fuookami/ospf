use time::{Duration, OffsetDateTime};
use super::aircraft::Aircraft;
use super::airport::Airport;

/// 飞行任务束 / Flight task bunch
/// 对齐 Kotlin FlightTaskBunch - 列生成的核心类型
#[derive(Debug, Clone)]
pub struct FlightTaskBunch {
    pub id: String,
    pub aircraft: Aircraft,
    pub tasks: Vec<String>, // task IDs
    pub cost: f64,
    pub dep: Airport,
    pub arr: Airport,
    pub start_time: OffsetDateTime,
    pub end_time: OffsetDateTime,
}

impl FlightTaskBunch {
    pub fn new(id: String, aircraft: Aircraft, dep: Airport, arr: Airport) -> Self {
        Self {
            id,
            aircraft,
            tasks: Vec::new(),
            cost: 0.0,
            dep,
            arr,
            start_time: OffsetDateTime::now_utc(),
            end_time: OffsetDateTime::now_utc(),
        }
    }

    pub fn contains(&self, task_id: &str) -> bool {
        self.tasks.contains(&task_id.to_string())
    }

    pub fn arrived_when(&self, airport: &Airport, time: &(OffsetDateTime, OffsetDateTime)) -> bool {
        &self.arr == airport && self.end_time >= time.0 && self.end_time <= time.1
    }

    pub fn departed_when(&self, airport: &Airport, time: &(OffsetDateTime, OffsetDateTime)) -> bool {
        &self.dep == airport && self.start_time >= time.0 && self.start_time <= time.1
    }
}
