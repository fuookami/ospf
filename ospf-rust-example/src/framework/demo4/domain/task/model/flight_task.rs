use time::{Duration, OffsetDateTime};
use std::collections::HashSet;
use ospf_rust_framework_gantt_scheduling::domain::task::{
    TaskTrait, TaskType, TaskKey, ExecutorTrait, AssignmentPolicyTrait, TaskStatus,
};
use ospf_rust_framework_gantt_scheduling::infrastructure::TimeRange;
use super::aircraft::Aircraft;
use super::airport::Airport;

/// 飞行任务类别 / Flight task category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightTaskCategory {
    Flight,
    VirtualFlight,
    Maintenance,
    AOG,
}

impl FlightTaskCategory {
    pub fn is_flight_type(&self) -> bool {
        matches!(self, FlightTaskCategory::Flight | FlightTaskCategory::VirtualFlight)
    }
}

/// 飞行任务状态 / Flight task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightTaskStatus {
    NotAdvance,
    NotDelay,
    NotCancel,
    NotCancelPreferred,
    NotAircraftChange,
    NotAircraftTypeChange,
    NotAircraftMinorTypeChange,
    NotTerminalChange,
    StrongLimitIgnored,
}

/// 飞行任务分配策略 / Flight task assignment policy
#[derive(Debug, Clone, Default)]
pub struct FlightTaskAssignment {
    pub aircraft: Option<Aircraft>,
    pub time: Option<OffsetDateTime>,
    pub route: Option<super::airport::Route>,
}

impl FlightTaskAssignment {
    pub fn is_empty(&self) -> bool {
        self.aircraft.is_none() && self.time.is_none() && self.route.is_none()
    }
}

/// 飞行任务类型 / Flight task type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlightTaskType {
    pub category: FlightTaskCategory,
    pub name: String,
}

/// 飞行任务变更 / Aircraft change
#[derive(Debug, Clone)]
pub struct AircraftChange {
    pub from: Aircraft,
    pub to: Aircraft,
}

/// 飞机类型变更 / Aircraft type change
#[derive(Debug, Clone)]
pub struct AircraftTypeChange {
    pub from: super::aircraft_type::AircraftType,
    pub to: super::aircraft_type::AircraftType,
}

/// 飞机子类型变更 / Aircraft minor type change
#[derive(Debug, Clone)]
pub struct AircraftMinorTypeChange {
    pub from: super::aircraft_type::AircraftMinorType,
    pub to: super::aircraft_type::AircraftMinorType,
}

/// 航线变更 / Route change
#[derive(Debug, Clone)]
pub struct RouteChange {
    pub from: super::airport::Route,
    pub to: super::airport::Route,
}

// ============================================================================
// 框架 trait 实现 / Framework trait implementations
// ============================================================================

/// Demo4 飞行任务类型实现 / Demo4 flight task type implementation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlightTaskTypeImpl {
    pub category: FlightTaskCategory,
    pub name: String,
}

/// 飞行任务计划 / Flight task plan
#[derive(Debug, Clone)]
pub struct FlightTaskPlanImpl {
    pub id: String,
    pub name: String,
    pub aircraft: Option<Aircraft>,
    pub dep: Airport,
    pub arr: Airport,
    pub time: Option<(OffsetDateTime, OffsetDateTime)>,
    pub scheduled_time: Option<(OffsetDateTime, OffsetDateTime)>,
    pub flight_task_status: Vec<FlightTaskStatus>,
}

/// 飞行任务 / Flight task (实现 TaskTrait)
#[derive(Debug, Clone)]
pub struct FlightTaskImpl {
    pub plan: FlightTaskPlanImpl,
    pub task_type: FlightTaskTypeImpl,
    pub recovery_aircraft: Option<Aircraft>,
    pub recovery_time: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl FlightTaskImpl {
    pub fn new(plan: FlightTaskPlanImpl, task_type: FlightTaskTypeImpl) -> Self {
        Self {
            plan,
            task_type,
            recovery_aircraft: None,
            recovery_time: None,
        }
    }

    pub fn aircraft(&self) -> Option<&Aircraft> {
        self.recovery_aircraft.as_ref().or(self.plan.aircraft.as_ref())
    }

    pub fn dep(&self) -> &Airport {
        &self.plan.dep
    }

    pub fn arr(&self) -> &Airport {
        &self.plan.arr
    }

    pub fn time(&self) -> Option<(OffsetDateTime, OffsetDateTime)> {
        self.recovery_time.or(self.plan.time)
    }

    pub fn recovered(&self) -> bool {
        self.recovery_aircraft.is_some() || self.recovery_time.is_some()
    }
}

// 实现 AssignmentPolicyTrait
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlightAssignmentPolicy {
    pub executor: Option<Aircraft>,
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
    fn id(&self) -> &str {
        &self.plan.id
    }

    fn name(&self) -> &str {
        &self.plan.name
    }

    fn type_(&self) -> TaskType {
        TaskType::new(&self.task_type.name)
    }

    fn key(&self) -> TaskKey {
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
        !self.plan.flight_task_status.contains(&FlightTaskStatus::NotCancel)
    }

    fn delay_enabled(&self) -> bool {
        !self.plan.flight_task_status.contains(&FlightTaskStatus::NotDelay)
    }

    fn advance_enabled(&self) -> bool {
        !self.plan.flight_task_status.contains(&FlightTaskStatus::NotAdvance)
    }

    fn executor(&self) -> Option<&Aircraft> {
        self.aircraft()
    }

    fn enabled_executors(&self) -> Vec<&Aircraft> {
        self.plan.aircraft.iter().collect()
    }
}
