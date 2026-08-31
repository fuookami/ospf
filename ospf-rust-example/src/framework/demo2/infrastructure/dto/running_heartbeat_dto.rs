//! 运行心跳数据传输对象 / Running heartbeat data transfer object
/// 运行心跳 DTO / Running heartbeat DTO
///
/// 对齐 Kotlin RunningHeartBeatDTO / Aligned with Kotlin RunningHeartBeatDTO
#[derive(Debug, Clone)]
pub struct RunningHeartBeatDto {
    /// 任务标识 / Task identifier
    pub task_id: String,
    /// 进度百分比 / Progress percentage (0.0 ~ 1.0)
    pub progress: f64,
    /// 运行状态 / Running status
    pub status: String,
}
