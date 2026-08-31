/// 装载方案 / Stowage solution (对齐 Kotlin Solution)
#[derive(Debug, Clone)]
pub struct Solution {
    pub assignments: Vec<(String, String)>, // (item_id, position_id)
    pub total_cost: f64,
}
