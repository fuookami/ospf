//! 服务模型 / Service model

/// 服务模型 / Service model (对齐 Kotlin Service)
#[derive(Debug, Clone)]
pub struct Service {
    /// 服务标识 / Service identifier
    pub id: u64,
    /// 服务容量 / Service capacity
    pub capacity: f64,
    /// 服务成本 / Service cost
    pub cost: f64,
}
