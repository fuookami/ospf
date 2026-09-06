//! 分配策略 / Assignment policy
//!
//! 描述任务与执行者之间的分配关系。
//! Describes the assignment relationship between tasks and executors.

use super::ExecutorTrait;
use crate::infrastructure::TimeRange;

/// 分配策略 trait / Assignment policy trait
///
/// 描述执行者和时间范围的分配关系。
/// Describes the executor and time range assignment for a task.
pub trait AssignmentPolicyTrait<E>: Send + Sync + std::fmt::Debug + Clone + 'static
where
    E: ExecutorTrait,
{
    /// 获取已分配的执行者 / Get assigned executor
    fn executor(&self) -> Option<&E>;

    /// 获取已分配的时间范围 / Get assigned time range
    fn time(&self) -> Option<&TimeRange>;

    /// 是否完整分配 / Whether fully assigned
    ///
    /// 当执行者和时间范围都已分配时返回 `true`。
    /// Returns `true` when both executor and time range are assigned.
    fn full(&self) -> bool {
        self.executor().is_some() && self.time().is_some()
    }

    /// 是否为空分配 / Whether empty assignment
    ///
    /// 当执行者和时间范围都未分配时返回 `true`。
    /// Returns `true` when neither executor nor time range is assigned.
    fn empty(&self) -> bool {
        self.executor().is_none() && self.time().is_none()
    }
}

/// 基础分配策略 / Basic assignment policy
///
/// 提供最简的分配策略实现。
/// Provides a minimal assignment policy implementation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct BasicAssignmentPolicy<E: ExecutorTrait> {
    /// 已分配的执行者 / Assigned executor
    pub executor: Option<E>,
    /// 已分配的时间范围 / Assigned time range
    pub time: Option<TimeRange>,
}

impl<E: ExecutorTrait> BasicAssignmentPolicy<E> {
    /// 创建新的基础分配策略 / Create new basic assignment policy
    pub fn new(executor: Option<E>, time: Option<TimeRange>) -> Self {
        Self { executor, time }
    }

    /// 创建空分配策略 / Create empty assignment policy
    pub fn empty_policy() -> Self {
        Self {
            executor: None,
            time: None,
        }
    }

    /// 创建完整分配策略 / Create full assignment policy
    pub fn full_policy(executor: E, time: TimeRange) -> Self {
        Self {
            executor: Some(executor),
            time: Some(time),
        }
    }
}

impl<E: ExecutorTrait> AssignmentPolicyTrait<E> for BasicAssignmentPolicy<E> {
    fn executor(&self) -> Option<&E> {
        self.executor.as_ref()
    }

    fn time(&self) -> Option<&TimeRange> {
        self.time.as_ref()
    }
}

/// 执行者变更 / Executor change
///
/// 记录任务从一个执行者变更为另一个执行者。
/// Records a task changing from one executor to another.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorChange<E: ExecutorTrait> {
    /// 原执行者 / Original executor
    pub from: E,
    /// 新执行者 / New executor
    pub to: E,
}

impl<E: ExecutorTrait> ExecutorChange<E> {
    /// 创建新的执行者变更 / Create new executor change
    pub fn new(from: E, to: E) -> Self {
        Self { from, to }
    }
}
