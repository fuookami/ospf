//! Renderer DTO / 渲染器 DTO
//!
//! 渲染器数据传输对象，通过 `serde` feature 开启序列化。
//! Renderer data transfer objects, with serialization enabled via `serde` feature.

use ospf_rust_math::geometry::Axis3;

/// 渲染形状类型 / Render shape type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RenderShapeType {
    /// 长方体 / Cuboid
    Cuboid,
    /// 圆柱 / Cylinder
    Cylinder,
}

/// 渲染算法形状类型 / Render algorithm shape type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RenderAlgorithmShapeType {
    /// 长方体 / Cuboid
    Cuboid,
    /// 竖直圆柱 / Vertical cylinder
    VerticalCylinder,
    /// X 轴横向圆柱 / X-axis horizontal cylinder
    HorizontalCylinderX,
    /// Z 轴横向圆柱 / Z-axis horizontal cylinder
    HorizontalCylinderZ,
}

/// 渲染三维轴 / Render 3D axis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RenderAxis3 {
    /// X 轴 / X axis
    X,
    /// Y 轴 / Y axis
    Y,
    /// Z 轴 / Z axis
    Z,
}

impl From<Axis3> for RenderAxis3 {
    fn from(axis: Axis3) -> Self {
        match axis {
            Axis3::X => Self::X,
            Axis3::Y => Self::Y,
            Axis3::Z => Self::Z,
        }
    }
}

impl From<RenderAxis3> for Axis3 {
    fn from(axis: RenderAxis3) -> Self {
        match axis {
            RenderAxis3::X => Self::X,
            RenderAxis3::Y => Self::Y,
            RenderAxis3::Z => Self::Z,
        }
    }
}

/// 渲染装载项 DTO / Render loading plan item DTO
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RenderLoadingPlanItemDto {
    /// 名称 / Name
    pub name: String,
    /// 包装类型 / Package type
    pub package_type: String,
    /// 宽度 / Width
    pub width: f64,
    /// 高度 / Height
    pub height: f64,
    /// 深度 / Depth
    pub depth: f64,
    /// X 坐标 / X coordinate
    pub x: f64,
    /// Y 坐标 / Y coordinate
    pub y: f64,
    /// Z 坐标 / Z coordinate
    pub z: f64,
    /// 重量 / Weight
    pub weight: f64,
    /// 装载顺序 / Loading order
    pub loading_order: u64,
    /// 形状类型 / Shape type
    pub shape_type: RenderShapeType,
    /// 算法形状类型 / Algorithm shape type
    pub algorithm_shape_type: RenderAlgorithmShapeType,
    /// 半径（圆柱专用）/ Radius (cylinder only)
    pub radius: Option<f64>,
    /// 对齐轴（圆柱专用）/ Alignment axis (cylinder only)
    pub axis: Option<RenderAxis3>,
    /// 包围宽度 / Bounding width
    pub bounding_width: f64,
    /// 包围高度 / Bounding height
    pub bounding_height: f64,
    /// 包围深度 / Bounding depth
    pub bounding_depth: f64,
    /// 实际体积 / Actual volume
    pub actual_volume: f64,
    /// 附加信息 / Additional info
    pub info: Option<String>,
}

/// 渲染装载计划 DTO / Render loading plan DTO
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RenderLoadingPlanDto {
    /// 分组 / Group
    pub group: String,
    /// 名称 / Name
    pub name: String,
    /// 类型编码 / Type code
    pub type_code: String,
    /// 宽度 / Width
    pub width: f64,
    /// 高度 / Height
    pub height: f64,
    /// 深度 / Depth
    pub depth: f64,
    /// 装载率 / Loading rate
    pub loading_rate: f64,
    /// 重量 / Weight
    pub weight: f64,
    /// 体积 / Volume
    pub volume: f64,
    /// 装载项列表 / Loading items
    pub items: Vec<RenderLoadingPlanItemDto>,
    /// 附加信息 / Additional info
    pub info: Option<String>,
}

/// 渲染方案 DTO / Schema DTO
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchemaDto {
    /// KPI 数据 / KPI data (JSON value, requires serde feature)
    #[cfg(feature = "serde_json")]
    pub kpi: Option<serde_json::Value>,
    /// 装载计划列表 / Loading plans
    pub loading_plans: Vec<RenderLoadingPlanDto>,
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_axis3_roundtrip() {
        assert_eq!(Axis3::from(RenderAxis3::from(Axis3::X)), Axis3::X);
        assert_eq!(Axis3::from(RenderAxis3::from(Axis3::Y)), Axis3::Y);
        assert_eq!(Axis3::from(RenderAxis3::from(Axis3::Z)), Axis3::Z);
    }

    #[test]
    fn render_loading_plan_dto_construction() {
        let item = RenderLoadingPlanItemDto {
            name: "item1".to_string(),
            package_type: "box".to_string(),
            width: 2.0,
            height: 3.0,
            depth: 4.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            weight: 1.0,
            loading_order: 1,
            shape_type: RenderShapeType::Cuboid,
            algorithm_shape_type: RenderAlgorithmShapeType::Cuboid,
            radius: None,
            axis: None,
            bounding_width: 2.0,
            bounding_height: 3.0,
            bounding_depth: 4.0,
            actual_volume: 24.0,
            info: None,
        };

        let plan = RenderLoadingPlanDto {
            group: "g1".to_string(),
            name: "bin1".to_string(),
            type_code: "BIN".into(),
            width: 10.0,
            height: 10.0,
            depth: 10.0,
            loading_rate: 0.5,
            weight: 1.0,
            volume: 1000.0,
            items: vec![item],
            info: None,
        };

        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].actual_volume, 24.0);
    }
}
