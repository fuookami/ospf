//! 装箱服务 / Packing services
//!
//! 几何守卫、余料分析、装载顺序、装箱器与渲染适配。
//! Geometry guard, remnant analysis, loading order, packer and render adaptation.

pub mod free_space_analyzer;
pub mod geometry_guard;
pub mod packer;
pub mod renderer_adapter;

pub use free_space_analyzer::{FreeSpaceAnalyzer, Remnant2};
pub use geometry_guard::{
    PackingGeometryGuard, PackingRotationRule, is_rotation_invariant, mixed_shape_diagnostic,
    placement_shape_diagnostic,
};
pub use packer::{LoadingOrderCalculator, Packer};
pub use renderer_adapter::PackingRendererAdapter;
