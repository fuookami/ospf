//! 航班模型 / Flight model
/// 航班 / Flight (stowage domain, 对齐 Kotlin Flight)
#[derive(Debug, Clone)]
pub struct Flight {
    /// 航班标识 / Flight identifier
    pub id: String,
    /// 航班名称 / Flight name
    pub name: String,
    /// 出发站 / Departure station
    pub dep: String,
    /// 到达站 / Arrival station
    pub arr: String,
}
