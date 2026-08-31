use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 装载方案 / Stowage solution (对齐 Kotlin Solution)
#[derive(Debug, Clone)]
pub struct Solution {
    pub assignments: Vec<(String, String)>, // (item_id, position_id)
    pub total_cost: Quantity<f64, Unit>,
}
