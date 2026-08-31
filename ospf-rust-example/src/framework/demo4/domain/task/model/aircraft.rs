//! 飞机模型模块 / Aircraft model module

use time::{Duration, OffsetDateTime};
use ospf_rust_framework_gantt_scheduling::domain::task::ExecutorTrait;
use crate::framework::demo4::infrastructure::{AircraftRegisterNumber, PassengerClass};
use super::aircraft_type::AircraftMinorType;
use super::airport::Airport;
use super::flight_cycle::FlightCyclePeriod;

/// 飞机容量 / Aircraft capacity
#[derive(Debug, Clone)]
pub enum AircraftCapacity {
    /// 客机容量 / Passenger aircraft capacity
    Passenger {
        /// 各舱位等级容量列表 / Capacity per passenger class
        capacities: Vec<(PassengerClass, u64)>,
    },
    /// 货机容量 / Cargo aircraft capacity
    Cargo {
        /// 最大载重 / Maximum cargo weight
        max_weight: f64,
    },
}

/// 飞机 / Aircraft
#[derive(Debug, Clone)]
pub struct Aircraft {
    /// 注册号 / Registration number
    pub reg_no: AircraftRegisterNumber,
    /// 飞机子类型 / Aircraft minor type
    pub minor_type: AircraftMinorType,
    /// 容量 / Capacity
    pub capacity: AircraftCapacity,
}

impl Aircraft {
    /// 获取飞机类型代码 / Get aircraft type code
    pub fn type_code(&self) -> &super::aircraft_type::AircraftType {
        &self.minor_type.aircraft_type
    }

    /// 获取每小时成本 / Get cost per flight hour
    pub fn cost_per_hour(&self) -> f64 {
        self.minor_type.cost_per_hour
    }
}

impl PartialEq for Aircraft {
    fn eq(&self, other: &Self) -> bool {
        self.reg_no == other.reg_no
    }
}
impl Eq for Aircraft {}

impl std::hash::Hash for Aircraft {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.reg_no.hash(state);
    }
}

/// 实现 ExecutorTrait for Aircraft
impl ExecutorTrait for Aircraft {
    type Id = String;

    fn id(&self) -> &Self::Id {
        &self.reg_no.0
    }

    fn name(&self) -> &str {
        &self.reg_no.0
    }
}

/// 飞机可用性 / Aircraft usability
#[derive(Debug, Clone)]
pub struct AircraftUsability {
    /// 当前所在机场 / Current location airport
    pub location: Airport,
    /// 可用时间 / Enabled time
    pub enabled_time: OffsetDateTime,
    /// 飞行循环周期列表 / Flight cycle periods
    pub flight_cycle_periods: Vec<FlightCyclePeriod>,
}
