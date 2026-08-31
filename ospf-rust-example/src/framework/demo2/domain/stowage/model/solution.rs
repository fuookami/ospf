//! 装载方案模型 / Solution model
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 装载方案 / Stowage solution (对齐 Kotlin Solution)
#[derive(Debug, Clone)]
pub struct Solution {
    /// 分配列表 (物品标识, 舱位标识) / Assignment list (item_id, position_id)
    pub assignments: Vec<(String, String)>, // (item_id, position_id)
    /// 总成本 / Total cost
    pub total_cost: Quantity<f64, Unit>,
}
