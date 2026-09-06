//! 通用建模辅助 / Common modeling helpers
//!
//! 放置跨 Gantt 子领域复用的约束映射和动态模型状态辅助结构。
//! Hosts constraint mapping and dynamic model-state helper structures shared across Gantt domains.

pub mod constraint_index;
pub mod id;

pub use constraint_index::{ConstraintIndexEntry, ConstraintIndexKey, ConstraintIndexMap};
pub use id::{
    ExecutorId, ExecutorIdTrait, GanttId, ProductionActionId, ProductionActionIdTrait,
    ProductionMaterialId, ProductionMaterialIdTrait, ResourceId, ResourceIdTrait, TaskId,
    TaskIdTrait, TaskPlanId, TaskPlanIdTrait, TaskStepId, TaskStepIdTrait, executor_id,
    production_action_id, production_material_id, resource_id, task_id, task_plan_id, task_step_id,
};
pub use ospf_rust_framework::model::{
    ColumnRange, ColumnState, DynamicModelLifecycle as GanttDynamicModelLifecycle,
    DynamicModelSnapshot as GanttDynamicModelSnapshot, DynamicModelState as GanttModelStateFacade,
};
