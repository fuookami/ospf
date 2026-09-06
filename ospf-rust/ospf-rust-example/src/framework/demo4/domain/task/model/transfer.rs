//! 调机任务模块 / Transfer flight module

use super::aircraft::Aircraft;
use super::airport::Airport;
use super::flight_task::{FlightTaskAssignment, FlightTaskStatus};
use time::{Duration, OffsetDateTime};

/// 调机计划 / Transfer plan
#[derive(Debug, Clone)]
pub struct TransferPlan {
    /// 计划标识 / Plan identifier
    pub id: String,
    /// 计划名称 / Plan name
    pub name: String,
    /// 执行飞机 / Executor aircraft
    pub aircraft: Aircraft,
    /// 出发机场 / Departure airport
    pub dep: Airport,
    /// 到达机场 / Arrival airport
    pub arr: Airport,
    /// 调机时间段 / Transfer time range
    pub time: (OffsetDateTime, OffsetDateTime),
    /// 飞行任务状态约束 / Flight task status constraints
    pub flight_task_status: Vec<FlightTaskStatus>,
}

/// 调机任务 / Transfer flight
#[derive(Debug, Clone)]
pub struct Transfer {
    /// 调机计划 / Transfer plan
    pub plan: TransferPlan,
    /// 恢复用替代飞机 / Recovery replacement aircraft
    pub recovery_aircraft: Option<Aircraft>,
    /// 恢复时间段 / Recovery time range
    pub recovery_time: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl Transfer {
    /// 创建新的调机任务 / Create a new transfer flight
    pub fn new(plan: TransferPlan) -> Self {
        Self {
            plan,
            recovery_aircraft: None,
            recovery_time: None,
        }
    }

    /// 获取当前执行飞机（恢复或原计划） / Get current aircraft (recovery or planned)
    pub fn aircraft(&self) -> &Aircraft {
        self.recovery_aircraft
            .as_ref()
            .unwrap_or(&self.plan.aircraft)
    }

    /// 获取出发机场 / Get departure airport
    pub fn dep(&self) -> &Airport {
        &self.plan.dep
    }

    /// 获取到达机场 / Get arrival airport
    pub fn arr(&self) -> &Airport {
        &self.plan.arr
    }

    /// 是否已恢复 / Check if recovered
    pub fn recovered(&self) -> bool {
        self.recovery_aircraft.is_some() || self.recovery_time.is_some()
    }
}
