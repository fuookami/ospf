//! 应用层入口 / Application entry points
//!
//! 映射 Kotlin `gantt-scheduling-application` 子模块。
//! Maps the Kotlin `gantt-scheduling-application` submodule.

pub mod algorithm;
pub mod iteration;
pub mod model;
pub mod service;

/// 高级计划与排程入口标记 / Advanced planning and scheduling entry marker
#[derive(Debug, Clone, Copy, Default)]
pub struct APS;

/// 主生产计划入口标记 / Master production scheduling entry marker
#[derive(Debug, Clone, Copy, Default)]
pub struct MPS;

/// 批次排序计划入口标记 / Lot scheduling planning entry marker
#[derive(Debug, Clone, Copy, Default)]
pub struct LSP;

pub use algorithm::{
    BunchBranchAndPriceAlgorithm, BunchCGPolicy, ColumnGenerationPolicy,
    TaskColumnGenerationAlgorithm,
};
pub use iteration::{Iteration, IterationSnapshot};
