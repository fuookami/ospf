//! 推荐重量均衡聚合 / Recommended weight equalization aggregation
/// 推荐重量均衡聚合 / Recommended weight equalization aggregation
/// 对齐 Kotlin recommended_weight_equalization Aggregation
#[derive(Debug)]
pub struct Aggregation {
    /// 优先预约列表 / Priority appointment list
    pub appointments: Vec<super::model::PriorityAppointment>,
}
