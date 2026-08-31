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
    pub items: Vec<String>,
}
