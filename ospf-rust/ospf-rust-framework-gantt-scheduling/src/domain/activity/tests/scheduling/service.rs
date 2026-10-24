use super::*;
use std::sync::Arc;
use ospf_rust_core::error::Result as CoreResult;
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{SolutionPresence, SolveTrace};
use ospf_rust_framework::solver::{ColumnGenerationSolver, FeasibleSolution, FrameworkSolveOptions, LPResult};
use crate::application::service::ActivitySchedulingApplicationService;

macro_rules! invoke {
    ($call:expr) => {{
        #[cfg(feature = "async")]
        { futures::executor::block_on($call) }
        #[cfg(not(feature = "async"))]
        { $call }
    }};
}

type Response = dyn Fn(&LinearTriadModel) -> CoreResult<SolveReport<f64>> + Send + Sync;

struct Fixture {
    calls: Arc<AtomicUsize>,
    response: Box<Response>,
}

impl Fixture {
    fn new(response: impl Fn(&LinearTriadModel) -> CoreResult<SolveReport<f64>> + Send + Sync + 'static) -> Self {
        Self { calls: Arc::new(AtomicUsize::new(0)), response: Box::new(response) }
    }
}

#[cfg_attr(feature = "async", async_trait::async_trait)]
impl ColumnGenerationSolver for Fixture {
    fn name(&self) -> &str { "activity-fake" }

    #[cfg(not(feature = "async"))]
    fn solve_milp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<FeasibleSolution> { panic!("report entry only") }

    #[cfg(feature = "async")]
    async fn solve_milp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<FeasibleSolution> { panic!("report entry only") }

    #[cfg(not(feature = "async"))]
    fn solve_lp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<LPResult> { panic!("single MILP, no LP") }

    #[cfg(feature = "async")]
    async fn solve_lp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<LPResult> { panic!("single MILP, no LP") }

    #[cfg(not(feature = "async"))]
    fn solve_milp_report_with_options(&self, model: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<SolveReport<f64>> {
        self.calls.fetch_add(1, Ordering::SeqCst); (self.response)(model)
    }

    #[cfg(feature = "async")]
    async fn solve_milp_report_with_options(&self, model: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<SolveReport<f64>> {
        self.calls.fetch_add(1, Ordering::SeqCst); (self.response)(model)
    }
}

fn backend_report(model: &LinearTriadModel, values: Vec<f64>) -> SolveReport<f64> {
    let mut solution = SolveSolution::vector(values);
    solution.objective_value = Some(model.c.iter().zip(&solution.values).map(|(c, x)| c * x).sum());
    SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed).solution(solution)
        .proof(SolveProof::optimality()).fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model).unwrap()), ..Default::default() })
        .build().unwrap()
}

#[test]
fn activity_application_single_call_extension_and_zero_column_solution() {
    let fixture = Fixture::new(|model| Ok(backend_report(model, vec![])));
    let calls = fixture.calls.clone(); let service = ActivitySchedulingApplicationService::new(fixture);
    let output = invoke!(service.solve(|| Ok(context(problem())), Default::default())).unwrap();
    assert!(output.solution.unwrap().value.unwrap().placements.is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let mut input = problem(); input.activities.push(activity());
    let fixture = Fixture::new(|model| Ok(backend_report(model, vec![0.0, 1.0, 1.0])));
    let service = ActivitySchedulingApplicationService::new(fixture);
    let output = invoke!(service.solve_with(|| Ok(context(input)), |model, aggregation| {
        let first = aggregation.compilation().variable(&key(1)).unwrap();
        model.add_le_constraint(&[(first, 1.0)], 0.0, "downstream-block").unwrap();
        let extension = model.register_auto_variable_with_range::<Binary>("extension", VariableRange::bounded(1.0, 1.0)).unwrap();
        model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(8.0, extension)], 4.0), "extension-cost"));
        Ok(())
    }, Default::default())).unwrap();
    assert_eq!(output.solution.as_ref().unwrap().objective_value, Some(10.0));
    assert_eq!(output.solution.unwrap().value.unwrap().placements[0].key, key(2));
}

