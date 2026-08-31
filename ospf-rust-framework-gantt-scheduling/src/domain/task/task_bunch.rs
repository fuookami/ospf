//! 任务束 / Task bunch
//!
//! 表示分配给同一执行者的有序任务组。
//! Represents an ordered group of tasks assigned to the same executor.

use time::{Duration, OffsetDateTime};
use ospf_rust_core::solver::value::SolveValue;
use super::{ExecutorTrait, AssignmentPolicyTrait, TaskTrait, Cost};
use crate::infrastructure::TimeRange;

/// 任务束 / Task bunch
///
/// 表示分配给同一执行者的有序任务组，包含成本、初始可用性和迭代信息。
/// Represents an ordered group of tasks assigned to the same executor,
/// containing cost, initial usability, and iteration information.
#[derive(Debug, Clone)]
pub struct TaskBunch<T, E, V: SolveValue>
where
    E: ExecutorTrait,
{
    /// 执行者 / Executor
    pub executor: E,
    /// 时间范围 / Time range
    pub time: TimeRange,
    /// 任务列表 / Task list
    pub tasks: Vec<T>,
    /// 成本 / Cost
    pub cost: Cost<V>,
    /// 迭代号 / Iteration number
    pub iteration: i64,
}

impl<T, E, V: SolveValue> TaskBunch<T, E, V>
where
    E: ExecutorTrait,
{
    /// 创建新的任务束 / Create new task bunch
    pub fn new(
        executor: E,
        time: TimeRange,
        tasks: Vec<T>,
        cost: Cost<V>,
        iteration: i64,
    ) -> Self {
        Self {
            executor,
            time,
            tasks,
            cost,
            iteration,
        }
    }

    /// 任务数量 / Number of tasks
    pub fn size(&self) -> usize {
        self.tasks.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// 时间范围持续时间 / Time range duration
    pub fn duration(&self) -> Duration {
        self.time.duration()
    }
}

/// 任务束扩展 trait / Task bunch extension trait
///
/// 为包含 `TaskTrait` 实现的任务束提供计算属性。
/// Provides computed properties for task bunches containing `TaskTrait` implementations.
pub trait TaskBunchExt<T, E, A, V: SolveValue>: Send + Sync
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
    T: TaskTrait<E, A>,
{
    /// 获取任务列表 / Get task list
    fn tasks(&self) -> &[T];

    /// 获取执行者 / Get executor
    fn executor(&self) -> &E;

    /// 获取时间范围 / Get time range
    fn time(&self) -> &TimeRange;

    /// 获取成本 / Get cost
    fn cost(&self) -> &Cost<V>;
    ///
    /// 所有任务持续时间之和。
    /// Sum of all task durations.
    fn busy_time(&self) -> Duration {
        self.time().duration()
    }

    /// 总延迟 / Total delay
    fn total_delay(&self) -> Duration {
        Duration::ZERO
    }

    /// 总提前 / Total advance
    fn total_advance(&self) -> Duration {
        Duration::ZERO
    }

    /// 执行者变更次数 / Executor change count
    fn executor_change_count(&self) -> u64 {
        0
    }

    /// 完工时间 / Makespan
    fn makespan(&self) -> OffsetDateTime {
        self.time().end
    }

    /// 成本密度 / Cost density
    fn cost_density(&self) -> f64 {
        let size = self.tasks().len();
        if size == 0 {
            0.0
        } else {
            self.cost().solver_cost(0.0) / size as f64
        }
    }
}
