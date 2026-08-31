//! 产能排程上下文 / Capacity scheduling context
//!
//! 映射 Kotlin `gantt-scheduling-domain-capacity-scheduling-context` 子模块。
//! Maps the Kotlin `gantt-scheduling-domain-capacity-scheduling-context` submodule.

pub mod model;
pub mod service;

/// 产能排程聚合占位 / Capacity scheduling aggregation placeholder
#[derive(Debug, Clone, Default)]
pub struct Aggregation;

/// 产能排程上下文占位 / Capacity scheduling context placeholder
#[derive(Debug, Clone, Default)]
pub struct CapacitySchedulingContext;
