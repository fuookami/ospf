use std::collections::HashMap;

/// 优先预约 / Priority appointment
/// 对齐 Kotlin PriorityAppointment - 映射 CargoPriority 到 loading order depths
#[derive(Debug, Clone)]
pub struct PriorityAppointment {
    /// 货物优先级到装载顺序深度的映射
    /// key: 货物ID, value: 装载顺序深度集合
    pub cargo_priority_depths: HashMap<String, Vec<u8>>,
}

impl PriorityAppointment {
    pub fn new() -> Self {
        Self {
            cargo_priority_depths: HashMap::new(),
        }
    }
}