#[test]
fn activity_execution_cannot_replay_across_equal_compilations_or_solve_twice() {
    let mut context = context(problem()); let mut model = MetaModel::new("execution"); context.register(&mut model).unwrap();
    let first = context.compile(&model).unwrap(); let second = context.compile(&model).unwrap();
    let fixture = Fixture::new(|model| Ok(backend_report(model, vec![])));
    let execution = invoke!(first.solve(&fixture, Default::default())).unwrap();
    assert!(context.extract_report(&model, &second, execution).is_err());
    assert!(invoke!(first.solve(&fixture, Default::default())).is_err());
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    let execution = invoke!(second.solve(&fixture, Default::default())).unwrap();
    assert!(context.extract_report(&model, &second, execution).is_ok());
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn activity_application_preserves_normal_terminals_and_propagates_technical_failure() {
    for reason in [TerminationReason::TimeLimit, TerminationReason::Cancelled, TerminationReason::Interrupted, TerminationReason::BackendFailure] {
        let service = ActivitySchedulingApplicationService::new(Fixture::new(move |_| SolveReport::builder(ProblemStatus::Unknown, reason).build()));
        let output = invoke!(service.solve(|| Ok(context(problem())), Default::default())).unwrap();
        assert_eq!(output.termination_reason, reason); assert_eq!(output.solution_presence, SolutionPresence::None);
    }
    let service = ActivitySchedulingApplicationService::new(Fixture::new(|model| {
        SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed).proof(SolveProof::infeasibility())
            .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() }).build()
    }));
    let mut input = problem(); let mut job = activity(); job.modes.clear(); input.activities.push(job);
    let output = invoke!(service.solve(|| Ok(context(input)), Default::default())).unwrap();
    assert_eq!(output.problem_status, ProblemStatus::Infeasible);
    let service = ActivitySchedulingApplicationService::new(Fixture::new(|_| Err(ospf_rust_core::error::CoreError::from(
        ospf_rust_core::error::SolverError::SolveFailed("fake startup failure".into())))));
    assert!(invoke!(service.solve(|| Ok(context(problem())), Default::default())).is_err());
}

#[test]
fn activity_time_limited_incumbent_keeps_backend_gap_and_separate_objective_offset() {
    let mut input = problem(); input.activities.push(activity());
    let service = ActivitySchedulingApplicationService::new(Fixture::new(|model| {
        let mut report = backend_report(model, vec![1.0, 0.0]);
        report.termination_reason = TerminationReason::TimeLimit; report.proof = None;
        report.solution_presence = SolutionPresence::Incumbent;
        report.statistics.best_bound_value = Some(1.0);
        report.statistics.absolute_gap = Some(1.0); report.statistics.relative_gap = Some(0.5);
        report.trace = SolveTrace { global_lower_bound: Some(1.0), upper_bound: Some(2.0), relative_gap: Some(0.5), ..Default::default() };
        report.validate().unwrap(); Ok(report)
    }));
    let output = invoke!(service.solve_with(|| Ok(context(input)), |model, _| {
        model.add_sub_objective(SubObjective::minimize(Linear::new(vec![], 7.0), "constant-offset"));
        Ok(())
    }, Default::default())).unwrap();
    assert_eq!(output.termination_reason, TerminationReason::TimeLimit);
    assert_eq!(output.problem_status, ProblemStatus::Feasible); assert_eq!(output.solution_presence, SolutionPresence::Incumbent);
    assert!(output.proof.is_none()); assert_eq!(output.diagnostics.extensions["activity.objectiveOffset"], "7");
    assert_eq!(output.solution.as_ref().unwrap().objective_value, Some(2.0));
    assert_eq!(output.statistics.best_bound_value, Some(1.0)); assert_eq!(output.statistics.absolute_gap, Some(1.0));
    assert_eq!(output.statistics.relative_gap, Some(0.5)); assert_eq!(output.trace.global_lower_bound, Some(1.0));
    assert_eq!(output.trace.upper_bound, Some(2.0)); assert_eq!(output.trace.relative_gap, Some(0.5));
    assert_eq!(output.solution.unwrap().value.unwrap().cost.value, 2.0);
}

#[test]
fn activity_application_rejects_illegal_incumbents_even_when_solver_claims_optimality() {
    for values in [vec![], vec![0.5, 0.5], vec![0.0, 0.0], vec![f64::NAN, 1.0], vec![f64::INFINITY, 0.0], vec![1.0, 1.0], vec![1.0, 0.0, 0.0]] {
        let mut input = problem(); input.activities.push(activity());
        let service = ActivitySchedulingApplicationService::new(Fixture::new(move |model| {
            let mut result = backend_report(model, vec![1.0, 0.0]); result.solution.as_mut().unwrap().values = values.clone(); Ok(result)
        }));
        assert!(invoke!(service.solve(|| Ok(context(input)), Default::default())).is_err());
    }
}

