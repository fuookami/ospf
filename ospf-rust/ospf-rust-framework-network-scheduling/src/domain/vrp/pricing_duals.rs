//! VRP 影子价格与定价对偶快照 / VRP shadow prices and immutable pricing-dual snapshots.

use std::collections::BTreeMap;

use super::{CustomerId, VehicleTypeId};

/// 定价阶段 / Pricing phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PricingPhase {
    /// Phase I 人工覆盖目标 / Phase I artificial-coverage objective.
    PhaseOne,
    /// Phase II 真实路线成本目标 / Phase II real-route-cost objective.
    PhaseTwo,
}

/// 与 route compilation 解耦的不可变定价对偶 / Immutable pricing duals decoupled from route compilation.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PricingDuals {
    /// 定价阶段 / Pricing phase.
    pub phase: PricingPhase,
    /// 客户覆盖等式对偶 / Customer-cover equality duals.
    pub customer: BTreeMap<CustomerId, f64>,
    /// 车队上界对偶 / Fleet upper-bound duals.
    pub fleet: BTreeMap<VehicleTypeId, f64>,
}

impl PricingDuals {
    /// 创建对偶快照 / Create a dual snapshot.
    pub fn new(
        phase: PricingPhase,
        customer: impl IntoIterator<Item = (CustomerId, f64)>,
        fleet: impl IntoIterator<Item = (VehicleTypeId, f64)>,
    ) -> Self {
        Self {
            phase,
            customer: customer.into_iter().collect(),
            fleet: fleet.into_iter().collect(),
        }
    }

    /// 获取客户对偶；缺失时为零 / Get a customer dual, defaulting to zero.
    pub fn customer_dual(&self, id: &CustomerId) -> f64 {
        self.customer.get(id).copied().unwrap_or(0.0)
    }

    /// 获取车辆类型对偶；缺失时为零 / Get a fleet dual, defaulting to zero.
    pub fn fleet_dual(&self, id: &VehicleTypeId) -> f64 {
        self.fleet.get(id).copied().unwrap_or(0.0)
    }
}

/// 稳定的 VRP 影子价格表 / Stable VRP shadow-price map.
#[derive(Debug, Clone, Default)]
pub struct VrpShadowPriceMap {
    customer: BTreeMap<CustomerId, f64>,
    fleet: BTreeMap<VehicleTypeId, f64>,
}

impl VrpShadowPriceMap {
    /// 创建空映射 / Create an empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// 写入客户对偶 / Set a customer dual.
    pub fn set_customer_dual(&mut self, id: CustomerId, value: f64) {
        self.customer.insert(id, value);
    }

    /// 写入车队对偶 / Set a fleet dual.
    pub fn set_fleet_dual(&mut self, id: VehicleTypeId, value: f64) {
        self.fleet.insert(id, value);
    }

    /// 读取客户对偶 / Read a customer dual.
    pub fn customer_dual(&self, id: &CustomerId) -> f64 {
        self.customer.get(id).copied().unwrap_or(0.0)
    }

    /// 读取车队对偶 / Read a fleet dual.
    pub fn fleet_dual(&self, id: &VehicleTypeId) -> f64 {
        self.fleet.get(id).copied().unwrap_or(0.0)
    }

    /// 输出不可变对偶快照 / Export an immutable dual snapshot.
    pub fn snapshot(&self, phase: PricingPhase) -> PricingDuals {
        PricingDuals::new(phase, self.customer.clone(), self.fleet.clone())
    }
}
