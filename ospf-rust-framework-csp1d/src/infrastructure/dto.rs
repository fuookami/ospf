//! 渲染 DTO / Render DTO

use std::collections::BTreeMap;

/// 生产类型 / Production type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RenderProductionType {
    Product,
    Costar,
}

/// 切割方案生产 DTO / Cutting plan production DTO
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RenderCuttingPlanProductionDTO {
    pub name: String,
    pub x: String,
    pub id: String,
    pub width: String,
    pub unit_length: Option<String>,
    pub production_type: RenderProductionType,
    pub amount: u64,
    pub info: BTreeMap<String, String>,
}

/// 切割方案 DTO / Cutting plan DTO
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RenderCuttingPlanDTO {
    pub group: Vec<String>,
    pub id: String,
    pub material_id: String,
    pub amount: u64,
    pub productions: Vec<RenderCuttingPlanProductionDTO>,
    pub width: String,
    pub standard_width: String,
    pub rest_width: Option<String>,
    pub info: BTreeMap<String, String>,
}

/// 方案渲染根对象 / Render schema root
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RenderSchemaDTO {
    pub kpi: BTreeMap<String, String>,
    pub cutting_plans: Vec<RenderCuttingPlanDTO>,
}
