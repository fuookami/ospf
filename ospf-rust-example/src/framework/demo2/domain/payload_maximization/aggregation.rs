//! 载量最大化聚合 / Payload maximization aggregation
/// 业载最大化聚合 / Payload maximization aggregation
/// 对齐 Kotlin payload_maximization Aggregation
#[derive(Debug)]
pub struct Aggregation {
    /// 飞机型号 / Aircraft model
    pub aircraft_model: super::super::aircraft::model::AircraftModel,
    /// 业载估算值 / Payload estimate value
    pub payload_estimate: f64,
}
