//! 实验性纵向平衡模型 / Experimental longitudinal balance model
/// 实验性纵向平衡 / Experimental longitudinal balance (对齐 Kotlin ExperimentalLongitudinalBalance)
#[derive(Debug, Clone)]
pub struct ExperimentalLongitudinalBalance {
    /// 平均空气动力弦位置值 / Mean aerodynamic chord position value
    pub mac_value: f64,
    /// 最小平均空气动力弦位置 / Minimum MAC position
    pub min_mac: f64,
    /// 最大平均空气动力弦位置 / Maximum MAC position
    pub max_mac: f64,
}

impl ExperimentalLongitudinalBalance {
    /// 判断纵向平衡是否在允许范围内 / Check if longitudinal balance is within allowed range
    pub fn in_range(&self) -> bool {
        self.mac_value >= self.min_mac && self.mac_value <= self.max_mac
    }
}
