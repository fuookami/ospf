//! AOG（飞机停场）模型模块 / AOG (Aircraft on Ground) model module

use super::aircraft::Aircraft;
use super::airport::Airport;
use super::flight_task::{FlightTaskAssignment, FlightTaskStatus};
use time::{Duration, OffsetDateTime};

/// AOG 计划 / AOG plan
#[derive(Debug, Clone)]
pub struct AogPlan {
    /// 计划标识 / Plan identifier
    pub id: String,
    /// 计划名称 / Plan name
    pub name: String,
    /// 关联飞机 / Associated aircraft
    pub aircraft: Aircraft,
    /// 所在机场 / Located airport
    pub dep: Airport,
    /// 停场时间段 / Grounded time range
    pub time: (OffsetDateTime, OffsetDateTime),
    /// 飞行任务状态约束 / Flight task status constraints
    pub flight_task_status: Vec<FlightTaskStatus>,
}

/// AOG (飞机停场) / Aircraft on Ground
#[derive(Debug, Clone)]
pub struct Aog {
    /// AOG 计划 / AOG plan
    pub plan: AogPlan,
    /// 恢复用替代飞机 / Recovery replacement aircraft
    pub recovery_aircraft: Option<Aircraft>,
    /// 恢复时间段 / Recovery time range
    pub recovery_time: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl Aog {
    /// 创建新的 AOG / Create a new AOG
    pub fn new(plan: AogPlan) -> Self {
        Self {
            plan,
            recovery_aircraft: None,
            recovery_time: None,
        }
    }

    /// 获取当前执行飞机 / Get the current aircraft (recovery or original)
    pub fn aircraft(&self) -> &Aircraft {
        self.recovery_aircraft
            .as_ref()
            .unwrap_or(&self.plan.aircraft)
    }

    /// 是否已恢复 / Check if recovered
    pub fn recovered(&self) -> bool {
        self.recovery_aircraft.is_some() || self.recovery_time.is_some()
    }
}
