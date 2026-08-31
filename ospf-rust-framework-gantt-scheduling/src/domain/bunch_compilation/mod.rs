//! 任务束编译上下文 / Bunch compilation context
//!
//! 映射 Kotlin `gantt-scheduling-domain-bunch-compilation-context` 子模块。
//! Maps the Kotlin `gantt-scheduling-domain-bunch-compilation-context` submodule.

pub mod model;
pub mod service;

/// 任务束编译聚合占位 / Bunch compilation aggregation placeholder
#[derive(Debug, Clone, Default)]
pub struct Aggregation;

/// 任务束编译上下文占位 / Bunch compilation context placeholder
#[derive(Debug, Clone, Default)]
pub struct BunchCompilationContext;
