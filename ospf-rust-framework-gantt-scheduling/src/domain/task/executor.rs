//! 执行者 / Executor
//!
//! 表示可以执行任务的实体。
//! Represents an entity that can execute tasks.

use time::OffsetDateTime;

/// 执行者 trait / Executor trait
///
/// 定义可以执行任务的实体接口，如工人、机器、产线等。
/// Defines the interface for entities that can execute tasks,
/// such as workers, machines, production lines, etc.
pub trait ExecutorTrait: Send + Sync + std::fmt::Debug + Clone + PartialEq + Eq + std::hash::Hash + 'static {
    /// 执行者 ID / Executor ID
    fn id(&self) -> &str;

    /// 执行者名称 / Executor name
    fn name(&self) -> &str;

    /// 实际 ID / Actual ID
    ///
    /// 默认返回 `id()`。子类可覆盖以区分逻辑 ID 和实际 ID。
    /// Defaults to `id()`. Subclasses may override to distinguish logical ID from actual ID.
    fn actual_id(&self) -> &str {
        self.id()
    }

    /// 显示名称 / Display name
    ///
    /// 默认返回 `name()`。
    /// Defaults to `name()`.
    fn display_name(&self) -> &str {
        self.name()
    }
}

/// 执行者初始可用性 / Executor initial usability
///
/// 描述执行者在排程开始时的初始状态。
/// Describes the executor's initial state at the start of scheduling.
#[derive(Debug, Clone)]
pub struct ExecutorInitialUsability<T, E>
where
    E: ExecutorTrait,
{
    /// 上一个任务 / Last task
    pub last_task: Option<T>,
    /// 可用时间 / Enabled time
    pub enabled_time: OffsetDateTime,
    _marker: std::marker::PhantomData<E>,
}

impl<T, E: ExecutorTrait> ExecutorInitialUsability<T, E> {
    /// 创建新的初始可用性 / Create new initial usability
    pub fn new(last_task: Option<T>, enabled_time: OffsetDateTime) -> Self {
        Self {
            last_task,
            enabled_time,
            _marker: std::marker::PhantomData,
        }
    }

    /// 是否启用 / Whether the executor is on
    ///
    /// 当 `last_task` 存在时返回 `true`。
    /// Returns `true` when `last_task` is present.
    pub fn on(&self) -> bool {
        self.last_task.is_some()
    }
}

/// 基础执行者 / Basic executor
///
/// 提供最简的执行者实现，仅包含 ID 和名称。
/// Provides a minimal executor implementation with just ID and name.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BasicExecutor {
    /// 执行者 ID / Executor ID
    pub id: String,
    /// 执行者名称 / Executor name
    pub name: String,
}

impl BasicExecutor {
    /// 创建新的基础执行者 / Create new basic executor
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

impl ExecutorTrait for BasicExecutor {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}
