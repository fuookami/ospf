//! 装载顺序响应数据传输对象 / Loading order response data transfer object
/// 装载顺序响应 DTO / Loading order response DTO
///
/// 对齐 Kotlin LoadingOrderResponseDTO / Aligned with Kotlin LoadingOrderResponseDTO
#[derive(Debug, Clone)]
pub struct LoadingOrderResponseDto {
    /// 求解状态 / Solve status
    pub status: String,
    /// 装载顺序列表 / Loading order list
    pub orders: Vec<String>,
    /// 备注信息 / Notes
    pub notes: Vec<String>,
}
