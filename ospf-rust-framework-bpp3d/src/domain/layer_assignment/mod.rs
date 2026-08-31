//! 层分配上下文 / Layer assignment context
//!
//! 映射 Kotlin `bpp3d-domain-layer-assignment-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-layer-assignment-context` submodule.

pub mod model;
pub mod service;

/// 层分配上下文占位 / Layer assignment context placeholder
#[derive(Debug, Clone, Default)]
pub struct LayerAssignmentContext;

/// 不精确聚合占位 / Imprecise aggregation placeholder
#[derive(Debug, Clone, Default)]
pub struct ImpreciseAggregation;

/// 精确聚合占位 / Precise aggregation placeholder
#[derive(Debug, Clone, Default)]
pub struct PreciseAggregation;
