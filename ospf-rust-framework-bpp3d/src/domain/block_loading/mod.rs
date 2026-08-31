//! 块装载上下文 / Block loading context
//!
//! 映射 Kotlin `bpp3d-domain-block-loading-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-block-loading-context` submodule.

pub mod model;
pub mod service;

/// 块装载上下文占位 / Block loading context placeholder
#[derive(Debug, Clone, Default)]
pub struct BlockLoadingContext;
