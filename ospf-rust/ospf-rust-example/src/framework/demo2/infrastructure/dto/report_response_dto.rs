//! 报告响应数据传输对象 / Report response data transfer object
/// 报告响应 DTO / Report response DTO
///
/// 对齐 Kotlin ReportResponseDTO / Aligned with Kotlin ReportResponseDTO
#[derive(Debug, Clone)]
pub struct ReportResponseDto {
    /// 求解状态 / Solve status
    pub status: String,
    /// 报告内容 / Report content
    pub report: String,
}
