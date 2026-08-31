use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 横向平衡 / Lateral balance (对齐 Kotlin LateralBalance)
#[derive(Debug, Clone)]
pub struct LateralBalance {
    pub moment: Quantity<f64, Unit>,
    pub max_imbalance: Quantity<f64, Unit>,
    pub in_range: bool,
}
