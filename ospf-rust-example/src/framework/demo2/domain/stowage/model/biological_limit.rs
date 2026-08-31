/// 生物限制 / Biological limit (对齐 Kotlin BiologicalLimit)
#[derive(Debug, Clone)]
pub struct BiologicalLimit {
    pub item_id: String,
    pub adjacent_items: Vec<String>,
}
