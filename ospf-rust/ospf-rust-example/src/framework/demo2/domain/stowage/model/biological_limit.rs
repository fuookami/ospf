//! 生物限制模型 / Biological limit model
/// 生物限制 / Biological limit (对齐 Kotlin BiologicalLimit)
#[derive(Debug, Clone)]
pub struct BiologicalLimit {
    /// 物品标识 / Item identifier
    pub item_id: String,
    /// 邻接物品标识列表 / Adjacent item identifiers
    pub adjacent_items: Vec<String>,
}
