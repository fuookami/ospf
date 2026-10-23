//! 无损飞行任务束 / Lossless flight task bunch.

use std::collections::HashSet;
use time::{Duration, OffsetDateTime};
use super::aircraft::Aircraft;
use super::airport::Airport;
use super::flight_cycle::{FlightCycle, FlightHour};
use super::recovery::{FlightTaskRecovery, RecoveryError};

/// 单架飞机的不可变恢复序列，端点及任务 ID 均从活动派生。 / Immutable recovery sequence with derived endpoints and task IDs.
#[derive(Debug, Clone)]
pub struct FlightTaskBunch {
    id: String,
    aircraft: Aircraft,
    activities: Vec<FlightTaskRecovery>,
}

impl FlightTaskBunch {
    /// 校验唯一任务、执行飞机和连续时空记录；空束无虚构时间或机场。 / Validate unique tasks, aircraft and continuity; empty bunches have no invented endpoints.
    pub fn new(
        id: String,
        aircraft: Aircraft,
        activities: Vec<FlightTaskRecovery>,
    ) -> Result<Self, RecoveryError> {
        let mut ids = HashSet::new();
        for activity in &activities {
            if !ids.insert(activity.original().plan.id.as_str()) {
                return Err(RecoveryError::DuplicateTask(activity.original().plan.id.clone()));
            }
            if !activity.aircraft().same_configuration(&aircraft) {
                return Err(RecoveryError::AircraftMismatch(activity.original().plan.id.clone()));
            }
        }
        for pair in activities.windows(2) {
            if pair[0].route().arr != pair[1].route().dep
                || pair[0].time().1 > pair[1].time().0
            {
                return Err(RecoveryError::Disconnected(pair[1].original().plan.id.clone()));
            }
        }
        Ok(Self { id, aircraft, activities })
    }

    /// 束标识 / Bunch identifier.
    pub fn id(&self) -> &str { &self.id }

    /// 实际执行飞机 / Actual executor aircraft.
    pub fn aircraft(&self) -> &Aircraft { &self.aircraft }

    /// 无损恢复活动 / Lossless recovery activities.
    pub fn activities(&self) -> &[FlightTaskRecovery] { &self.activities }

    /// 派生的任务 ID 视图 / Derived task ID view.
    pub fn task_ids(&self) -> impl Iterator<Item = &str> {
        self.activities.iter().map(|activity| activity.original().plan.id.as_str())
    }

    /// 按原始任务 ID 查询恢复记录 / Find the recovery record by original task ID.
    pub fn recovery(&self, task_id: &str) -> Option<&FlightTaskRecovery> {
        self.activities.iter().find(|activity| activity.original().plan.id == task_id)
    }

    /// 是否覆盖原任务 / Whether the original task is covered.
    pub fn contains(&self, task_id: &str) -> bool { self.recovery(task_id).is_some() }

    /// 两任务是否连续 / Whether two tasks are consecutive.
    pub fn consecutive(&self, from: &str, to: &str) -> bool {
        self.activities.windows(2).any(|pair| {
            pair[0].original().plan.id == from && pair[1].original().plan.id == to
        })
    }

    /// 恢复后的出发机场 / Recovered departure airport.
    pub fn dep(&self) -> Option<&Airport> {
        self.activities.first().map(|activity| &activity.route().dep)
    }

    /// 恢复后的到达机场 / Recovered arrival airport.
    pub fn arr(&self) -> Option<&Airport> {
        self.activities.last().map(|activity| &activity.route().arr)
    }

    /// 恢复后的完整时间范围 / Complete recovered time range.
    pub fn time(&self) -> Option<(OffsetDateTime, OffsetDateTime)> {
        self.activities.first().zip(self.activities.last())
            .map(|(first, last)| (first.time().0, last.time().1))
    }

    /// 查询全部活动的到达事件，包含内部中转机场；窗口为闭区间。 / Query all arrival events including internal airports; bounds are inclusive.
    pub fn arrived_when(&self, airport: &Airport, time: &(OffsetDateTime, OffsetDateTime)) -> bool {
        self.activities.iter().any(|activity| {
            &activity.route().arr == airport && time.0 <= activity.time().1 && activity.time().1 <= time.1
        })
    }

    /// 查询全部活动的出发事件，包含内部中转机场；窗口为闭区间。 / Query all departure events including internal airports; bounds are inclusive.
    pub fn departed_when(&self, airport: &Airport, time: &(OffsetDateTime, OffsetDateTime)) -> bool {
        self.activities.iter().any(|activity| {
            &activity.route().dep == airport && time.0 <= activity.time().0 && activity.time().0 <= time.1
        })
    }

    /// 飞行活动的实际飞行时长，不含地面等待。 / Actual duration of flight activities, excluding ground waiting.
    pub fn flight_hour(&self) -> FlightHour {
        FlightHour::new(self.activities.iter().filter(|activity| activity.is_flight())
            .map(|activity| activity.time().1 - activity.time().0)
            .fold(Duration::ZERO, |sum, duration| sum + duration))
    }

    /// 实际飞行活动的循环数 / Number of actual flight activity cycles.
    pub fn flight_cycle(&self) -> FlightCycle {
        FlightCycle::new(self.activities.iter().filter(|activity| activity.is_flight()).count() as u64)
    }
}

#[cfg(test)]
mod tests;
