use std::collections::HashMap;

/// 绝对顺序 / Absolute order (对齐 Kotlin AbsoluteOrder)
#[derive(Debug, Clone)]
pub struct AbsoluteOrder {
    pub coefficients: HashMap<String, HashMap<String, f64>>, // priority -> (position_id -> coefficient)
}

impl AbsoluteOrder {
    pub fn coefficient(&self, priority: &str, position_id: &str) -> f64 {
        self.coefficients
            .get(priority)
            .and_then(|m| m.get(position_id))
            .copied()
            .unwrap_or(1.0)
    }
}
