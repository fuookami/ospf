//! 绝对顺序模型 / Absolute order model
use std::collections::HashMap;

/// 绝对顺序 / Absolute order (对齐 Kotlin AbsoluteOrder)
#[derive(Debug, Clone)]
pub struct AbsoluteOrder {
    /// 优先级到位置系数的映射 / Priority to position-coefficient mapping
    pub coefficients: HashMap<String, HashMap<String, f64>>, // priority -> (position_id -> coefficient)
}

impl AbsoluteOrder {
    /// 获取指定优先级和位置的系数 / Get coefficient for given priority and position
    pub fn coefficient(&self, priority: &str, position_id: &str) -> f64 {
        self.coefficients
            .get(priority)
            .and_then(|m| m.get(position_id))
            .copied()
            .unwrap_or(1.0)
    }
}
