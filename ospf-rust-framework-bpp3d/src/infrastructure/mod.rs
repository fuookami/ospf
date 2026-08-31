//! BPP3D 基础设施 / BPP3D infrastructure
//!
//! 映射 Kotlin `bpp3d-infrastructure` 子模块。
//! Maps the Kotlin `bpp3d-infrastructure` submodule.

pub mod geometry;
pub mod renderer;

/// 包装形状类型占位 / Packing shape type placeholder
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackingShapeType {
    /// 长方体 / Cuboid
    Cuboid,
    /// 圆柱 / Cylinder
    Cylinder,
}

/// 算法侧包装形状类型占位 / Algorithm-side packing shape type placeholder
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackingAlgorithmShapeType {
    /// 长方体 / Cuboid
    Cuboid,
    /// 竖直圆柱 / Vertical cylinder
    VerticalCylinder,
    /// X 轴横向圆柱 / X-axis horizontal cylinder
    HorizontalCylinderX,
    /// Z 轴横向圆柱 / Z-axis horizontal cylinder
    HorizontalCylinderZ,
}
