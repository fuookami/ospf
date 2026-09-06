//! 通用网络成本 / Generic network cost.

use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::Unit;
use ospf_rust_quantities::unit::UnitTrait;

/// 允许负值的通用网络弧成本 / Generic network-arc cost; negative values are allowed.
#[derive(Debug, Clone)]
pub struct NetworkCost<V, U: UnitTrait = Unit> {
    /// 成本物理量 / Cost quantity.
    pub quantity: Quantity<V, U>,
}

impl<V, U: UnitTrait> NetworkCost<V, U> {
    /// 创建网络成本 / Create a network cost.
    pub fn new(quantity: Quantity<V, U>) -> Self {
        Self { quantity }
    }
}
