use time::{Duration, OffsetDateTime};

/// 飞行小时 / Flight hour
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FlightHour {
    pub hours: Duration,
}

impl FlightHour {
    pub const ZERO: Self = Self { hours: Duration::ZERO };

    pub fn new(hours: Duration) -> Self {
        Self { hours }
    }
}

impl std::ops::Add for FlightHour {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self { hours: self.hours + rhs.hours }
    }
}

impl std::ops::Sub for FlightHour {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self { hours: self.hours - rhs.hours }
    }
}

/// 飞行循环 / Flight cycle
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FlightCycle {
    pub cycles: u64,
}

impl FlightCycle {
    pub const ZERO: Self = Self { cycles: 0 };

    pub fn new(cycles: u64) -> Self {
        Self { cycles }
    }
}

impl std::ops::Add for FlightCycle {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self { cycles: self.cycles + rhs.cycles }
    }
}

impl std::ops::Sub for FlightCycle {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self { cycles: self.cycles.saturating_sub(rhs.cycles) }
    }
}

/// 飞行循环周期 / Flight cycle period
#[derive(Debug, Clone)]
pub struct FlightCyclePeriod {
    pub expiration_time: OffsetDateTime,
    pub remaining_flight_hour: Option<FlightHour>,
    pub remaining_flight_cycle: Option<FlightCycle>,
}

impl FlightCyclePeriod {
    pub fn enabled_flight_hour(&self, flight_hour: &FlightHour) -> bool {
        self.remaining_flight_hour
            .as_ref()
            .map(|remaining| flight_hour <= remaining)
            .unwrap_or(true)
    }

    pub fn enabled_flight_cycle(&self, flight_cycle: &FlightCycle) -> bool {
        self.remaining_flight_cycle
            .as_ref()
            .map(|remaining| flight_cycle <= remaining)
            .unwrap_or(true)
    }

    pub fn over_flight_hour(&self, flight_hour: &FlightHour) -> FlightHour {
        if let Some(remaining) = &self.remaining_flight_hour {
            if remaining < flight_hour {
                return flight_hour.clone() - remaining.clone();
            }
        }
        FlightHour::ZERO
    }

    pub fn over_flight_cycle(&self, flight_cycle: &FlightCycle) -> FlightCycle {
        if let Some(remaining) = &self.remaining_flight_cycle {
            if remaining <= flight_cycle {
                return flight_cycle.clone() - remaining.clone();
            }
        }
        FlightCycle::ZERO
    }
}
