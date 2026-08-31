//! Demo4 应用层 / Demo4 application layer
//!
//! 航班调度与恢复示例的应用入口，包含框架 trait 实现与演示流程。
//! Application entry for airline scheduling and recovery demo, with framework trait implementations and showcase flow.

use ospf_rust_framework_gantt_scheduling::domain::task::{
    AssignmentPolicyTrait, Cost, CostItem, ExecutorTrait, TaskKey, TaskStatus, TaskTrait, TaskType,
};
use ospf_rust_framework_gantt_scheduling::infrastructure::{
    DurationUnit, TimeRange, TimeWindow, WorkingCalendar,
};
use std::collections::HashSet;
use std::error::Error;
use time::{Date, Duration, Month, OffsetDateTime, Time};

use super::domain::task::model::{
    Aircraft, AircraftCapacity, AircraftMinorType, AircraftType, AircraftUsability, Airport,
    AirportType, FlightAssignmentPolicy, FlightCycle, FlightHour, FlightLeg, FlightLegPlan,
    FlightTaskAssignment, FlightTaskCategory, FlightTaskImpl, FlightTaskPlanImpl, FlightTaskStatus,
    FlightTaskTypeImpl,
};
use super::infrastructure::{
    AircraftMinorTypeCode, AircraftRegisterNumber, AircraftTypeCode, Icao, PassengerClass,
};

/// 构造 OffsetDateTime 辅助函数 / Helper to construct an OffsetDateTime from date-time components
fn dt(year: i32, month: Month, day: u8, hour: u8, min: u8, sec: u8) -> OffsetDateTime {
    Date::from_calendar_date(year, month, day)
        .expect("valid date")
        .with_time(Time::from_hms(hour, min, sec).expect("valid time"))
        .assume_utc()
}

// ============================================================================
// 框架 trait 实现 / Framework trait implementations
// ============================================================================

/// Demo4 任务实现 / Demo4 task implementation
/// 对齐 Kotlin Demo4Task (实现 IterativeAbstractTask)
#[derive(Debug, Clone)]
struct Demo4Task {
    /// 任务唯一标识 / Unique task identifier
    id: String,
    /// 任务名称 / Task name
    name: String,
    /// 迭代次数 / Iteration count
    iteration: i64,
}

impl Demo4Task {
    fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            iteration: 0,
        }
    }
}

impl TaskTrait<Demo4Executor, Demo4AssignmentPolicy> for Demo4Task {
    type Id = String;

    fn id(&self) -> &Self::Id {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }

    fn type_(&self) -> TaskType {
        TaskType::new("Demo4Task")
    }

    fn status(&self) -> HashSet<TaskStatus> {
        let mut s = HashSet::new();
        s.insert(TaskStatus::NotCancel);
        s
    }

    fn cancel_enabled(&self) -> bool {
        false
    }
    fn delay_enabled(&self) -> bool {
        true
    }
    fn advance_enabled(&self) -> bool {
        true
    }

    fn executor(&self) -> Option<&Demo4Executor> {
        None
    }
    fn time(&self) -> Option<&TimeRange> {
        None
    }
}

/// Demo4 执行者 / Demo4 executor
/// 对齐 Kotlin Executor
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Demo4Executor {
    /// 执行者唯一标识 / Unique executor identifier
    id: String,
    /// 执行者名称 / Executor name
    name: String,
}

impl Demo4Executor {
    fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
        }
    }
}

impl ExecutorTrait for Demo4Executor {
    type Id = String;

    fn id(&self) -> &Self::Id {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
}

/// Demo4 分配策略 / Demo4 assignment policy
/// 对齐 Kotlin AssignmentPolicy
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Demo4AssignmentPolicy {
    /// 指定执行者 / Assigned executor
    executor: Option<Demo4Executor>,
    /// 指定时间范围 / Assigned time range
    time: Option<TimeRange>,
}

impl AssignmentPolicyTrait<Demo4Executor> for Demo4AssignmentPolicy {
    fn executor(&self) -> Option<&Demo4Executor> {
        self.executor.as_ref()
    }

