use super::flight_leg::FlightLegPlan;

/// 加飞 / Additional flight (对齐 Kotlin AdditionalFlight)
#[derive(Debug, Clone)]
pub struct AdditionalFlight {
    pub plan: FlightLegPlan,
}

impl AdditionalFlight {
    pub fn new(plan: FlightLegPlan) -> Self {
        Self { plan }
    }
}
