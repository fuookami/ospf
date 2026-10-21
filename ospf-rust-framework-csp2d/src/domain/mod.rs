//! CSP2D 领域层 / CSP2D domain layer
//!
//! 与 Kotlin 的 `csp2d-domain-*-context` 子模块一一对应。领域层泛型于 `V` 并保持物理量化；
//! 求解器数值类型只出现在 produce 主问题（建模边界）、sheet_assignment 求解边界与应用层。
//! Corresponds one-to-one with the Kotlin `csp2d-domain-*-context` submodules. The domain layer is generic
//! over `V` and physically quantified; solver value types appear only in the produce master problem (the
//! modelling boundary), the sheet_assignment solving boundary and the application layer.

pub mod material;
pub mod pattern_generation;
pub mod produce;
pub mod sheet_assignment;
pub mod wasting_minimization;
pub mod r#yield;
