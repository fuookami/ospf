//! 航段模型模块 / Flight leg model module

use time::{Duration, OffsetDateTime};
use crate::framework::demo4::infrastructure::AircraftRegisterNumber;
use super::aircraft::Aircraft;
use super::airport::Airport;
use super::flight_task::{FlightTaskAssignment, FlightTaskStatus, FlightTaskType, FlightTaskCategory};

/// 航班计划 / Flight leg plan
#[derive(Debug, Clone)]
pub struct FlightLegPlan {
    /// 实际标识 / Actual identifier
    pub actual_id: String,
    /// 航班号 / Flight number
    pub no: String,
    /// 航班类型 / Flight type
    pub flight_type: super::flight_type::FlightType,
    /// 计划执行飞机 / Planned aircraft
    pub aircraft: Aircraft,
    /// 可用飞机列表 / Enabled aircraft list
    pub enabled_aircrafts: Vec<Aircraft>,
    /// 出发机场 / Departure airport
    pub dep: Airport,
    /// 到达机场 / Arrival airport
    pub arr: Airport,
    /// 计划时间 / Scheduled time range
    pub scheduled_time: (OffsetDateTime, OffsetDateTime),
    /// 预估时间 / Estimated time range
    pub estimated_time: Option<(OffsetDateTime, OffsetDateTime)>,
    /// 实际时间 / Actual time range
    pub actual_time: Option<(OffsetDateTime, OffsetDateTime)>,
    /// 撤轮挡时间 / Out time (wheels off chocks)
    pub out_time: Option<OffsetDateTime>,
    /// 飞行任务状态约束 / Flight task status constraints
    pub flight_task_status: Vec<FlightTaskStatus>,
    /// 权重 / Weight
    pub weight: f64,
}

impl FlightLegPlan {
    /// 获取计划标识 / Get plan identifier
    pub fn id(&self) -> String {
        format!("f_{}", self.actual_id)
    }

    /// 获取计划名称 / Get plan name
    pub fn name(&self) -> String {
        format!("{}_{}", self.no, self.actual_id)
    }

    /// 获取有效时间（实际或预估） / Get effective time (actual or estimated)
    pub fn time(&self) -> Option<(OffsetDateTime, OffsetDateTime)> {
        self.actual_time.or(self.estimated_time)
    }

    /// 是否允许恢复调整 / Check if recovery adjustments are enabled
    pub fn recovery_enabled(&self) -> bool {
        self.actual_time.is_none() && self.out_time.is_none()
    }
}

/// 航班 / Flight leg
#[derive(Debug, Clone)]
pub struct FlightLeg {
    /// 航班计划 / Flight leg plan
    pub plan: FlightLegPlan,
    /// 恢复用替代飞机 / Recovery replacement aircraft
    pub recovery_aircraft: Option<Aircraft>,
    /// 恢复时间段 / Recovery time range
    pub recovery_time: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl FlightLeg {
    /// 创建新的航段 / Create a new flight leg
    pub fn new(plan: FlightLegPlan) -> Self {
        Self {
            plan,
            recovery_aircraft: None,
            recovery_time: None,
        }
    }

    /// 获取当前执行飞机（恢复或原计划） / Get current aircraft (recovery or planned)
    pub fn aircraft(&self) -> &Aircraft {
        self.recovery_aircraft.as_ref().unwrap_or(&self.plan.aircraft)
    }

    /// 获取有效时间（恢复或计划） / Get effective time (recovery or planned)
    pub fn time(&self) -> Option<(OffsetDateTime, OffsetDateTime)> {
        self.recovery_time.or(self.plan.time())
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
