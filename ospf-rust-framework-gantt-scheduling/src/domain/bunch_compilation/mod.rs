//! 任务束编译上下文 / Bunch compilation context
//!
//! 映射 Kotlin `gantt-scheduling-domain-bunch-compilation-context` 子模块。
//! 实现列生成主问题的编译模型、聚合、约束和目标。
//!
//! Maps the Kotlin `gantt-scheduling-domain-bunch-compilation-context` submodule.
//! Implements compilation model, aggregation, constraints, and objectives for the column generation master problem.

pub mod context;
pub mod iterative;
pub mod model;
pub mod service;
pub mod slot_based;

pub use model::{BunchCompilation, BunchAggregation, BunchEntry, BunchSolution, BunchSolutionSummary};
pub use iterative::IterativeBunchCompilation;
pub use context::{
    IterativeBunchCompilationContext,
    BasicBunchCompilationContext,
    BunchShadowPricePipeline,
    TaskShadowPriceKey,
};
pub use slot_based::{
    BasicSlotBasedBunchCompilationContext,
    SlotBasedBunchCompilationContext,
    SlotBasedCapacityPreSolver,
    StaticSlotBasedCapacityPreSolver,
};
pub use service::limits::ExecutorSlotCompilationConstraint;
