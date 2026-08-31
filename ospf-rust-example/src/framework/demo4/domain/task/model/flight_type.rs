use super::airport::AirportType;

/// 航班类型 / Flight type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FlightType {
    Domestic,
    Regional,
    International,
}

impl FlightType {
    pub fn is_domain_type(&self) -> bool {
        matches!(self, FlightType::Domestic)
    }

    pub fn from_airport_types(dep: AirportType, arr: AirportType) -> Self {
        match dep.max(arr) {
            AirportType::Domestic => FlightType::Domestic,
            AirportType::Regional => FlightType::Regional,
            AirportType::International => FlightType::International,
        }
    }
}
