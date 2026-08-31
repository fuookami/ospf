//! 任务编译上下文 / Task compilation context
//!
//! 映射 Kotlin `gantt-scheduling-domain-task-compilation-context` 子模块。
//! Maps the Kotlin `gantt-scheduling-domain-task-compilation-context` submodule.

pub mod model;
pub mod service;

/// 任务编译聚合占位 / Task compilation aggregation placeholder
#[derive(Debug, Clone, Default)]
pub struct Aggregation;

/// 迭代任务编译聚合占位 / Iterative task compilation aggregation placeholder
#[derive(Debug, Clone, Default)]
pub struct IterativeAggregation;

/// 迭代任务编译上下文占位 / Iterative task compilation context placeholder
#[derive(Debug, Clone, Default)]
pub struct IterativeContext;
