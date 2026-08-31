//! 维护任务模块 / Maintenance task module

use time::{Duration, OffsetDateTime};
use super::aircraft::Aircraft;
use super::airport::Airport;
use super::flight_task::{FlightTaskAssignment, FlightTaskStatus};

/// 维护类别 / Maintenance category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaintenanceCategory {
    /// A 检 / A-check
    ACheck,
    /// B 检 / B-check
    BCheck,
    /// C 检 / C-check
    CCheck,
    /// D 检 / D-check
    DCheck,
}

/// 维护计划 / Maintenance plan
#[derive(Debug, Clone)]
pub struct MaintenancePlan {
    /// 计划标识 / Plan identifier
    pub id: String,
    /// 计划名称 / Plan name
    pub name: String,
    /// 维护类别 / Maintenance category
    pub category: MaintenanceCategory,
    /// 维护飞机 / Aircraft under maintenance
    pub aircraft: Aircraft,
    /// 维护所在机场 / Airport where maintenance is performed
    pub dep: Airport,
    /// 维护时间段 / Maintenance time range
    pub time: (OffsetDateTime, OffsetDateTime),
    /// 飞行任务状态约束 / Flight task status constraints
    pub flight_task_status: Vec<FlightTaskStatus>,
}

/// 维护任务 / Maintenance task
#[derive(Debug, Clone)]
pub struct Maintenance {
    /// 维护计划 / Maintenance plan
    pub plan: MaintenancePlan,
    /// 恢复用替代飞机 / Recovery replacement aircraft
    pub recovery_aircraft: Option<Aircraft>,
    /// 恢复时间段 / Recovery time range
    pub recovery_time: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl Maintenance {
    /// 创建新的维护任务 / Create a new maintenance task
    pub fn new(plan: MaintenancePlan) -> Self {
        Self { plan, recovery_aircraft: None, recovery_time: None }
    }

    /// 获取当前执行飞机（恢复或原计划） / Get current aircraft (recovery or planned)
    pub fn aircraft(&self) -> &Aircraft {
        self.recovery_aircraft.as_ref().unwrap_or(&self.plan.aircraft)
    }

    /// 是否已恢复 / Check if recovered
    pub fn recovered(&self) -> bool {
        self.recovery_aircraft.is_some() || self.recovery_time.is_some()
    }
}
