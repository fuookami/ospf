use time::{Duration, OffsetDateTime};
use ospf_rust_framework::solver::{ColumnGenerationSolver, FrameworkSolveOptions};
use ospf_rust_framework_gantt_scheduling::application::service::ActivitySchedulingApplicationService;
use ospf_rust_framework_gantt_scheduling::domain::activity::*;
use ospf_rust_framework_gantt_scheduling::domain::common::{task_id, executor_id};
use ospf_rust_framework_gantt_scheduling::domain::task::F64SolverValueAdapter;
use ospf_rust_framework_gantt_scheduling::infrastructure::TimeRange;
use ospf_rust_quantities::{Quantity, unit::{CTUnit, NoneUnit}};

pub fn make_problem() -> ActivitySchedulingProblem<f64> {
    let at = |h| OffsetDateTime::UNIX_EPOCH + Duration::hours(h);
    let q = |value| Quantity::new(value, NoneUnit::INSTANT.clone());
    // #region input
    let resource = ActivityResource { id: executor_id("machine"), capacity: q(2.0),
        unavailable_times: vec![TimeRange::new(at(1), at(2))],
        capacities: vec![ActivityCapacity { time: TimeRange::new(at(3), at(4)), capacity: q(1.0) }] };
    let mode = ActivityMode { id: "normal".into(), duration: Duration::hours(2), starts: vec![at(0), at(2), at(4)],
        resources: vec![ActivityResourceUse { resource: executor_id("machine"), quantity: q(2.0), from: Duration::ZERO, until: None }],
        cost: q(0.0), material_flows: vec![] };
    let task = Activity { id: task_id("a"), modes: vec![mode], cancellation_cost: None };
    // #endregion input
    ActivitySchedulingProblem { horizon: TimeRange::new(at(0), at(6)), activities: vec![task], resources: vec![resource],
        dependencies: vec![], baselines: vec![], executions: vec![], rows: vec![], cost_unit: NoneUnit::INSTANT.clone(),
        materials: vec![], material_events: vec![] }
}

pub fn run_business<S: ColumnGenerationSolver>(solver: S) -> Result<(), String> {
    // #region rule
    let late_start_fee = |p: &ActivityPlacement<f64>| -> ospf_rust_framework_gantt_scheduling::GanttResult<_> {
        let late = p.key.start >= OffsetDateTime::UNIX_EPOCH + Duration::hours(4);
        Ok(Quantity::new(if late { 2.0 } else { 0.0 }, NoneUnit::INSTANT.clone()))
    };
    let context = ActivitySchedulingContext::new(make_problem(), &F64SolverValueAdapter,
        Some(&late_start_fee), Default::default()).map_err(|e| format!("{e:?}"))?;
    // #endregion rule
    // #region entry
    let report = ActivitySchedulingApplicationService::new(solver)
        .solve(|| Ok(context), FrameworkSolveOptions::default()).map_err(|e| format!("{e:?}"))?;
    println!("{report:?}");
    // #endregion entry
    Ok(())
}
