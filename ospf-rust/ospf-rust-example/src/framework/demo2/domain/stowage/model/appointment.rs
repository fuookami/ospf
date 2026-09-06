//! 指定舱位模型 / Appointment model
/// 预约 / Appointment (对齐 Kotlin Appointment)
#[derive(Debug, Clone)]
pub struct Appointment {
    /// 物品标识 / Item identifier
    pub item_id: String,
    /// 舱位标识 / Position identifier
    pub position_id: String,
}
