//! 飞行任务模型模块 / Flight task model module

use super::aircraft::Aircraft;
use super::airport::Airport;
use ospf_rust_framework_gantt_scheduling::domain::task::{
    AssignmentPolicyTrait, ExecutorTrait, TaskKey, TaskStatus, TaskTrait, TaskType,
};
use ospf_rust_framework_gantt_scheduling::infrastructure::TimeRange;
use std::collections::HashSet;
use time::{Duration, OffsetDateTime};

/// 飞行任务类别 / Flight task category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightTaskCategory {
    /// 航班 / Flight
    Flight,
    /// 虚拟航班 / Virtual flight
    VirtualFlight,
    /// 维护 / Maintenance
    Maintenance,
    /// 飞机停场 / Aircraft on Ground
    AOG,
}

impl FlightTaskCategory {
    /// 是否为航班类型 / Check if this is a flight type category
    pub fn is_flight_type(&self) -> bool {
        matches!(
            self,
            FlightTaskCategory::Flight | FlightTaskCategory::VirtualFlight
        )
    }
}

/// 飞行任务状态 / Flight task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightTaskStatus {
    /// 不允许提前 / No advance allowed
    NotAdvance,
    /// 不允许延误 / No delay allowed
    NotDelay,
    /// 不允许取消 / No cancellation allowed
    NotCancel,
    /// 不允许取消（优先约束） / No cancellation allowed (preferred constraint)
    NotCancelPreferred,
    /// 不允许换飞机 / No aircraft change allowed
    NotAircraftChange,
    /// 不允许换机型 / No aircraft type change allowed
    NotAircraftTypeChange,
    /// 不允许换飞机子类型 / No aircraft minor type change allowed
    NotAircraftMinorTypeChange,
    /// 不允许换航站楼 / No terminal change allowed
    NotTerminalChange,
    /// 强限制可忽略 / Strong limit ignored
    StrongLimitIgnored,
}

/// 飞行任务分配策略 / Flight task assignment policy
#[derive(Debug, Clone, Default)]
pub struct FlightTaskAssignment {
    /// 分配飞机 / Assigned aircraft
    pub aircraft: Option<Aircraft>,
    /// 分配时间 / Assigned time
    pub time: Option<OffsetDateTime>,
    /// 分配航线 / Assigned route
    pub route: Option<super::airport::Route>,
}

impl FlightTaskAssignment {
    /// 分配策略是否为空 / Check if assignment policy is empty
    pub fn is_empty(&self) -> bool {
        self.aircraft.is_none() && self.time.is_none() && self.route.is_none()
    }
}

/// 飞行任务类型 / Flight task type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlightTaskType {
    /// 任务类别 / Task category
    pub category: FlightTaskCategory,
    /// 类型名称 / Type name
    pub name: String,
}

/// 飞行任务变更 / Aircraft change
#[derive(Debug, Clone)]
pub struct AircraftChange {
    /// 原飞机 / Original aircraft
    pub from: Aircraft,
    /// 新飞机 / New aircraft
    pub to: Aircraft,
}

/// 飞机类型变更 / Aircraft type change
#[derive(Debug, Clone)]
pub struct AircraftTypeChange {
    /// 原机型 / Original aircraft type
    pub from: super::aircraft_type::AircraftType,
    /// 新机型 / New aircraft type
    pub to: super::aircraft_type::AircraftType,
}

/// 飞机子类型变更 / Aircraft minor type change
#[derive(Debug, Clone)]
pub struct AircraftMinorTypeChange {
    /// 原子类型 / Original minor type
    pub from: super::aircraft_type::AircraftMinorType,
    /// 新子类型 / New minor type
    pub to: super::aircraft_type::AircraftMinorType,
}

/// 航线变更 / Route change
#[derive(Debug, Clone)]
pub struct RouteChange {
    /// 原航线 / Original route
    pub from: super::airport::Route,
    /// 新航线 / New route
    pub to: super::airport::Route,
}

// ============================================================================
// 框架 trait 实现 / Framework trait implementations
// ============================================================================

/// Demo4 飞行任务类型实现 / Demo4 flight task type implementation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlightTaskTypeImpl {
    /// 任务类别 / Task category
    pub category: FlightTaskCategory,
    /// 类型名称 / Type name
    pub name: String,
}

