//! 最大装载重量模型 / Max load weight model
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 最大装载重量 / Max load weight (对齐 Kotlin MaxLoadWeight)
#[derive(Debug, Clone)]
pub struct MaxLoadWeight {
    /// 舱位标识 / Position identifier
    pub position_id: String,
    /// 最大重量 / Maximum weight
    pub max_weight: Quantity<f64, Unit>,
}
