/// 业载最大化聚合 / Payload maximization aggregation
/// 对齐 Kotlin payload_maximization Aggregation
#[derive(Debug)]
pub struct Aggregation {
    pub aircraft_model: super::super::aircraft::model::AircraftModel,
    pub payload_estimate: f64,
}
