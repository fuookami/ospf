//! 恢复模型模块 / Recovery model module

use std::error::Error;
use std::fmt::{Display, Formatter};
use time::OffsetDateTime;
use super::aircraft::Aircraft;
use super::airport::Route;
use super::flight_task::FlightTaskImpl;

/// 恢复记录的结构错误，不表示优化模型不可行。 / Structural recovery errors, distinct from model infeasibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryError {
    /// 任务时间非正 / Nonpositive activity time.
    InvalidTime(String),
    /// 重复任务 / Duplicate task.
    DuplicateTask(String),
    /// 执行飞机不一致 / Inconsistent executor aircraft.
    AircraftMismatch(String),
    /// 时空序列不连续 / Discontinuous spacetime sequence.
    Disconnected(String),
    /// 非有限成本或运算结果 / Nonfinite cost or arithmetic result.
    InvalidCost,
    /// 非法规则定义 / Invalid rule definition.
    InvalidRule(String),
}

impl Display for RecoveryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid flight recovery: {self:?}")
    }
}

impl Error for RecoveryError {}

/// 保留原任务及实际飞机、航线和逐活动时间的不可变分配记录。 / Immutable assignment preserving the original task and actual aircraft, route and activity time.
#[derive(Debug, Clone)]
pub struct FlightTaskRecovery {
    original: FlightTaskImpl,
    aircraft: Aircraft,
    route: Route,
    time: (OffsetDateTime, OffsetDateTime),
}

impl FlightTaskRecovery {
    /// 创建记录；不改写原任务中的计划和恢复字段。 / Create a record without rewriting the original task's planned or recovery fields.
    pub fn new(
        original: FlightTaskImpl,
        aircraft: Aircraft,
        route: Route,
        time: (OffsetDateTime, OffsetDateTime),
    ) -> Result<Self, RecoveryError> {
        if time.0 >= time.1 {
            return Err(RecoveryError::InvalidTime(original.plan.id.clone()));
        }
        Ok(Self { original, aircraft, route, time })
    }

    /// 未改写的原任务 / Unmodified original task.
    pub fn original(&self) -> &FlightTaskImpl { &self.original }

    /// 实际执行飞机 / Actual executor aircraft.
    pub fn aircraft(&self) -> &Aircraft { &self.aircraft }

    /// 实际恢复航线 / Actual recovery route.
    pub fn route(&self) -> &Route { &self.route }

    /// 实际活动开始和结束时间 / Actual activity start and end times.
    pub fn time(&self) -> (OffsetDateTime, OffsetDateTime) { self.time }

    /// 是否消耗飞行小时和循环 / Whether flight hours and cycles are consumed.
    pub fn is_flight(&self) -> bool { self.original.task_type.category.is_flight_type() }
}

/// 恢复航班任务键 / Recovery flight task key (对齐 Kotlin RecoveryFlightTaskKey)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecoveryFlightTaskKey {
    /// 任务标识 / Task identifier
    pub task_id: String,
    /// 恢复类型 / Recovery type
    pub recovery_type: RecoveryType,
}

/// 恢复类型 / Recovery type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoveryType {
    /// 换飞机 / Aircraft change
    AircraftChange,
    /// 时间调整 / Time change
    TimeChange,
    /// 航线调整 / Route change
    RouteChange,
    /// 取消 / Cancel
    Cancel,
}
