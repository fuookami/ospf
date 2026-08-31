//! 货物领域上下文 / Item domain context
//!
//! 映射 Kotlin `bpp3d-domain-item-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-item-context` submodule.

pub mod model;
pub mod service;

/// 货物上下文占位 / Item context placeholder
#[derive(Debug, Clone, Default)]
pub struct ItemContext;

/// 货物聚合占位 / Item aggregation placeholder
#[derive(Debug, Clone, Default)]
pub struct Aggregation;
