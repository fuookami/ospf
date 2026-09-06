//! 横向平衡模型 / Lateral balance model
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 横向平衡 / Lateral balance (对齐 Kotlin LateralBalance)
#[derive(Debug, Clone)]
pub struct LateralBalance {
    /// 横向力矩 / Lateral moment
    pub moment: Quantity<f64, Unit>,
    /// 最大不平衡量 / Maximum imbalance
    pub max_imbalance: Quantity<f64, Unit>,
    /// 是否在安全范围内 / Whether within safe range
    pub in_range: bool,
}