fn rich_evidence(model: &LinearTriadModel, status: ProblemStatus) -> SolveReport<f64> {
    let mut report = if status == ProblemStatus::Feasible {
        let mut values = vec![0.0; model.num_variables()];
        if let Some(first) = values.first_mut() { *first = 1.0; }
        backend_report(model, values)
    } else {
        SolveReport::builder(status, TerminationReason::Completed).proof(match status {
            ProblemStatus::Infeasible => SolveProof::infeasibility(), ProblemStatus::Unbounded => SolveProof::unboundedness(),
            _ => SolveProof::infeasible_or_unbounded(),
        }).fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model).unwrap()), ..Default::default() }).build().unwrap()
    };
    let bound = report.solution.as_ref().and_then(|solution| solution.objective_value).unwrap_or(1.0);
    report.statistics.best_bound = Some(bound); report.statistics.best_bound_value = Some(bound);
    report.statistics.absolute_gap = (status == ProblemStatus::Feasible).then_some(0.0);
    report.statistics.relative_gap = (status == ProblemStatus::Feasible).then_some(0.0);
    report.statistics.extensions.insert("untrusted-bound".into(), "1".into());
    report.trace = SolveTrace { global_lower_bound: Some(bound), upper_bound: Some(bound), relative_gap: Some(0.0), total_iterations: 1,
        iteration_snapshots: vec![ospf_rust_core::solver::SolveIterationSnapshot { iteration: 1, stage: "compiled".into(), best_bound: Some(bound), objective_value: Some(bound), proof_reference: Some("compiled-proof".into()), ..Default::default() }], ..Default::default() };
    report.model_mapping = Some(ospf_rust_core::solver::audit::linear_model_mapping(model).unwrap());
    if let Some(solution) = &report.solution {
        report.diagnostics = ospf_rust_core::solver::evaluate_linear_solution(model, &solution.values, 1e-7).unwrap();
    }
    report.diagnostics.extensions.insert("solver-only-proof".into(), "compiled".into());
    report.warnings.push(ospf_rust_core::solver::SolveWarning::new("backend.warning", "compiled backend warning"));
    report.diagnostics.warnings.push(ospf_rust_core::solver::SolveWarning::new("backend.diagnostic", "compiled diagnostic warning"));
    report.diagnostics.issues.push(ospf_rust_core::solver::SolveIssue::new("backend.issue", "compiled diagnostic issue"));
    if status == ProblemStatus::Infeasible {
        report.diagnostics.infeasibility_evidence = Some(ospf_rust_core::solver::InfeasibilityEvidence {
            source: ospf_rust_core::solver::InfeasibilityEvidenceSource::NativeIis,
            reliability: ospf_rust_core::solver::ProofReliability::Reliable,
            completeness: ospf_rust_core::solver::ProofCompleteness::Complete,
            constraint_ids: report.model_mapping.as_ref().unwrap().constraints_by_row.iter().cloned().collect(),
            members: BTreeSet::new(), minimality: Default::default(), computation_time: Default::default(), unavailable_reason: None,
        });
    }
    report
}

#[test]
fn activity_identity_reports_preserve_transferable_metadata_and_diagnostics() {
    for status in [ProblemStatus::Feasible, ProblemStatus::Infeasible] {
        let mut input = problem(); input.activities.push(activity());
        let service = ActivitySchedulingApplicationService::new(Fixture::new(move |model| Ok(rich_evidence(model, status))));
        let output = invoke!(service.solve(|| Ok(context(input)), Default::default())).unwrap();
        assert_eq!(output.problem_status, status);
        assert!(output.proof.is_some()); assert!(output.model_mapping.is_some());
        assert_eq!(output.warnings, vec![ospf_rust_core::solver::SolveWarning::new("backend.warning", "compiled backend warning")]);
        assert_eq!(output.diagnostics.warnings, vec![ospf_rust_core::solver::SolveWarning::new("backend.diagnostic", "compiled diagnostic warning")]);
        assert_eq!(output.diagnostics.issues, vec![ospf_rust_core::solver::SolveIssue::new("backend.issue", "compiled diagnostic issue")]);
        assert_eq!(output.statistics.extensions["untrusted-bound"], "1");
        assert_eq!(output.diagnostics.extensions["solver-only-proof"], "compiled");
        assert_eq!(output.diagnostics.infeasibility_evidence.is_some(), status == ProblemStatus::Infeasible);
        assert!(output.diagnostics.constraint_evaluations.is_empty()); assert!(output.diagnostics.variable_bound_evaluations.is_empty());
        assert!(output.diagnostics.extensions["activity.solverModelReport.debug"].contains("compiled"));
        output.validate().unwrap();
    }
}

