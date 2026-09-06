//! 渲染数据传输对象 / Render data transfer object
use std::collections::HashMap;

/// 渲染 DTO / Render DTO
///
/// 对齐 Kotlin RenderDTO / Aligned with Kotlin RenderDTO
#[derive(Debug, Clone)]
pub struct RenderDto {
    /// 装载分配列表 / Assignment list for rendering
    pub assignments: Vec<RenderAssignmentDto>,
    /// KPI 键值对映射 / KPI key-value mapping
    pub kpi: HashMap<String, String>,
}

/// 渲染分配 DTO / Render assignment DTO
#[derive(Debug, Clone)]
pub struct RenderAssignmentDto {
    /// 货物名称 / Cargo name
    pub cargo: String,
    /// 货舱位置名称 / Position name
    pub position: String,
}
