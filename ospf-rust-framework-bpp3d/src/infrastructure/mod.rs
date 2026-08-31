//! BPP3D 基础设施 / BPP3D infrastructure
//!
//! 映射 Kotlin `bpp3d-infrastructure` 子模块。
//! Maps the Kotlin `bpp3d-infrastructure` submodule.
//!
//! # 子模块 / Submodules
//!
//! - `geometry`: 类型化几何适配层 / Typed geometry adapter
//! - `packing_shape`: 包装形状 / Packing shapes
//! - `orientation`: 朝向 / Orientation
//! - `pwl_approximation`: PWL 半径平方近似 / PWL radius-squared approximation
//! - `renderer`: 渲染器 DTO / Renderer DTO

pub mod geometry;
pub mod orientation;
pub mod packing_shape;
pub mod pwl_approximation;
pub mod renderer;

pub use geometry::{
    MetricAabb2, MetricAabb3, MetricPlacement2, MetricPlacement3, MetricPoint2, MetricPoint3,
    MetricSize2, MetricSize3, MetricVector2, MetricVector3, scalar_cuboid3_to_typed_size,
    scalar_point2_to_typed, scalar_point3_to_typed, typed_point2_to_scalar, typed_point3_to_scalar,
    typed_size3_to_scalar_cuboid,
};
pub use orientation::{Orientation, OrientationCategory};
pub use packing_shape::{
    PackingShape3, PackingShapeType as PackingShapeTypeInfra, ShapeFootprint2,
    cuboid_packing_shape, cylinder_packing_shape,
};
pub use pwl_approximation::{
    ConservativeRadiusEnvelope, HorizontalCylinderSupportGeometry, PwlBreakpointStrategy,
    PwlRadiusApproximationConfig, PwlRadiusSquaredApproximation, SegmentCountDerivation,
    horizontal_cylinder_cuboid_support_coverage, horizontal_cylinder_support_radial_axis,
    intervals_cover_span,
};
pub use renderer::{
    RenderAlgorithmShapeType, RenderAxis3, RenderLoadingPlanDto, RenderLoadingPlanItemDto,
    RenderShapeType, SchemaDto,
};

/// 包装形状类型 / Packing shape type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackingShapeType {
    /// 长方体 / Cuboid
    Cuboid,
    /// 圆柱 / Cylinder
    Cylinder,
}

/// 算法侧包装形状类型 / Algorithm-side packing shape type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackingAlgorithmShapeType {
    /// 长方体 / Cuboid
    Cuboid,
    /// 竖直圆柱（Y 轴对齐）/ Vertical cylinder (Y-axis aligned)
    VerticalCylinder,
    /// X 轴横向圆柱 / X-axis horizontal cylinder
    HorizontalCylinderX,
    /// Z 轴横向圆柱 / Z-axis horizontal cylinder
    HorizontalCylinderZ,
}

impl PackingAlgorithmShapeType {
    /// 从圆柱对齐轴推导算法形状类型 / Derive algorithm shape type from cylinder alignment axis
    pub fn from_cylinder_axis(axis: ospf_rust_math::geometry::Axis3) -> Self {
        match axis {
            ospf_rust_math::geometry::Axis3::Y => Self::VerticalCylinder,
            ospf_rust_math::geometry::Axis3::X => Self::HorizontalCylinderX,
            ospf_rust_math::geometry::Axis3::Z => Self::HorizontalCylinderZ,
        }
    }
}