#[test]
fn activity_lossy_f64_reports_isolate_every_evidence_channel_including_no_incumbent() {
    for status in [ProblemStatus::Feasible, ProblemStatus::Infeasible, ProblemStatus::Unbounded, ProblemStatus::InfeasibleOrUnbounded] {
        let mut input = problem(); input.activities.push(activity());
        let service = ActivitySchedulingApplicationService::new(Fixture::new(move |model| Ok(rich_evidence(model, status))));
        let output = invoke!(service.solve(|| ActivitySchedulingContext::new(input, &BrokenAdapter(4), None, Default::default()), Default::default())).unwrap();
        assert_eq!(output.problem_status, if status == ProblemStatus::Feasible { ProblemStatus::Feasible } else { ProblemStatus::Unknown });
        assert_eq!(output.solution_presence, if status == ProblemStatus::Feasible { SolutionPresence::Incumbent } else { SolutionPresence::None });
        assert!(output.proof.is_none()); assert!(output.statistics.best_bound.is_none()); assert!(output.statistics.best_bound_value.is_none());
        assert!(output.statistics.absolute_gap.is_none()); assert!(output.statistics.relative_gap.is_none()); assert!(output.statistics.extensions.is_empty());
        assert!(output.trace.global_lower_bound.is_none()); assert!(output.trace.upper_bound.is_none()); assert!(output.trace.relative_gap.is_none());
        assert!(output.trace.iteration_snapshots.is_empty()); assert!(output.model_mapping.is_none());
        assert!(output.fingerprints.model.is_none()); assert!(output.diagnostics.infeasibility_evidence.is_none());
        assert!(output.diagnostics.constraint_evaluations.is_empty()); assert!(output.diagnostics.variable_bound_evaluations.is_empty());
        assert!(output.warnings.iter().all(|warning| warning.code == "activity.solverModelOnly"));
        assert!(output.diagnostics.warnings.is_empty()); assert!(output.diagnostics.issues.is_empty());
        assert!(!output.diagnostics.extensions.contains_key("solver-only-proof"));
        assert!(output.diagnostics.extensions["activity.solverModelReport.debug"].contains("compiled"));
        if let Some(solution) = output.solution { assert!(solution.objective_value.is_none()); assert_eq!(solution.value.unwrap().cost.value, 2.0); }
    }
}

#[cfg(feature = "big-decimal")]
#[test]
fn activity_decimal_service_preserves_native_cost_and_isolates_no_incumbent_conclusions() {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;
    use crate::domain::task::SolverValueAdapter;
    let precise = BigDecimal::from(1) + BigDecimal::from_str("1e-100").unwrap();
    for status in [ProblemStatus::Feasible, ProblemStatus::Infeasible, ProblemStatus::Unbounded, ProblemStatus::InfeasibleOrUnbounded] {
        let input = numeric_problem(precise.clone());
        let service = ActivitySchedulingApplicationService::new(Fixture::new(move |model| {
            let mut report = rich_evidence(model, if status == ProblemStatus::Feasible { ProblemStatus::Feasible } else { status });
            if status == ProblemStatus::Feasible { report = backend_report(model, vec![1.0]); }
            Ok(report)
        }));
        let output = invoke!(service.solve(|| ActivitySchedulingContext::new(input, &SolverValueAdapter::<BigDecimal>::new(), None, Default::default()), Default::default())).unwrap();
        assert!(output.proof.is_none()); assert!(output.statistics.best_bound_value.is_none());
        if status == ProblemStatus::Feasible { assert_eq!(output.solution.unwrap().value.unwrap().cost.value, precise); }
        else { assert_eq!(output.problem_status, ProblemStatus::Unknown); assert_eq!(output.solution_presence, SolutionPresence::None); }
    }
}

#[cfg(feature = "scip")]
fn native(input: ActivitySchedulingProblem<f64>) -> SolveReport<ActivitySchedule<f64>> {
    let prepared = prepare(input.clone()).unwrap();
    let service = ActivitySchedulingApplicationService::new(ospf_rust_framework::solver::scip_extension::ScipColumnGenerationSolver::new());
    let output = invoke!(service.solve(|| Ok(context(input)), FrameworkSolveOptions::default())).unwrap();
    if let Some(solution) = &output.solution {
        let schedule = solution.value.as_ref().unwrap();
        assert!(crate::domain::activity::validation::validate_activity_selection(&prepared, &schedule.placements.iter().map(|placement| placement.key.clone()).collect::<Vec<_>>(), &schedule.cancelled).unwrap().feasible());
    }
    output
}

