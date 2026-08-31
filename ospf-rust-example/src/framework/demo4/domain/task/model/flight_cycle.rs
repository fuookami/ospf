//! 飞行循环模块 / Flight cycle module

use time::{Duration, OffsetDateTime};

/// 飞行小时 / Flight hour
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FlightHour {
    /// 小时数 / Duration in hours
    pub hours: Duration,
}

impl FlightHour {
    /// 零值 / Zero value
    pub const ZERO: Self = Self { hours: Duration::ZERO };

    /// 创建新的飞行小时 / Create a new flight hour
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
    /// 循环次数 / Number of cycles
    pub cycles: u64,
}

impl FlightCycle {
    /// 零值 / Zero value
    pub const ZERO: Self = Self { cycles: 0 };

    /// 创建新的飞行循环 / Create a new flight cycle
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
    /// 到期时间 / Expiration time
    pub expiration_time: OffsetDateTime,
    /// 剩余飞行小时 / Remaining flight hours
    pub remaining_flight_hour: Option<FlightHour>,
    /// 剩余飞行循环 / Remaining flight cycles
    pub remaining_flight_cycle: Option<FlightCycle>,
}

impl FlightCyclePeriod {
    /// 判断飞行小时是否在剩余限制内 / Check if flight hour is within remaining limit
    pub fn enabled_flight_hour(&self, flight_hour: &FlightHour) -> bool {
        self.remaining_flight_hour
            .as_ref()
            .map(|remaining| flight_hour <= remaining)
            .unwrap_or(true)
    }

    /// 判断飞行循环是否在剩余限制内 / Check if flight cycle is within remaining limit
    pub fn enabled_flight_cycle(&self, flight_cycle: &FlightCycle) -> bool {
        self.remaining_flight_cycle
            .as_ref()
            .map(|remaining| flight_cycle <= remaining)
            .unwrap_or(true)
    }

    /// 计算超出剩余飞行小时的部分 / Compute the amount exceeding remaining flight hours
    pub fn over_flight_hour(&self, flight_hour: &FlightHour) -> FlightHour {
        if let Some(remaining) = &self.remaining_flight_hour {
            if remaining < flight_hour {
                return flight_hour.clone() - remaining.clone();
            }
        }
        FlightHour::ZERO
    }

    /// 计算超出剩余飞行循环的部分 / Compute the amount exceeding remaining flight cycles
    pub fn over_flight_cycle(&self, flight_cycle: &FlightCycle) -> FlightCycle {
        if let Some(remaining) = &self.remaining_flight_cycle {
            if remaining <= flight_cycle {
                return flight_cycle.clone() - remaining.clone();
            }
        }
        FlightCycle::ZERO
    }
}
