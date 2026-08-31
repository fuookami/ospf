//! 任务计划 / Task plan
//!
//! 描述任务的计划信息，包括时间约束、执行者约束和状态标记。
//! Describes task plan information including time constraints, executor constraints, and status flags.

use super::ExecutorTrait;
use crate::domain::common::{TaskPlanId, TaskPlanIdTrait};
use crate::infrastructure::TimeRange;
use std::collections::HashSet;
use time::{Duration, OffsetDateTime};

/// 任务状态 / Task status
///
/// 描述任务的各种约束和特性标记。
/// Describes various constraint and characteristic flags for a task.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    /// 不允许提前 / Not allowed to advance
    NotAdvance,
    /// 不允许延迟 / Not allowed to delay
    NotDelay,
    /// 不允许取消 / Not allowed to cancel
    NotCancel,
    /// 不偏好取消 / Not preferred to cancel
    NotCancelPreferred,
    /// 不允许更换执行者 / Not allowed to change executor
    NotExecutorChange,
    /// 可并行 / Can be parallelized
    Parallelable,
    /// 可分割 / Can be divided
    Divisible,
}

/// 任务计划 trait / Task plan trait
///
/// 定义任务的计划信息接口。
/// Defines the interface for task plan information.
pub trait TaskPlanTrait<E>: Send + Sync + std::fmt::Debug + 'static
where
    E: ExecutorTrait,
{
    /// 计划 ID 类型 / Plan id type
    type Id: TaskPlanIdTrait;

    /// 计划 ID / Plan ID
    fn id(&self) -> &Self::Id;
    /// 计划名称 / Plan name
    fn name(&self) -> &str;
    /// 状态集合 / Status set
    fn status(&self) -> &HashSet<TaskStatus>;
    /// 指定执行者 / Specified executor
    fn executor(&self) -> Option<&E> {
        None
    }
    /// 可用执行者集合 / Enabled executors set
    fn enabled_executors(&self) -> &Vec<E>;
    /// 计划时间 / Scheduled time
    fn scheduled_time(&self) -> Option<&TimeRange> {
        None
    }
    /// 实际时间 / Actual time (defaults to scheduled_time)
    fn time(&self) -> Option<&TimeRange> {
        self.scheduled_time()
    }
    /// 最早结束时间 / Earliest end time
    fn earliest_end_time(&self) -> Option<OffsetDateTime> {
        None
    }
    /// 最晚结束时间 / Latest end time
    fn last_end_time(&self) -> Option<OffsetDateTime> {
        None
    }
    /// 持续时间 / Duration
    fn duration(&self) -> Option<Duration> {
        self.time()
            .map(|t| t.duration())
            .or_else(|| self.scheduled_time().map(|t| t.duration()))
    }
    /// 最小持续时间 / Minimum duration
    fn min_duration(&self) -> Option<Duration> {
        self.duration()
    }
    /// 最大持续时间 / Maximum duration
    fn max_duration(&self) -> Option<Duration> {
        self.duration()
    }
    /// 是否允许取消 / Whether cancellation is enabled
    fn cancel_enabled(&self) -> bool {
        !self.status().contains(&TaskStatus::NotCancel)
    }
    /// 是否偏好不取消 / Whether not-cancel is preferred
    fn not_cancel_preferred(&self) -> bool {
        self.status().contains(&TaskStatus::NotCancelPreferred)
    }
    /// 是否允许提前 / Whether advance is enabled
    fn advance_enabled(&self) -> bool {
        !self.status().contains(&TaskStatus::NotAdvance)
    }
    /// 是否允许延迟 / Whether delay is enabled
    fn delay_enabled(&self) -> bool {
        !self.status().contains(&TaskStatus::NotDelay)
    }
    /// 是否允许更换执行者 / Whether executor change is enabled
    fn executor_change_enabled(&self) -> bool {
        !self.status().contains(&TaskStatus::NotExecutorChange)
    }
    /// 是否可并行 / Whether parallelable
    fn parallelable(&self) -> bool {
        self.status().contains(&TaskStatus::Parallelable)
    }
    /// 是否可分割 / Whether divisible
    fn divisible(&self) -> bool {
        self.status().contains(&TaskStatus::Divisible)
    }
}

