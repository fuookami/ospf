//! 块装载上下文 / Block loading context
//!
//! 映射 Kotlin `bpp3d-domain-block-loading-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-block-loading-context` submodule.

pub mod model;
pub mod service;

pub use model::{Block, BlockPlacement, ComplexBlock, ItemView, SimpleBlock, Space};

pub use service::{
    ComplexBlockGenerator, ComplexBlockGeneratorConfig, DepthFirstSearchAlgorithm,
    DepthFirstSearchConfig, MultiLayerHeuristicSearchAlgorithm, MultiLayerHeuristicSearchConfig,
    SimpleBlockGenerator, SimpleBlockGeneratorConfig,
};
