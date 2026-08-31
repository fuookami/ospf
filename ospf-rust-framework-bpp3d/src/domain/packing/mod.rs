//! 装箱上下文 / Packing context
//!
//! 映射 Kotlin `bpp3d-domain-packing-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-packing-context` submodule.

pub mod model;
pub mod service;

/// 装箱上下文占位 / Packing context placeholder
#[derive(Debug, Clone, Default)]
pub struct PackingContext;
