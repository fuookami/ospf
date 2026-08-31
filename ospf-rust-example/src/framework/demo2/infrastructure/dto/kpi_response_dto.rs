use std::collections::HashMap;

/// KPI 响应 DTO / KPI response DTO
/// 对齐 Kotlin KPIResponseDTO
#[derive(Debug, Clone)]
pub struct KpiResponseDto {
    pub kpi: HashMap<String, String>,
}
