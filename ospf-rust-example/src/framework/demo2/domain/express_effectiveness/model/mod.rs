use std::collections::HashMap;

/// 货物优先级 / Cargo priority (对齐 Kotlin CargoPriority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CargoPriority {
    High,
    Medium,
    Low,
}

/// 绝对顺序 / Absolute order (对齐 Kotlin AbsoluteOrder)
#[derive(Debug, Clone)]
pub struct AbsoluteOrder {
    pub coefficients: HashMap<CargoPriority, HashMap<String, f64>>, // priority -> (position_id -> coefficient)
}

impl AbsoluteOrder {
    pub fn coefficient(&self, priority: CargoPriority, position_id: &str) -> f64 {
        self.coefficients
            .get(&priority)
            .and_then(|m| m.get(position_id))
            .copied()
            .unwrap_or(1.0)
    }
}

/// 相对顺序 / Relative order (对齐 Kotlin RelativeOrder)
#[derive(Debug, Clone)]
pub struct RelativeOrder {
    pub precedence: HashMap<String, Vec<String>>, // item_id -> [must_load_before items]
}
