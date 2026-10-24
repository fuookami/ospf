use time::{Duration, OffsetDateTime};
use ospf_rust_framework::solver::{ColumnGenerationSolver, FrameworkSolveOptions};
use ospf_rust_framework_gantt_scheduling::application::service::ActivitySchedulingApplicationService;
use ospf_rust_framework_gantt_scheduling::domain::activity::*;
use ospf_rust_framework_gantt_scheduling::domain::common::{task_id, executor_id};
use ospf_rust_framework_gantt_scheduling::domain::task::F64SolverValueAdapter;
use ospf_rust_framework_gantt_scheduling::infrastructure::TimeRange;
use ospf_rust_quantities::{Quantity, unit::{CTUnit, NoneUnit}};

pub fn run_example<S: ColumnGenerationSolver>(solver: S) -> Result<(), String> {
    let q = |value| Quantity::new(value, NoneUnit::INSTANT.clone());
    let at = |hour| OffsetDateTime::UNIX_EPOCH + Duration::hours(hour);
    let mode = |id: &str, hours: i64, resource: &str, cost: f64| ActivityMode {
        id: id.into(), duration: Duration::hours(hours), starts: vec![at(0), at(2)],
        resources: vec![ActivityResourceUse {
            resource: executor_id(resource), quantity: q(1.0), from: Duration::ZERO, until: None,
        }],
        cost: q(cost), material_flows: vec![],
    };
    let resource = |id: &str| ActivityResource {
        id: executor_id(id), capacity: q(1.0), unavailable_times: vec![], capacities: vec![],
    };
    let problem = ActivitySchedulingProblem {
        horizon: TimeRange::new(at(0), at(6)),
        activities: vec![
            Activity { id: task_id("a"), modes: vec![mode("fast", 2, "machine", 3.0), mode("slow", 3, "alternate", 1.0)], cancellation_cost: None },
            Activity { id: task_id("b"), modes: vec![mode("next", 2, "machine", 0.0)], cancellation_cost: None },
        ],
        resources: vec![resource("machine"), resource("alternate")],
        dependencies: vec![ActivityDependency {
            predecessor: task_id("a"), successor: task_id("b"),
            from: ActivityEndpoint::Finish, to: ActivityEndpoint::Start,
            minimum: Duration::ZERO, maximum: None, presence: ActivityDependencyPresence::RequireBoth,
        }],
        baselines: vec![], executions: vec![], rows: vec![],
        cost_unit: NoneUnit::INSTANT.clone(), materials: vec![], material_events: vec![],
    };
    let report = ActivitySchedulingApplicationService::new(solver).solve(
        || ActivitySchedulingContext::new(problem, &F64SolverValueAdapter, None, Default::default()),
        FrameworkSolveOptions::default(),
    ).map_err(|e| format!("{e:?}"))?;
    println!("report={report:?}");
    match report.solution.and_then(|solution| solution.value) {
        Some(schedule) => println!("cost={:?}, placements={:?}", schedule.cost, schedule.placements),
        None => println!("No schedule; inspect the report before returning a business result"),
    }
    Ok(())
}
