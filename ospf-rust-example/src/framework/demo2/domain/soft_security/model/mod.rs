//! 软安全领域模型 / Soft security domain model.
/// 分离空装载 / Divide empty loading (对齐 Kotlin DivideEmptyLoading)
#[derive(Debug, Clone)]
pub struct DivideEmptyLoading {
    /// 货物标识 / Item identifier
    pub item_id: String,
    /// 舱位标识 / Position identifier
    pub position_id: String,
    /// 空载比率 / Empty loading ratio
    pub empty_ratio: f64,
}