    fn time(&self) -> Option<&TimeRange> {
        self.time.as_ref()
    }
}

/// 运行 Demo4 航班调度示例 / Run Demo4 airline scheduling example
pub fn run() -> Result<(), Box<dyn Error>> {
    println!("=== Framework Demo4 (Airline Scheduling) ===");

    // 创建机场 / Create airports
    let airport_zbaa = Airport {
        icao: Icao("ZBAA".into()),
        airport_type: AirportType::Domestic,
        passenger_transfer_time: Duration::minutes(90),
        cargo_transfer_time: Duration::minutes(120),
        base: true,
    };
    let airport_zuuu = Airport {
        icao: Icao("ZUUU".into()),
        airport_type: AirportType::Domestic,
        passenger_transfer_time: Duration::minutes(90),
        cargo_transfer_time: Duration::minutes(120),
        base: false,
    };
    println!(
        "Airports: {} ({:?}), {} ({:?})",
        airport_zbaa.icao.0,
        airport_zbaa.airport_type,
        airport_zuuu.icao.0,
        airport_zuuu.airport_type,
    );

    // 创建飞机类型 / Create aircraft type
    let aircraft_type = AircraftType {
        code: AircraftTypeCode("B737".into()),
    };
    let minor_type = AircraftMinorType {
        aircraft_type: aircraft_type.clone(),
        code: AircraftMinorTypeCode("B737-800".into()),
        cost_per_hour: 2500.0,
        route_fly_time: std::collections::HashMap::new(),
        connection_time: std::collections::HashMap::new(),
        max_fly_time: Some(Duration::hours(4)),
    };

    // 创建飞机 / Create aircraft
    let aircraft = Aircraft {
        reg_no: AircraftRegisterNumber("B1234".into()),
        minor_type: minor_type.clone(),
        capacity: AircraftCapacity::Passenger {
            capacities: vec![
                (PassengerClass("C".into()), 8),
                (PassengerClass("Y".into()), 150),
            ],
        },
    };
    println!(
        "Aircraft: {} type={:?} cost/h={}",
        aircraft.reg_no.0,
        aircraft.type_code().code.0,
        aircraft.cost_per_hour()
    );

    // 创建航班 / Create flight leg
    let flight_plan = FlightLegPlan {
        actual_id: "FL001".into(),
        no: "CA1234".into(),
        flight_type: super::domain::task::model::FlightType::Domestic,
        aircraft: aircraft.clone(),
        enabled_aircrafts: vec![aircraft.clone()],
        dep: airport_zbaa.clone(),
        arr: airport_zuuu.clone(),
        scheduled_time: (
            dt(2026, Month::June, 7, 8, 0, 0),
            dt(2026, Month::June, 7, 11, 0, 0),
        ),
        estimated_time: None,
        actual_time: None,
        out_time: None,
        flight_task_status: vec![FlightTaskStatus::NotCancel],
        weight: 1.0,
    };
    let flight_leg = FlightLeg::new(flight_plan);
    println!(
        "Flight: {} {} -> {} time={:?}",
        flight_leg.plan.no,
        flight_leg.dep().icao.0,
        flight_leg.arr().icao.0,
        flight_leg.plan.time().map(|(s, e)| (s, e)),
    );

    // 创建 FlightTaskImpl (实现 TaskTrait)
    let flight_task = FlightTaskImpl::new(
        FlightTaskPlanImpl {
            id: "CA1234".into(),
            name: "CA1234_20260607".into(),
            aircraft: Some(aircraft.clone()),
            dep: airport_zbaa.clone(),
            arr: airport_zuuu.clone(),
            time: Some((
                dt(2026, Month::June, 7, 8, 0, 0),
                dt(2026, Month::June, 7, 11, 0, 0),
            )),
            scheduled_time: Some((
                dt(2026, Month::June, 7, 8, 0, 0),
                dt(2026, Month::June, 7, 11, 0, 0),
            )),
            flight_task_status: vec![FlightTaskStatus::NotCancel],
        },
        FlightTaskTypeImpl {
            category: FlightTaskCategory::Flight,
            name: "flight".into(),
        },
    );
    println!(
        "FlightTask: id={} name={} cancel_enabled={} delay_enabled={}",
        flight_task.id(),
        flight_task.name(),
        flight_task.cancel_enabled(),
        flight_task.delay_enabled(),
    );

    // 创建聚合 / Create aggregation
    let task_agg = super::domain::task::Aggregation::new(
        vec![airport_zbaa, airport_zuuu],
        vec![aircraft],
        vec![],
        vec![flight_leg],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    println!(
        "Task aggregation: {} airports, {} aircrafts, {} legs",
        task_agg.airports.len(),
        task_agg.aircrafts.len(),
        task_agg.legs.len()
    );

    // GenericQuantitySample 对齐 Kotlin Demo4GenericQuantitySample
    let time = TimeRange::new(
        dt(2026, Month::June, 7, 0, 0, 0),
        dt(2026, Month::June, 7, 8, 0, 0),
    );

    let cost = Cost::new(vec![CostItem::with_quantity("generic-demo", 1.25_f64)]);
    let cost_sum: f64 = cost.cost_sum.unwrap_or(0.0);
    println!("Cost: items={}, sum={:.2}", cost.items.len(), cost_sum);

    let time_window: TimeWindow<f64> = TimeWindow::hours(time.clone(), 0.0, true, 1.0);
    let duration_val = time_window.value_of_duration(Duration::hours(2));
    println!("TimeWindow value_of_duration(2h): {}", duration_val);

    let instant_val = time_window.value_of_instant(dt(2026, Month::June, 7, 3, 0, 0));
    println!("TimeWindow value_of_instant(3:00): {}", instant_val);

    // WorkingCalendar / ActualTime
    let calendar: WorkingCalendar<f64> = WorkingCalendar::new(time_window.clone(), vec![]);
    let query_range = TimeRange::new(
        dt(2026, Month::June, 7, 0, 0, 0),
        dt(2026, Month::June, 7, 4, 0, 0),
    );
    let actual_time = calendar.actual_time_range(&query_range, &[], None, None, None);
    println!("WorkingCalendar actual_time: {:?}", actual_time);

    // FlightHour / FlightCycle
    let fh = FlightHour::new(Duration::hours(10));
    let fc = FlightCycle::new(5);
    println!(
        "FlightHour: {}h, FlightCycle: {} cycles",
        DurationUnit::Hours.to_value(fh.hours),
        fc.cycles
    );

    // Demo4Task (框架 trait 实现)
    let demo4_task = Demo4Task::new("demo4-task", "Demo Task");
    let demo4_executor = Demo4Executor::new("demo4-executor", "Demo Executor");
    println!(
        "Demo4Task: id={} name={} type={}",
        demo4_task.id(),
        demo4_task.name(),
        demo4_task.type_().name
    );
    println!(
        "Demo4Executor: id={} name={}",
        demo4_executor.id(),
        demo4_executor.name()
    );

    // TaskKey
    let task_key: TaskKey<String> = TaskKey::new("CA1234", TaskType::default_type());
    println!("TaskKey: id={}, type={}", task_key.id, task_key.type_.name);

    // TimeWindow value conversions
    let duration_from_val = time_window.duration_of(3.0);
    println!("TimeWindow duration_of(3.0): {:?}", duration_from_val);

    let instant_from_val = time_window.instant_of(5.0);
    println!("TimeWindow instant_of(5.0): {:?}", instant_from_val);

    println!("Demo4 airline scheduling showcase complete.");
    Ok(())
}
