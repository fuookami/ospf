//! 拖车模型 / Trailer model
/// 拖车类型 / Trailer type (对齐 Kotlin TrailerType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrailerType {
    /// 停机坪拖车 / Hardstand trailer
    Hardstand,
    /// 过境拖车 / Transit trailer
    Transit,
    /// 仓库拖车 / Warehouse trailer
    Warehouse,
}

/// 拖车 / Trailer (对齐 Kotlin Trailer)
#[derive(Debug, Clone)]
pub struct Trailer {
    /// 拖车类型 / Trailer type
    pub trailer_type: TrailerType,
    /// 拖车顺序 / Trailer order
    pub order: u8,
    /// 拖车名称 / Trailer name
    pub name: String,
    /// 拖车上的物品列表 / Items on the trailer
    pub items: Vec<String>,
}
