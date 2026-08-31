//! 飞行任务束模块 / Flight task bunch module

use super::aircraft::Aircraft;
use super::airport::Airport;
use time::{Duration, OffsetDateTime};

/// 飞行任务束 / Flight task bunch
/// 对齐 Kotlin FlightTaskBunch - 列生成的核心类型
#[derive(Debug, Clone)]
pub struct FlightTaskBunch {
    /// 任务束标识 / Bunch identifier
    pub id: String,
    /// 执行飞机 / Executor aircraft
    pub aircraft: Aircraft,
    /// 包含的任务标识列表 / List of task IDs contained
    pub tasks: Vec<String>, // task IDs
    /// 成本 / Cost
    pub cost: f64,
    /// 出发机场 / Departure airport
    pub dep: Airport,
    /// 到达机场 / Arrival airport
    pub arr: Airport,
    /// 开始时间 / Start time
    pub start_time: OffsetDateTime,
    /// 结束时间 / End time
    pub end_time: OffsetDateTime,
}

impl FlightTaskBunch {
    /// 创建新的飞行任务束 / Create a new flight task bunch
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

    /// 是否包含指定任务 / Check if the bunch contains a given task
    pub fn contains(&self, task_id: &str) -> bool {
        self.tasks.contains(&task_id.to_string())
    }

    /// 是否在指定时间到达某机场 / Check if arrived at the given airport within the time range
    pub fn arrived_when(&self, airport: &Airport, time: &(OffsetDateTime, OffsetDateTime)) -> bool {
        &self.arr == airport && self.end_time >= time.0 && self.end_time <= time.1
    }

    /// 是否在指定时间从某机场出发 / Check if departed from the given airport within the time range
    pub fn departed_when(
        &self,
        airport: &Airport,
        time: &(OffsetDateTime, OffsetDateTime),
    ) -> bool {
        &self.dep == airport && self.start_time >= time.0 && self.start_time <= time.1
    }
}
