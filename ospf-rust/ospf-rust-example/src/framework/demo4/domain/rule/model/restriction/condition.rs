//! 通用限制共享完整分配视图，内置条件和扩展条件不持有模型状态。 / General restrictions share full assignment views; built-in and extension conditions hold no model state.

use std::collections::BTreeSet;
use std::sync::Arc;
use time::OffsetDateTime;
use crate::framework::demo4::domain::task::model::{Aircraft, Airport, FlightTaskImpl, FlightTaskRecovery, RecoveryError};
use super::RestrictionReferences;

/// 原任务或完整实际恢复的只读分配视图。 / Read-only assignment view for original tasks or complete actual recoveries.
#[derive(Debug, Clone, Copy)]
pub struct RestrictionAssignment<'a> {
    /// 不可变原任务。 / Immutable original task.
    pub task: &'a FlightTaskImpl,
    /// 当前飞机，原任务可能未分配。 / Current aircraft, possibly absent on an original task.
    pub aircraft: Option<&'a Aircraft>,
    /// 当前出发机场。 / Current departure airport.
    pub departure: &'a Airport,
    /// 当前到达机场。 / Current arrival airport.
    pub arrival: &'a Airport,
    /// 当前时间，原任务可能缺失。 / Current times, possibly absent on an original task.
    pub time: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl<'a> RestrictionAssignment<'a> {
    /// 使用原任务的有效飞机及时间，不虚构缺失时间。 / Use the task's effective aircraft and times without inventing absent times.
    pub fn from_task(task: &'a FlightTaskImpl) -> Self {
        Self { task, aircraft: task.aircraft(), departure: task.dep(), arrival: task.arr(), time: task.time() }
    }

    /// 所有条件消费同一完整实际恢复。 / All conditions consume the same complete actual recovery.
    pub fn from_recovery(recovery: &'a FlightTaskRecovery) -> Self {
        Self { task: recovery.original(), aircraft: Some(recovery.aircraft()), departure: &recovery.route().dep,
            arrival: &recovery.route().arr, time: Some(recovery.time()) }
    }
}

/// 不可变扩展条件；相同分配的结果须确定且无模型状态。 / Immutable extension condition; results must be deterministic and model-state-free for an assignment.
pub trait GeneralRestrictionPredicate: std::fmt::Debug + Send + Sync {
    /// 是否命中限制，true 表示适用的违反条件。 / Whether the restriction is hit; true denotes an applicable violation condition.
    fn matches(&self, assignment: &RestrictionAssignment<'_>) -> bool;
}

/// 基于实际机场、飞机和时间的命中条件。 / Hit conditions using actual airports, aircraft and times.
#[derive(Debug, Clone)]
pub enum GeneralRestrictionCondition {
    /// 原任务标识集合。 / Original task identities.
    Flights(BTreeSet<String>),
    /// 出发事件命中；时间范围闭区间。 / Departure event match, with an inclusive time window.
    Departure {
        /// 出发机场集合。 / Departure airport identities.
        airports: BTreeSet<String>,
        /// 可选实际出发时间范围。 / Optional actual departure-time window.
        time: Option<(OffsetDateTime, OffsetDateTime)>,
    },
    /// 到达事件命中；时间范围闭区间。 / Arrival event match, with an inclusive time window.
    Arrival {
        /// 到达机场集合。 / Arrival airport identities.
        airports: BTreeSet<String>,
        /// 可选实际到达时间范围。 / Optional actual arrival-time window.
        time: Option<(OffsetDateTime, OffsetDateTime)>,
    },
    /// 双向机场对，指定时间时要求完整活动落在范围内。 / Bidirectional airport pair, requiring full containment when timed.
    Bidirectional {
        /// 第一机场集合。 / First airport set.
        first: BTreeSet<String>,
        /// 第二机场集合。 / Second airport set.
        second: BTreeSet<String>,
        /// 可选完整活动时间范围。 / Optional full-activity time window.
        time: Option<(OffsetDateTime, OffsetDateTime)>,
    },
    /// 实际飞机在集合中。 / Actual aircraft belongs to the set.
    EnabledAircraft(BTreeSet<String>),
    /// 实际飞机不在集合中。 / Actual aircraft does not belong to the set.
    DisabledAircraft(BTreeSet<String>),
    /// 全部条件同时命中，空集合恒真。 / All conditions match; empty conjunction is true.
    All(Vec<Self>),
    /// 自定义只读领域条件。 / Custom read-only domain condition.
    Custom(Arc<dyn GeneralRestrictionPredicate>),
}

impl GeneralRestrictionCondition {
    /// 缺少被要求的时间时不命中，不回退到原始估计时间。 / Do not match when required times are absent, without falling back to original estimates.
    pub fn matches(&self, assignment: &RestrictionAssignment<'_>) -> bool {
        let contained = |window: &Option<(OffsetDateTime, OffsetDateTime)>, event: Option<OffsetDateTime>| {
            window.is_none_or(|(start, end)| event.is_some_and(|event| start <= event && event <= end))
        };
        match self {
            Self::Flights(tasks) => tasks.contains(&assignment.task.plan.id),
            Self::Departure { airports, time } => airports.contains(&assignment.departure.icao.0)
                && contained(time, assignment.time.map(|time| time.0)),
            Self::Arrival { airports, time } => airports.contains(&assignment.arrival.icao.0)
                && contained(time, assignment.time.map(|time| time.1)),
            Self::Bidirectional { first, second, time } => ((first.contains(&assignment.departure.icao.0) && second.contains(&assignment.arrival.icao.0))
                || (second.contains(&assignment.departure.icao.0) && first.contains(&assignment.arrival.icao.0)))
                && contained(time, assignment.time.map(|time| time.0)) && contained(time, assignment.time.map(|time| time.1)),
            Self::EnabledAircraft(aircraft) => assignment.aircraft.is_some_and(|executor| aircraft.contains(&executor.reg_no.0)),
            Self::DisabledAircraft(aircraft) => assignment.aircraft.is_none_or(|executor| !aircraft.contains(&executor.reg_no.0)),
            Self::All(conditions) => conditions.iter().all(|condition| condition.matches(assignment)),
            Self::Custom(predicate) => predicate.matches(assignment),
        }
    }

    pub(super) fn validate(&self, references: &RestrictionReferences<'_>) -> Result<(), RecoveryError> {
        let valid_window = |time: &Option<(OffsetDateTime, OffsetDateTime)>| time.is_none_or(|(start, end)| start <= end);
        let valid = match self {
            Self::Flights(tasks) => tasks.iter().all(|task| references.tasks.contains(task.as_str())),
            Self::Departure { airports, time } | Self::Arrival { airports, time } => valid_window(time)
                && airports.iter().all(|airport| references.airports.contains(airport.as_str())),
            Self::Bidirectional { first, second, time } => valid_window(time)
                && first.iter().chain(second).all(|airport| references.airports.contains(airport.as_str())),
            Self::EnabledAircraft(aircraft) | Self::DisabledAircraft(aircraft) => aircraft.iter().all(|executor| references.aircraft.contains(executor.as_str())),
            Self::All(conditions) => { for condition in conditions { condition.validate(references)?; } true },
            Self::Custom(_) => true,
        };
        if valid { Ok(()) } else { Err(RecoveryError::InvalidRule("unknown restriction condition reference or reversed time window".into())) }
    }
}