#[cfg(feature = "scip")]
#[test]
fn activity_native_scip_flexible_job_shop() {
    let mut input = problem(); input.activities = vec![activity(), second()];
    input.resources = vec![resource("machine", 1.0), resource("alternative-machine", 1.0)];
    for job in &mut input.activities { job.modes[0].resources.push(usage("machine", 1.0, 0, 1)); }
    let mut alternative = input.activities[0].modes[0].clone(); alternative.id = "alternative".into();
    alternative.resources[0].resource = "alternative-machine".into(); alternative.cost = quantity(5.0); input.activities[0].modes.push(alternative);
    input.dependencies.push(ActivityDependency { predecessor: "job".into(), successor: "other".into(), from: ActivityEndpoint::Finish, to: ActivityEndpoint::Start,
        minimum: Duration::ZERO, maximum: None, presence: ActivityDependencyPresence::RequireBoth });
    let output = native(input); assert_eq!(output.solution.unwrap().value.unwrap().cost.value, 4.0);
}

#[cfg(feature = "scip")]
#[test]
fn activity_native_scip_joint_phases_calendar() {
    let mut input = problem(); input.activities.push(activity()); input.resources = vec![resource("machine", 1.0), resource("crew", 1.0)];
    input.activities[0].modes[0].resources = vec![usage("machine", 1.0, 0, 1), ActivityResourceUse { resource: "crew".into(), quantity: quantity(1.0), from: Duration::minutes(30), until: None }];
    input.resources[1].unavailable_times.push(TimeRange::new(time(1) + Duration::minutes(30), time(2)));
    let output = native(input); let schedule = output.solution.unwrap().value.unwrap();
    assert_eq!(schedule.placements[0].key, key(2)); assert_eq!(schedule.placements[0].reservations.len(), 2);
}

#[cfg(feature = "scip")]
#[test]
fn activity_native_scip_running_recovery_baseline() {
    let mut input = problem(); input.activities.push(activity()); input.resources.push(resource("machine", 1.0));
    input.activities[0].modes[0].resources.push(usage("machine", 1.0, 0, 1));
    input.executions.push(ActivityExecution { placement: ActivityPlacement { key: choice("running", -1), time: TimeRange::new(time(-1), time(2)),
        reservations: vec![ActivityReservation { resource: "machine".into(), time: TimeRange::new(time(0), time(2)), quantity: quantity(1.0) }] }, status: ActivityExecutionStatus::Running });
    input.baselines.push(ActivityBaseline { choice: key(1), frozen: false, start_change_cost: quantity(3.0), mode_change_cost: quantity(9.0) });
    let schedule = native(input).solution.unwrap().value.unwrap(); assert_eq!(schedule.cost.value, 5.0); assert_eq!(schedule.executions.len(), 1);
}

#[cfg(feature = "scip")]
#[test]
fn activity_native_scip_frozen_calendar_infeasible() {
    let mut input = problem(); input.activities.push(activity()); input.resources.push(resource("machine", 1.0));
    input.resources[0].unavailable_times.push(TimeRange::new(time(1), time(2)));
    input.activities[0].modes[0].resources.push(usage("machine", 1.0, 0, 1));
    input.baselines.push(ActivityBaseline { choice: key(1), frozen: true, start_change_cost: quantity(0.0), mode_change_cost: quantity(0.0) });
    let output = native(input); assert_eq!(output.problem_status, ProblemStatus::Infeasible); assert!(output.solution.is_none());
}

#[cfg(feature = "scip")]
#[test]
fn activity_native_scip_material_waiting() {
    let mut input = problem(); input.activities.push(activity()); material(&mut input, 0.0, None);
    input.activities[0].modes[0].material_flows.push(flow(0, -1.0)); input.material_events.push(event(2, 1.0));
    let schedule = native(input).solution.unwrap().value.unwrap(); assert_eq!(schedule.placements[0].key, key(2));
    assert_eq!(schedule.material_inventory[0].levels.iter().find(|level| level.time == time(2)).unwrap().quantity.value, 0.0);
}

#[cfg(feature = "scip")]
#[test]
fn activity_native_scip_explicit_cancellation() {
    let mut input = problem(); input.activities.push(activity()); material(&mut input, 0.0, None);
    input.activities[0].modes[0].material_flows.push(flow(0, -1.0)); input.activities[0].cancellation_cost = Some(quantity(7.0));
    let schedule = native(input).solution.unwrap().value.unwrap(); assert!(schedule.placements.is_empty());
    assert_eq!(schedule.cancelled, BTreeSet::from(["job".into()])); assert_eq!(schedule.cost.value, 7.0);
}
