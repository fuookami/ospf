/// 拖车类型 / Trailer type (对齐 Kotlin TrailerType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrailerType {
    Hardstand,
    Transit,
    Warehouse,
}

/// 拖车 / Trailer (对齐 Kotlin Trailer)
#[derive(Debug, Clone)]
pub struct Trailer {
    pub trailer_type: TrailerType,
    pub order: u8,
    pub name: String,
    pub items: Vec<String>, // item IDs
}

/// 建议装载 / Advice loading (对齐 Kotlin AdviceLoading)
#[derive(Debug, Clone)]
pub struct AdviceLoading {
    pub item_id: String,
    pub position_id: String,
    pub priority: u32,
}

/// 顺序装载 / Sequential loading (对齐 Kotlin SequentialLoading)
#[derive(Debug, Clone)]
pub struct SequentialLoading {
    pub item_id: String,
    pub position_id: String,
    pub order: u32,
}

/// 拖车装载 / Trailer loading (对齐 Kotlin TrailerLoading)
#[derive(Debug, Clone)]
pub struct TrailerLoading {
    pub trailer: Trailer,
    pub position_id: String,
}

/// 转运邻接装载 / Transfer adjacent loading (对齐 Kotlin TransferAdjacentLoading)
#[derive(Debug, Clone)]
pub struct TransferAdjacentLoading {
    pub item_id: String,
    pub adjacent_position_id: String,
}