/// 单步任务计划 / Single step task plan
///
/// 最简的任务计划实现，包含 ID、名称、可用执行者和状态。
/// Minimal task plan implementation with ID, name, enabled executors, and status.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct SingleStepTaskPlan<E: ExecutorTrait> {
    /// 计划 ID / Plan ID
    pub id: TaskPlanId,
    /// 计划名称 / Plan name
    pub name: String,
    /// 可用执行者列表 / Enabled executors list
    pub enabled_executors: Vec<E>,
    /// 状态集合 / Status set
    pub status: HashSet<TaskStatus>,
    /// 指定执行者 / Specified executor
    pub executor: Option<E>,
    /// 计划时间 / Scheduled time
    pub scheduled_time: Option<TimeRange>,
    /// 最早结束时间 / Earliest end time
    pub earliest_end_time: Option<OffsetDateTime>,
    /// 最晚结束时间 / Latest end time
    pub last_end_time: Option<OffsetDateTime>,
    /// 持续时间（覆盖 time 的 duration） / Duration (overrides time's duration)
    pub duration: Option<Duration>,
    /// 最小持续时间 / Minimum duration
    pub min_duration: Option<Duration>,
    /// 最大持续时间 / Maximum duration
    pub max_duration: Option<Duration>,
}

impl<E: ExecutorTrait> SingleStepTaskPlan<E> {
    /// 创建新的单步任务计划 / Create new single step task plan
    pub fn new(
        id: impl Into<TaskPlanId>,
        name: impl Into<String>,
        enabled_executors: Vec<E>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            enabled_executors,
            status: HashSet::new(),
            executor: None,
            scheduled_time: None,
            earliest_end_time: None,
            last_end_time: None,
            duration: None,
            min_duration: None,
            max_duration: None,
        }
    }

    /// 设置状态 / Set status
    pub fn with_status(mut self, status: HashSet<TaskStatus>) -> Self {
        self.status = status;
        self
    }

    /// 设置指定执行者 / Set specified executor
    pub fn with_executor(mut self, executor: E) -> Self {
        self.executor = Some(executor);
        self
    }

    /// 设置计划时间 / Set scheduled time
    pub fn with_scheduled_time(mut self, time: TimeRange) -> Self {
        self.scheduled_time = Some(time);
        self
    }

    /// 设置持续时间 / Set duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// 设置最早结束时间 / Set earliest end time
    pub fn with_earliest_end_time(mut self, time: OffsetDateTime) -> Self {
        self.earliest_end_time = Some(time);
        self
    }

    /// 设置最晚结束时间 / Set latest end time
    pub fn with_last_end_time(mut self, time: OffsetDateTime) -> Self {
        self.last_end_time = Some(time);
        self
    }
}

impl<E: ExecutorTrait> TaskPlanTrait<E> for SingleStepTaskPlan<E> {
    type Id = TaskPlanId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn status(&self) -> &HashSet<TaskStatus> {
        &self.status
    }

    fn executor(&self) -> Option<&E> {
        self.executor.as_ref()
    }

    fn enabled_executors(&self) -> &Vec<E> {
        &self.enabled_executors
    }

    fn scheduled_time(&self) -> Option<&TimeRange> {
        self.scheduled_time.as_ref()
    }

    fn earliest_end_time(&self) -> Option<OffsetDateTime> {
        self.earliest_end_time
    }

    fn last_end_time(&self) -> Option<OffsetDateTime> {
        self.last_end_time
    }

    fn duration(&self) -> Option<Duration> {
        self.duration
            .or_else(|| self.time().map(|t| t.duration()))
            .or_else(|| self.scheduled_time().map(|t| t.duration()))
    }

    fn min_duration(&self) -> Option<Duration> {
        self.min_duration.or(self.duration())
    }

    fn max_duration(&self) -> Option<Duration> {
        self.max_duration.or(self.duration())
    }
}
