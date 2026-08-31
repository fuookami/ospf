use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 最大装载重量 / Max load weight (对齐 Kotlin MaxLoadWeight)
#[derive(Debug, Clone)]
pub struct MaxLoadWeight {
    pub position_id: String,
    pub max_weight: Quantity<f64, Unit>,
}
