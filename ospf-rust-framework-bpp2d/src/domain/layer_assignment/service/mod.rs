//! 层分配服务 / Layer assignment services
//!
//! 板材容量约束与列选择诊断。
//! Sheet capacity constraint and column-selection diagnostics.

pub mod capacity_constraint;
pub mod selection;

pub use capacity_constraint::{SheetCapacityConstraint, sheets_used_by_copies};
pub use selection::{
    ColumnSelection, select_columns_by_copies, total_area_used, total_waste_area,
};
