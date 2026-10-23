//! 任务生成上下文 / Task generation context
//!
//! 映射 Kotlin `gantt-scheduling-domain-task-generation-context` 子模块。
//! Maps the Kotlin `gantt-scheduling-domain-task-generation-context` submodule.
mod pricing;

pub use pricing::{TaskCGPolicy, TaskPricingColumn, TaskPricingDomain, TaskPricingRequest};
pub(crate) use pricing::TaskPricingSession;
pub(crate) use pricing::validate_column;
