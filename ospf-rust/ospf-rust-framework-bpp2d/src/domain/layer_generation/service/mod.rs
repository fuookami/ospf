//! 层生成服务 / Layer generation services
//!
//! 块装载上下文、圆形网格、各类层生成器与组合上下文。
//! Block loading context, circle grids, the individual layer generators and the composite context.

pub mod block_loading_context;
pub mod circle_grid_generator;
pub mod generators;

pub use block_loading_context::BlockLoadingContext;
pub use circle_grid_generator::{
    CircleGridGenerator, CircleGridLayout, CircleGridMode, translate_placements,
};
pub use generators::{
    BlaLayerGenerator, BlockGlobalLayerGenerator, BlockLocalLayerGenerator, CircleGridLayerGenerator,
    HistoryLayerGenerator, MixedShapeLayerGenerator, PatternLayerGenerator, SpaceSplitLayerGenerator,
    initial_space_of, layer_of,
};
