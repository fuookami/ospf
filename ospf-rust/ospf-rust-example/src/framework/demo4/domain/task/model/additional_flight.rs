//! 加飞航班模块 / Additional flight module

use super::flight_leg::FlightLegPlan;

/// 加飞 / Additional flight (对齐 Kotlin AdditionalFlight)
#[derive(Debug, Clone)]
pub struct AdditionalFlight {
    /// 加飞航班计划 / Additional flight plan
    pub plan: FlightLegPlan,
}

impl AdditionalFlight {
    /// 创建新的加飞航班 / Create a new additional flight
    pub fn new(plan: FlightLegPlan) -> Self {
        Self { plan }
    }
}
