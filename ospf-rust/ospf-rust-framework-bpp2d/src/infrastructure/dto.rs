//! 渲染 DTO / Render DTOs
//!
//! 与 Kotlin `RendererDTO.kt` 对齐：板材、层与放置的渲染输出。启用 `serde` 时使用 `camelCase`
//! 字段名，与 Kotlin `@Serializable` 的输出口径一致。
//! Mirrors Kotlin `RendererDTO.kt`: render output for sheets, layers and placements. With `serde` enabled
//! the fields use `camelCase`, matching the Kotlin `@Serializable` output convention.

use ospf_rust_framework_geometry2d::PackingShape2Type;

// ============================================================================
// RenderShape2Type - 渲染形状类型 / Render shape type
// ============================================================================

/// 渲染形状类型 / Render shape type
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderShape2Type {
    /// 矩形 / Rectangle
    Rectangle,
    /// 圆形 / Circle
    Circle,
}

impl From<PackingShape2Type> for RenderShape2Type {
    fn from(value: PackingShape2Type) -> Self {
        match value {
            PackingShape2Type::Rectangle => Self::Rectangle,
            PackingShape2Type::Circle => Self::Circle,
        }
    }
}

// ============================================================================
// RenderPlacementDto - 放置渲染 DTO / Placement render DTO
// ============================================================================

/// 放置渲染 DTO / Placement render DTO
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone, PartialEq)]
pub struct RenderPlacementDto {
    /// 物品编号 / Item identifier
    pub item_id: Option<String>,
    /// 左下角 x 坐标 / Lower-left x coordinate
    pub x: f64,
    /// 左下角 y 坐标 / Lower-left y coordinate
    pub y: f64,
    /// 包围宽度 / Bounding width
    pub width: f64,
    /// 包围高度 / Bounding height
    pub height: f64,
    /// 半径（圆形件）/ Radius (circle items)
    pub radius: Option<f64>,
    /// 形状类型 / Shape type
    pub shape_type: RenderShape2Type,
    /// 是否旋转 90 度 / Whether rotated by 90 degrees
    pub rotated: bool,
}

// ============================================================================
// RenderLayerDto - 层渲染 DTO / Layer render DTO
// ============================================================================

/// 层渲染 DTO / Layer render DTO
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone, PartialEq)]
pub struct RenderLayerDto {
    /// 生成来源 / Generation source
    pub source: String,
    /// 所属板材类型编码 / Owning sheet type code
    pub sheet_type_code: Option<String>,
    /// 包围宽度 / Bounding width
    pub width: f64,
    /// 包围高度 / Bounding height
    pub height: f64,
    /// 实际占用面积 / Actually occupied area
    pub area: f64,
    /// 件数 / Amount
    pub amount: u64,
    /// 放置列表 / Placement list
    pub placements: Vec<RenderPlacementDto>,
}

// ============================================================================
// RenderSheetDto - 板材渲染 DTO / Sheet render DTO
// ============================================================================

/// 板材渲染 DTO / Sheet render DTO
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone, PartialEq)]
pub struct RenderSheetDto {
    /// 板材类型编码 / Sheet type code
    pub sheet_type_code: String,
    /// 板材宽度 / Sheet width
    pub width: f64,
    /// 板材高度 / Sheet height
    pub height: f64,
    /// 板材面积 / Sheet area
    pub area: f64,
    /// 装载率 / Loading rate
    pub loading_rate: f64,
    /// 本次使用的层 / Layers used in this sheet
    pub layers: Vec<RenderLayerDto>,
}

// ============================================================================
// RenderLoadingPlanDto - 装载方案渲染 DTO / Loading plan render DTO
// ============================================================================

/// 装载方案渲染 DTO / Loading plan render DTO
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Debug, Clone, PartialEq)]
pub struct RenderLoadingPlanDto {
    /// 板材列表 / Sheet list
    pub sheets: Vec<RenderSheetDto>,
    /// 总件数 / Total amount
    pub total_amount: u64,
    /// 总面积 / Total area
    pub total_area: f64,
    /// 总余料面积 / Total waste area
    pub total_waste_area: f64,
}
