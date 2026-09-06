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

pub mod assignment;
pub mod cost;
pub mod cost_policy;
pub mod executor;
pub mod scheduling_solver_value_adapter;
pub mod shadow_price;
pub mod task_bunch;
pub mod task_plan;
pub mod task_step_graph;
pub mod task_trait;

// ========================================================================
// 公共重导出 / Public re-exports
// ========================================================================

pub use crate::domain::common::{
    ExecutorId, TaskId, TaskPlanId, TaskStepId, executor_id, task_id, task_plan_id, task_step_id,
};

pub use executor::{BasicExecutor, ExecutorInitialUsability, ExecutorTrait};

pub use assignment::{AssignmentPolicyTrait, BasicAssignmentPolicy, ExecutorChange};

pub use task_trait::{TaskKey, TaskTrait, TaskType};

pub use task_plan::{SingleStepTaskPlan, TaskPlanTrait, TaskStatus};

pub use task_step_graph::{
    BackwardTaskStepVector, BasicTaskStep, ForwardTaskStepVector, StartSteps, StepRelation,
    TaskStepGraph, TaskStepGraphBuilder, TaskStepTrait,
};

pub use task_bunch::TaskBunch;

pub use cost::{Cost, CostItem, MutableCost};

pub use cost_policy::{
    BunchCostPolicy, CostBreakdown, DefaultBunchCostPolicy, FunctionalBunchCostPolicy,
};

pub use scheduling_solver_value_adapter::{
    F64SolverValueAdapter, SchedulingSolverValueAdapter, SolverValueAdapter,
};

pub use shadow_price::{
    BunchGanttSchedulingShadowPriceArguments, ExecutorSlotCompilationShadowPriceKey,
    GanttSchedulingShadowPriceArguments, GanttShadowPriceKey,
    SlotBunchGanttSchedulingShadowPriceArguments,
};
