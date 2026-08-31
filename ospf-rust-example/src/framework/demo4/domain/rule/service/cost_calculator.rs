//! 成本计算器 / Cost calculator
/// 对齐 Kotlin CostCalculator
pub struct CostCalculator;

impl CostCalculator {
    /// 根据飞行时长和单位成本计算总成本 / Calculate total cost based on flight duration and cost per hour
    pub fn calculate(&self, _task_id: &str, duration_hours: f64, cost_per_hour: f64) -> f64 {
        duration_hours * cost_per_hour
    }
}
