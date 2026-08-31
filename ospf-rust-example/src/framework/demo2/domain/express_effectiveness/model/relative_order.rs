use std::collections::HashMap;

/// 相对顺序 / Relative order (对齐 Kotlin RelativeOrder)
#[derive(Debug, Clone)]
pub struct RelativeOrder {
    pub precedence: HashMap<String, Vec<String>>, // item_id -> [must_load_before items]
}
