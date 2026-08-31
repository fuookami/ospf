/// 顺序装载 / Sequential loading (对齐 Kotlin SequentialLoading)
#[derive(Debug, Clone)]
pub struct SequentialLoading {
    pub item_id: String,
    pub position_id: String,
    pub order: u32,
}
