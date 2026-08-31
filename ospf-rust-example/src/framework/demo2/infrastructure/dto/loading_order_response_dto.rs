/// 装载顺序响应 DTO / Loading order response DTO
/// 对齐 Kotlin LoadingOrderResponseDTO
#[derive(Debug, Clone)]
pub struct LoadingOrderResponseDto {
    pub status: String,
    pub orders: Vec<String>,
    pub notes: Vec<String>,
}
