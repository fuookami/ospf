//! 层生成上下文 / Layer generation context
//!
//! 映射 Kotlin `bpp3d-domain-layer-generation-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-layer-generation-context` submodule.

/// 层生成上下文占位 / Layer generation context placeholder
#[derive(Debug, Clone, Default)]
pub struct LayerGenerationContext;

/// 层生成请求占位 / Layer generation request placeholder
#[derive(Debug, Clone, Default)]
pub struct LayerGenerationRequest;

/// 层生成结果占位 / Layer generation result placeholder
#[derive(Debug, Clone, Default)]
pub struct LayerGenerationResult;

/// 层生成器 trait 占位 / Layer generator trait placeholder
pub trait LayerGenerator: Send + Sync {}
