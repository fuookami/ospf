//! 约束规划模型 / Constraint-programming model.
//!
//! 本模块使用精确 `i64` AST 和不可变 snapshot，与现有 `f64` 数学模型保持边界分离。
//! This module uses an exact `i64` AST and immutable snapshots, separated from the existing
//! `f64` mathematical-model boundary.

pub mod constraint;
pub mod domain;
pub mod expression;
pub mod interval;
pub mod literal;
pub mod model;
pub mod objective;
pub mod snapshot;
pub mod variable;

pub use constraint::*;
pub use domain::*;
pub use expression::*;
pub use interval::*;
pub use literal::*;
pub use model::*;
pub use objective::*;
pub use snapshot::*;
pub use variable::*;
