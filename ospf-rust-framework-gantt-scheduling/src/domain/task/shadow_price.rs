//! 甘特排程影子价格 / Gantt scheduling shadow price
//!
//! 集成框架 ShadowPriceMap，提供任务级和束级影子价格参数。
//! Integrates framework ShadowPriceMap, providing task-level and bunch-level
//! shadow price arguments.

use std::any::TypeId;
use ospf_rust_framework::model::shadow_price::ShadowPriceKey;
use super::executor::ExecutorTrait;

/// 甘特排程影子价格参数 / Gantt scheduling shadow price arguments
///
/// 基础影子价格参数，包含执行者和可选任务键。
/// Base shadow price arguments containing executor and optional task key.
#[derive(Debug, Clone)]
pub struct GanttSchedulingShadowPriceArguments<E: ExecutorTrait> {
    /// 执行者 / Executor
    pub executor: E,
    /// 任务键 / Task key (optional)
    pub task_key: Option<String>,
}

impl<E: ExecutorTrait> GanttSchedulingShadowPriceArguments<E> {
    /// 创建新的影子价格参数 / Create new shadow price arguments
    pub fn new(executor: E, task_key: Option<String>) -> Self {
        Self { executor, task_key }
    }

    /// 创建仅含执行者的参数 / Create executor-only arguments
    pub fn for_executor(executor: E) -> Self {
        Self {
            executor,
            task_key: None,
        }
    }
}

/// 束级影子价格参数 / Bunch-level shadow price arguments
///
/// 包含执行者、当前任务键和前驱任务键。
/// Contains executor, current task key, and predecessor task key.
#[derive(Debug, Clone)]
pub struct BunchGanttSchedulingShadowPriceArguments<E: ExecutorTrait> {
    /// 执行者 / Executor
    pub executor: E,
    /// 当前任务键 / Current task key
    pub task_key: Option<String>,
    /// 前驱任务键 / Predecessor task key
    pub prev_task_key: Option<String>,
}

impl<E: ExecutorTrait> BunchGanttSchedulingShadowPriceArguments<E> {
    /// 创建新的束级影子价格参数 / Create new bunch-level shadow price arguments
    pub fn new(
        executor: E,
        task_key: Option<String>,
        prev_task_key: Option<String>,
    ) -> Self {
        Self {
            executor,
            task_key,
            prev_task_key,
        }
    }
}

/// 甘特排程影子价格键 / Gantt scheduling shadow price key
///
/// 基于 TypeId 和名称的影子价格键，用于在 ShadowPriceMap 中索引。
/// TypeId and name-based shadow price key for indexing in ShadowPriceMap.
#[derive(Debug, Clone)]
pub struct GanttShadowPriceKey {
    /// 类型 ID / Type ID
    pub type_id: TypeId,
    /// 名称 / Name
    pub name: String,
}

impl GanttShadowPriceKey {
    /// 创建新的影子价格键 / Create new shadow price key
    pub fn new<T: 'static>(name: impl Into<String>) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            name: name.into(),
        }
    }

    /// 创建任务编译影子价格键 / Create task compilation shadow price key
    pub fn task_compilation(task_key: &str) -> Self {
        Self::new::<Self>(format!("task_compilation_{}", task_key))
    }

    /// 创建执行器编译影子价格键 / Create executor compilation shadow price key
    pub fn executor_compilation(executor_id: &str) -> Self {
        Self::new::<Self>(format!("executor_compilation_{}", executor_id))
    }
}

impl From<GanttShadowPriceKey> for ShadowPriceKey {
    fn from(key: GanttShadowPriceKey) -> Self {
        ShadowPriceKey::named::<ShadowPriceKey>(key.name)
    }
}
