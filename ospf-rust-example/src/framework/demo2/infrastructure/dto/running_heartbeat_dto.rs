/// 运行心跳 DTO / Running heartbeat DTO
/// 对齐 Kotlin RunningHeartBeatDTO
#[derive(Debug, Clone)]
pub struct RunningHeartBeatDto {
    pub task_id: String,
    pub progress: f64,
    pub status: String,
}