/// 飞行任务计划 / Flight task plan
#[derive(Debug, Clone)]
pub struct FlightTaskPlanImpl {
    /// 计划标识 / Plan identifier
    pub id: String,
    /// 计划名称 / Plan name
    pub name: String,
    /// 执行飞机 / Executing aircraft
    pub aircraft: Option<Aircraft>,
    /// 出发机场 / Departure airport
    pub dep: Airport,
    /// 到达机场 / Arrival airport
    pub arr: Airport,
    /// 执行时间段 / Execution time range
    pub time: Option<(OffsetDateTime, OffsetDateTime)>,
    /// 计划时间段 / Scheduled time range
    pub scheduled_time: Option<(OffsetDateTime, OffsetDateTime)>,
    /// 飞行任务状态约束 / Flight task status constraints
    pub flight_task_status: Vec<FlightTaskStatus>,
}

/// 飞行任务 / Flight task (实现 TaskTrait)
#[derive(Debug, Clone)]
pub struct FlightTaskImpl {
    /// 飞行任务计划 / Flight task plan
    pub plan: FlightTaskPlanImpl,
    /// 任务类型 / Task type
    pub task_type: FlightTaskTypeImpl,
    /// 恢复用替代飞机 / Recovery replacement aircraft
    pub recovery_aircraft: Option<Aircraft>,
    /// 恢复时间段 / Recovery time range
    pub recovery_time: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl FlightTaskImpl {
    /// 创建新的飞行任务 / Create a new flight task
    pub fn new(plan: FlightTaskPlanImpl, task_type: FlightTaskTypeImpl) -> Self {
        Self {
            plan,
            task_type,
            recovery_aircraft: None,
            recovery_time: None,
        }
    }

    /// 获取执行飞机（恢复或原计划） / Get aircraft (recovery or planned)
    pub fn aircraft(&self) -> Option<&Aircraft> {
        self.recovery_aircraft
            .as_ref()
            .or(self.plan.aircraft.as_ref())
    }

    /// 获取出发机场 / Get departure airport
    pub fn dep(&self) -> &Airport {
        &self.plan.dep
    }

    /// 获取到达机场 / Get arrival airport
    pub fn arr(&self) -> &Airport {
        &self.plan.arr
    }

    /// 获取有效时间（恢复或计划） / Get effective time (recovery or planned)
    pub fn time(&self) -> Option<(OffsetDateTime, OffsetDateTime)> {
        self.recovery_time.or(self.plan.time)
    }

    /// 是否已恢复 / Check if recovered
    pub fn recovered(&self) -> bool {
        self.recovery_aircraft.is_some() || self.recovery_time.is_some()
    }
}

/// 飞行任务分配策略实现 / Flight assignment policy implementation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlightAssignmentPolicy {
    /// 分配的执行飞机 / Assigned executor aircraft
    pub executor: Option<Aircraft>,
    /// 分配的时间范围 / Assigned time range
    pub time: Option<TimeRange>,
}

impl AssignmentPolicyTrait<Aircraft> for FlightAssignmentPolicy {
    fn executor(&self) -> Option<&Aircraft> {
        self.executor.as_ref()
    }

    fn time(&self) -> Option<&TimeRange> {
        self.time.as_ref()
    }
}

// 实现 TaskTrait
impl TaskTrait<Aircraft, FlightAssignmentPolicy> for FlightTaskImpl {
    type Id = String;

    fn id(&self) -> &Self::Id {
        &self.plan.id
    }

    fn name(&self) -> &str {
        &self.plan.name
    }

    fn type_(&self) -> TaskType {
        TaskType::new(&self.task_type.name)
    }

    fn key(&self) -> TaskKey<Self::Id> {
        TaskKey::new(self.plan.id.clone(), self.type_())
    }

    fn time_window(&self) -> Option<&TimeRange> {
        None
    }

    fn scheduled_time(&self) -> Option<&TimeRange> {
        None
    }

    fn duration(&self) -> Option<Duration> {
        self.plan.scheduled_time.map(|(start, end)| end - start)
    }

    fn cancel_enabled(&self) -> bool {
        !self
            .plan
            .flight_task_status
            .contains(&FlightTaskStatus::NotCancel)
    }

    fn delay_enabled(&self) -> bool {
        !self
            .plan
            .flight_task_status
            .contains(&FlightTaskStatus::NotDelay)
    }

    fn advance_enabled(&self) -> bool {
        !self
            .plan
            .flight_task_status
            .contains(&FlightTaskStatus::NotAdvance)
    }

    fn executor(&self) -> Option<&Aircraft> {
        self.aircraft()
    }

    fn enabled_executors(&self) -> Vec<&Aircraft> {
        self.plan.aircraft.iter().collect()
    }
}
