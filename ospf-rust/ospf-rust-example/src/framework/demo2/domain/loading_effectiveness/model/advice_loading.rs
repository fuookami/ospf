//! 建议装载模型 / Advice loading model
/// 建议装载 / Advice loading (对齐 Kotlin AdviceLoading)
#[derive(Debug, Clone)]
pub struct AdviceLoading {
    /// 物品标识 / Item identifier
    pub item_id: String,
    /// 舱位标识 / Position identifier
    pub position_id: String,
    /// 优先级 / Priority
    pub priority: u32,
}
