//! 任务领域模型 / Task domain models
//!
//! 映射 Kotlin `gantt-scheduling-domain-task-context` 子模块。
//! Maps the Kotlin `gantt-scheduling-domain-task-context` submodule.
//!
//! # 核心类型 / Core Types
//!
//! - [`ExecutorTrait`]: 执行者接口 / Executor interface
//! - [`AssignmentPolicyTrait`]: 分配策略接口 / Assignment policy interface
//! - [`TaskTrait`]: 任务接口 / Task interface
//! - [`TaskPlanTrait`]: 任务计划接口 / Task plan interface
//! - [`TaskBunch`]: 任务束 / Task bunch
//! - [`Cost`]: 成本 / Cost
//! - [`SchedulingSolverValueAdapter`]: 求解器值适配器 / Solver value adapter

pub mod executor;
pub mod assignment;
pub mod task_trait;
pub mod task_plan;
pub mod task_bunch;
pub mod cost;
pub mod scheduling_solver_value_adapter;
pub mod shadow_price;

// ========================================================================
// 公共重导出 / Public re-exports
// ========================================================================

pub use executor::{
    ExecutorTrait,
    ExecutorInitialUsability,
    BasicExecutor,
};

pub use assignment::{
    AssignmentPolicyTrait,
    BasicAssignmentPolicy,
    ExecutorChange,
};

pub use task_trait::{
    TaskTrait,
    TaskType,
    TaskKey,
};

pub use task_plan::{
    TaskStatus,
    TaskPlanTrait,
    SingleStepTaskPlan,
};

pub use task_bunch::TaskBunch;

pub use cost::{
    CostItem,
    Cost,
    MutableCost,
};

pub use scheduling_solver_value_adapter::{
    SchedulingSolverValueAdapter,
    F64SolverValueAdapter,
    GenericSolverValueAdapter,
};

pub use shadow_price::{
    GanttSchedulingShadowPriceArguments,
    BunchGanttSchedulingShadowPriceArguments,
    GanttShadowPriceKey,
};
