//! KPI 响应数据传输对象 / KPI response data transfer object
use std::collections::HashMap;

/// KPI 响应 DTO / KPI response DTO
///
/// 对齐 Kotlin KPIResponseDTO / Aligned with Kotlin KPIResponseDTO
#[derive(Debug, Clone)]
pub struct KpiResponseDto {
    /// KPI 键值对映射 / KPI key-value mapping
    pub kpi: HashMap<String, String>,
}
