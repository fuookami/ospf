//! 最大区域载荷重量模型 / Max zone load weight model
/// 最大区域载荷重量 / Max zone load weight (对齐 Kotlin MaxZoneLoadWeight)
#[derive(Debug, Clone)]
pub struct MaxZoneLoadWeight {
    /// 区域标识 / Zone identifier
    pub zone: String,
    /// 最大重量 / Maximum weight
    pub max_weight: f64,
}
