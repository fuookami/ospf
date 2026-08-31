use std::collections::HashMap;

/// 渲染 DTO / Render DTO
/// 对齐 Kotlin RenderDTO
#[derive(Debug, Clone)]
pub struct RenderDto {
    pub assignments: Vec<RenderAssignmentDto>,
    pub kpi: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct RenderAssignmentDto {
    pub cargo: String,
    pub position: String,
}
