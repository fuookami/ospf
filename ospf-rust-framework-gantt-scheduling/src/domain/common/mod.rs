//! 通用建模辅助 / Common modeling helpers
//!
//! 放置跨 Gantt 子领域复用的约束映射和动态模型状态辅助结构。
//! Hosts constraint mapping and dynamic model-state helper structures shared across Gantt domains.

pub mod constraint_index;

pub use constraint_index::{ConstraintIndexEntry, ConstraintIndexKey, ConstraintIndexMap};
pub use ospf_rust_framework::model::{
    ColumnRange, ColumnState, DynamicModelLifecycle as GanttDynamicModelLifecycle,
    DynamicModelSnapshot as GanttDynamicModelSnapshot, DynamicModelState as GanttModelStateFacade,
};
