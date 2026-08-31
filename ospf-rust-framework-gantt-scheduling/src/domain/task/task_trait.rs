//! 任务 trait / Task trait
//!
//! 定义任务的核心接口，包括时间、成本、约束和分配。
//! Defines the core interface for tasks, including time, cost, constraints, and assignment.

use time::{Duration, OffsetDateTime};
use std::collections::HashSet;
use super::{ExecutorTrait, AssignmentPolicyTrait, TaskStatus};
use crate::infrastructure::TimeRange;

/// 任务类型标识 / Task type identifier
///
/// 使用 `TypeId` 和字符串标识任务类型，不依赖 JVM class。
/// Uses `TypeId` and string to identify task types, not JVM class.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskType {
    /// 类型名称 / Type name
    pub name: String,
}

impl TaskType {
    /// 创建新的任务类型 / Create new task type
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// 默认任务类型 / Default task type
    pub fn default_type() -> Self {
        Self::new("Task")
    }
}

/// 任务键 / Task key
///
/// 复合键，包含任务 ID 和类型。
/// Composite key containing task ID and type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskKey {
    /// 任务 ID / Task ID
    pub id: String,
    /// 任务类型 / Task type
    pub type_: TaskType,
}

impl TaskKey {
    /// 创建新的任务键 / Create new task key
    pub fn new(id: impl Into<String>, type_: TaskType) -> Self {
        Self { id: id.into(), type_ }
    }
}

