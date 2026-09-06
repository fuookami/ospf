//! 推荐重量均衡领域模型 / Recommended weight equalization domain model.
use std::collections::HashMap;

/// 优先预约 / Priority appointment
/// 对齐 Kotlin PriorityAppointment - 映射 CargoPriority 到 loading order depths
#[derive(Debug, Clone)]
pub struct PriorityAppointment {
    /// 货物优先级到装载顺序深度的映射 / Mapping from cargo priority to loading order depths
    /// key: 货物ID, value: 装载顺序深度集合
    pub cargo_priority_depths: HashMap<String, Vec<u8>>,
}

impl PriorityAppointment {
    /// 创建新的优先预约 / Create a new priority appointment
    pub fn new() -> Self {
        Self {
            cargo_priority_depths: HashMap::new(),
        }
    }
}
