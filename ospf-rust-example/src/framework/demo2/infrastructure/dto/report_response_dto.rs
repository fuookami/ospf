/// 报告响应 DTO / Report response DTO
/// 对齐 Kotlin ReportResponseDTO
#[derive(Debug, Clone)]
pub struct ReportResponseDto {
    pub status: String,
    pub report: String,
}