/// 任务 trait / Task trait
///
/// 定义调度任务的核心接口。大量属性通过默认方法实现，
/// 实现者只需提供 `id`、`name` 和少量核心属性。
///
/// Defines the core interface for scheduling tasks.
/// Most properties are implemented via default methods;
/// implementers only need to provide `id`, `name`, and a few core properties.
pub trait TaskTrait<E, A>: Send + Sync + std::fmt::Debug + 'static
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
{
    // ========================================================================
    // 核心属性（必须实现）/ Core properties (must implement)
    // ========================================================================

    /// 任务 ID / Task ID
    fn id(&self) -> &str;
    /// 任务名称 / Task name
    fn name(&self) -> &str;

    // ========================================================================
    // 默认属性 / Default properties
    // ========================================================================

    /// 任务类型 / Task type
    fn type_(&self) -> TaskType {
        TaskType::default_type()
    }

    /// 任务键 / Task key
    fn key(&self) -> TaskKey {
        TaskKey::new(self.id(), self.type_())
    }

    /// 实际 ID / Actual ID
    fn actual_id(&self) -> &str {
        self.id()
    }

    /// 显示名称 / Display name
    fn display_name(&self) -> &str {
        self.name()
    }

    /// 状态集合 / Status set
    fn status(&self) -> HashSet<TaskStatus> {
        HashSet::new()
    }

    /// 分配策略 / Assignment policy
    fn assignment_policy(&self) -> Option<&A> {
        None
    }

    /// 指定执行者 / Specified executor
    fn executor(&self) -> Option<&E> {
        self.assignment_policy().and_then(|p| p.executor())
    }

    /// 可用执行者集合 / Enabled executors set
    fn enabled_executors(&self) -> Vec<&E> {
        vec![]
    }

    /// 计划时间 / Scheduled time
    fn scheduled_time(&self) -> Option<&TimeRange> {
        None
    }

    /// 实际时间 / Actual time
    fn time(&self) -> Option<&TimeRange> {
        self.assignment_policy().and_then(|p| p.time())
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
        self.time().map(|t| t.duration())
    }

    /// 最小持续时间 / Minimum duration
    fn min_duration(&self) -> Option<Duration> {
        self.duration()
    }

    /// 最大持续时间 / Maximum duration
    fn max_duration(&self) -> Option<Duration> {
        self.duration()
    }

    /// 时间窗口 / Time window
    fn time_window(&self) -> Option<&TimeRange> {
        None
    }

    /// 最早开始时间 / Earliest start time
    fn earliest_start_time(&self) -> Option<OffsetDateTime> {
        None
    }

    /// 最晚开始时间 / Latest start time
    fn last_start_time(&self) -> Option<OffsetDateTime> {
        None
    }

    /// 最大延迟 / Maximum delay
    fn max_delay(&self) -> Option<Duration> {
        None
    }

    /// 最大提前 / Maximum advance
    fn max_advance(&self) -> Option<Duration> {
        None
    }

    // ========================================================================
    // 状态标记（从 status 集合派生）/ Status flags (derived from status set)
    // ========================================================================

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

    // ========================================================================
    // 计算属性 / Computed properties
    // ========================================================================

    /// 提前时间 / Advance time
    ///
    /// 当计划时间早于排程时间时返回提前量。
    /// Returns advance amount when planned time is earlier than scheduled time.
    fn advance(&self) -> Duration {
        if let (Some(scheduled), Some(actual)) = (self.scheduled_time(), self.time()) {
            if actual.start < scheduled.start {
                scheduled.start - actual.start
            } else {
                Duration::ZERO
            }
        } else {
            Duration::ZERO
        }
    }

    /// 延迟时间 / Delay time
    ///
    /// 当实际时间晚于排程时间时返回延迟量。
    /// Returns delay amount when actual time is later than scheduled time.
    fn delay(&self) -> Duration {
        if let (Some(scheduled), Some(actual)) = (self.scheduled_time(), self.time()) {
            if actual.start > scheduled.start {
                actual.start - scheduled.start
            } else {
                Duration::ZERO
            }
        } else {
            Duration::ZERO
        }
    }

    /// 超最大延迟时间 / Over maximum delay time
    ///
    /// 当延迟超过最大延迟限制时返回超额量。
    /// Returns excess when delay exceeds maximum delay limit.
    fn over_max_delay(&self) -> Duration {
        let delay = self.delay();
        let max_delay = self.max_delay().unwrap_or(Duration::ZERO);
        if delay > max_delay {
            delay - max_delay
        } else {
            Duration::ZERO
        }
    }

    /// 超最大提前时间 / Over maximum advance time
    fn over_max_advance(&self) -> Duration {
        let advance = self.advance();
        let max_advance = self.max_advance().unwrap_or(Duration::ZERO);
        if advance > max_advance {
            advance - max_advance
        } else {
            Duration::ZERO
        }
    }

    /// 是否准时 / Whether on time
    fn on_time(&self) -> bool {
        self.over_max_delay() == Duration::ZERO && self.over_max_advance() == Duration::ZERO
    }

    // ========================================================================
    // 动态方法 / Dynamic methods
    // ========================================================================

    /// 按执行者获取持续时间 / Get duration by executor
    fn duration_for_executor(&self, _executor: &E) -> Option<Duration> {
        self.duration()
    }

    /// 按执行者获取最早开始时间 / Get earliest start time by executor
    fn earliest_start_time_for_executor(&self, _executor: &E) -> Option<OffsetDateTime> {
        self.earliest_start_time()
    }

    /// 按执行者获取最晚开始时间 / Get latest start time by executor
    fn last_start_time_for_executor(&self, _executor: &E) -> Option<OffsetDateTime> {
        self.last_start_time()
    }

    /// 连接时间 / Connection time
    ///
    /// 两个相邻任务之间的连接时间。
    /// Connection time between two adjacent tasks.
    fn connection_time(
        &self,
        _prev_task: Option<&dyn TaskTrait<E, A>>,
        _succ_task: Option<&dyn TaskTrait<E, A>>,
    ) -> Option<Duration> {
        None
    }

    /// 连接时间（指定执行者）/ Connection time with specified executor
    fn connection_time_for_executor(
        &self,
        _executor: &E,
        _prev_task: Option<&dyn TaskTrait<E, A>>,
        _succ_task: Option<&dyn TaskTrait<E, A>>,
    ) -> Duration {
        Duration::ZERO
    }

    /// 是否启用排程 / Whether scheduling is enabled
    fn schedule_enabled(&self, _time_window: &TimeRange) -> bool {
        false
    }

    /// 是否需要排程 / Whether scheduling is needed
    fn schedule_needed(&self, _time_window: &TimeRange) -> bool {
        false
    }

    /// 是否允许分配 / Whether assignment is enabled
    fn assigning_enabled(&self, _policy: &A) -> bool {
        false
    }
}
