//! 非空 Gantt 循环、候选回放与共享路径定价回归。 / Nonempty Gantt-loop, candidate-replay and shared-path pricing regressions.

mod fleet;
mod initial;
mod operating_costs;
mod restrictions;
mod flow_control;
mod passenger_recovery;
mod passenger_rebooking;
mod transition;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use ospf_rust_core::error::{CoreError, Result as CoreResult, SolverError};
use ospf_rust_core::model::{MetaModel, intermediate::LinearTriadModel};
use ospf_rust_core::solver::{ProblemStatus, SolveReport, TerminationReason};
use ospf_rust_framework::solver::column_generation_solver::{ColumnGenerationSolver, FeasibleSolution, LPResult, LinearDualSolution};
use ospf_rust_framework::solver::framework_solve_options::FrameworkSolveOptions;
use ospf_rust_framework_gantt_scheduling::application::algorithm::{BunchPricingRequest, ColumnGenerationPolicy};
use ospf_rust_framework_gantt_scheduling::domain::bunch_compilation::IterativeBunchCompilationContext;
use crate::framework::demo4::application::{RecoveryApplicationConfiguration, RecoveryApplicationService};
use crate::framework::demo4::domain::bunch_generation::pricing::RecoveryPricingPolicy;
use crate::framework::demo4::domain::bunch_generation::problem::{RecoveryAircraftDomain, RecoveryProblem};
use super::*;

struct Reward;

impl RecoveryCostPolicy for Reward {
    fn activity_cost(&self, activity: &FlightTaskRecovery) -> Result<f64, RecoveryError> { CostCalculator.activity_cost(activity) }
    fn terminal_cost(&self, bunch: &FlightTaskBunch) -> Result<f64, RecoveryError> {
        Ok(if bunch.activities().len() == 2 { -500.0 } else { 0.0 })
    }
}

fn problem<C: RecoveryCostPolicy>(cost: C) -> Arc<RecoveryProblem<C>> {
    Arc::new(RecoveryProblem::new(RecoveryMasterConfiguration {
        tasks: bunch().activities().iter().map(|activity| activity.original().clone()).collect(),
        aircraft: vec![aircraft()], cancellation_costs: vec![Some(1000.0); 2],
    }, vec![RecoveryAircraftDomain { aircraft: aircraft(), usability: usability(), previous: None, candidates: candidates() }], vec![], cost).unwrap())
}

fn request() -> BunchPricingRequest {
    BunchPricingRequest { scope: scope(), iteration: 1, executor_ids: vec!["B1001".into()],
        shadow_prices: HashMap::new(), executor_slot_shadow_prices: HashMap::new(),
        row_duals: BTreeMap::from([("task_compilation_0".into(), 200.0), ("task_compilation_1".into(), 200.0),
            ("executor_compilation_B1001".into(), -50.0)]),
        verified_duals: None, fixed_groups: Default::default(), kept_groups: Default::default(),
        hidden_executors: Default::default(), min_column_amount_per_executor: 0 }
}

#[test]
fn full_row_pricing_retains_true_cost_trace_and_heuristic_proof_boundary() {
    let problem = problem(CostCalculator);
    let policy = RecoveryPricingPolicy::new(problem.clone(), None);
    let request = request();
    let result = policy.price(&request, &PathSearchBudget::default()).unwrap();
    assert_eq!(result.batch.completion, ColumnGenerationCompletion::Exhausted);
    assert_eq!(result.batch.no_improving_columns, None);
    assert!(!result.batch.certifies_no_improving_columns(&request.scope));
    assert_eq!(result.batch.columns.len(), 2);
    let entry = result.batch.columns.iter().find(|entry| entry.task_indices.len() == 2).unwrap();
    assert_eq!(entry.cost, 300.0);
    assert_eq!(result.reduced_costs[entry.id.as_str()], -50.0);
    assert_eq!(result.searches[0].search.diagnostics.total_results, 3);
    let candidate = problem.resolve_candidate(entry).unwrap();
    assert!(candidate.bunch.arrived_when(&airport("B"), &(at(9, 0), at(9, 0))));
    let path = result.searches[0].search.paths.iter().find(|path| path.state.bunch.activities().len() == 2).unwrap();
    assert_eq!(result.searches[0].search.trace(path.label).unwrap().len(), 3);
    let coefficients = problem.column_coefficients(&candidate).unwrap();
    assert_eq!(coefficients.values().copied().collect::<Vec<_>>(), vec![1.0; 3]);
}

#[test]
fn missing_zero_duals_unmapped_rows_bad_scopes_and_interrupted_search_cannot_certify() {
    let policy = RecoveryPricingPolicy::new(problem(Reward), None);
    let mut wrong = request();
    wrong.row_duals.remove("executor_compilation_B1001");
    assert!(policy.price(&wrong, &PathSearchBudget::default()).is_err());
    wrong.row_duals.insert("unmapped".into(), 0.0);
    assert!(policy.price(&wrong, &PathSearchBudget::default()).is_err());
    wrong = request();
    wrong.row_duals.insert("task_compilation_0".into(), f64::NAN);
    assert!(policy.price(&wrong, &PathSearchBudget::default()).is_err());
    wrong = request();
    wrong.scope.groups.insert("unknown".into());
    assert!(policy.price(&wrong, &PathSearchBudget::default()).is_err());
    let limited = policy.price(&request(), &PathSearchBudget { maximum_labels: Some(1), ..Default::default() }).unwrap();
    assert_eq!(limited.batch.completion, ColumnGenerationCompletion::BudgetLimit);
    assert_eq!(limited.batch.no_improving_columns, None);
    let interrupted = || true;
    let result = policy.price(&request(), &PathSearchBudget { interrupted: Some(&interrupted), ..Default::default() }).unwrap();
    assert_eq!(result.batch.completion, ColumnGenerationCompletion::Interrupted);
    assert_eq!(result.batch.no_improving_columns, None);
}

#[test]
fn gantt_context_resolves_full_payload_atomically_and_does_not_share_registration_state() {
    let problem = problem(CostCalculator);
    let generated = RecoveryPricingPolicy::new(problem.clone(), None).price(&request(), &PathSearchBudget::default()).unwrap();
    let mut left = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut right = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut left_model = MetaModel::new("left-recovery-node");
    let mut right_model = MetaModel::new("right-recovery-node");
    left.register(&mut left_model).unwrap();
    right.register(&mut right_model).unwrap();
    let mut entry = generated.batch.columns[0].clone();
    entry.index = 40;
    let mut bad = entry.clone();
    bad.cost += 1.0;
    assert!(left.add_columns(1, vec![entry.clone(), bad], &mut left_model).is_err());
    assert_eq!(left_model.as_basic().num_tokens(), 2);
    assert_eq!(left.column_count(), 0);
    left.add_columns(1, vec![entry.clone()], &mut left_model).unwrap();
    assert_eq!(right.column_count(), 0);
    assert_eq!(right_model.as_basic().num_tokens(), 2);
    assert!(left.add_columns(1, vec![entry.clone()], &mut right_model).is_err());
    right.add_columns(1, vec![entry], &mut right_model).unwrap();
    left.remove_columns(&[40], &mut left_model).unwrap();
    assert_eq!(left.column_count(), 0);
    assert_eq!(right.column_count(), 1);
    let (_, values) = master_optimum(&right_model).unwrap();
    let schedule = right.extract_solution(&right_model, &values).unwrap();
    assert_eq!(schedule.selected[0].bunch.recovery("F1").unwrap().time(), (at(8, 0), at(9, 0)));
}

#[test]
fn full_candidate_identity_replay_rejects_forged_variants_and_noncanonical_encodings() {
    let problem = problem(CostCalculator);
    let result = RecoveryPricingPolicy::new(problem.clone(), None).price(&request(), &PathSearchBudget::default()).unwrap();
    let entry = result.batch.columns.iter().find(|entry| entry.task_indices.len() == 2).unwrap().clone();
    for id in ["6:B100133:idle:12:F1-recovered:12:unknown-long", "5:B1001", "5:B10010:", "5:B10014:idle:999999999999999999999999999999:a"] {
        let mut forged = entry.clone();
        forged.id = id.into();
        assert!(problem.resolve_candidate(&forged).is_err());
    }
    let mut forged = entry;
    forged.task_indices.reverse();
    assert!(problem.resolve_candidate(&forged).is_err());
}

#[test]
fn unicode_variant_identity_uses_utf8_byte_lengths_and_rejects_split_codepoints() {
    let base = problem(CostCalculator);
    let mut domains = base.domains().to_vec();
    domains[0].candidates[0].id = "恢复:一".into();
    let problem = RecoveryProblem::new(base.configuration().clone(), domains, vec![], CostCalculator).unwrap();
    let candidate = single(&problem, 0, 0);
    let entry = problem.entry(&candidate).unwrap();
    assert_eq!(problem.resolve_candidate(&entry).unwrap().bunch.recovery("F1").unwrap().time().0, at(8, 0));
    let mut forged = entry;
    let encoded = "idle:1:恢复:一";
    forged.id = format!("5:B1001{}:{}", encoded.len(), encoded).into();
    assert!(problem.resolve_candidate(&forged).is_err());
}

#[derive(Clone, Default)]
struct EnumeratingSolver(Arc<Mutex<Vec<(bool, usize)>>>);

// 只用于小夹具的独立二元 oracle；LP 报告刻意无证明，不冒充通用 LP solver。 / Independent binary oracle for tiny fixtures only; LP reports deliberately carry no proof and do not impersonate a general LP solver.
fn enumerate(model: &LinearTriadModel) -> CoreResult<FeasibleSolution> {
    assert!(model.num_variables() < 12);
    (0..(1usize << model.num_variables())).filter_map(|mask| {
        let values = (0..model.num_variables()).map(|index| ((mask >> index) & 1) as f64).collect::<Vec<_>>();
        if values.iter().enumerate().any(|(index, value)| *value < model.lb[index] || *value > model.ub[index])
            || model.A.rows.iter().zip(&model.b).any(|(row, bound)| row.entries.iter()
                .map(|(index, coefficient)| values[*index] * coefficient).sum::<f64>() > *bound + 1e-9)
        { return None; }
        let objective = values.iter().zip(&model.c).map(|(value, coefficient)| value * coefficient).sum::<f64>();
        Some(FeasibleSolution::new(objective, values))
    }).min_by(|left, right| left.obj.total_cmp(&right.obj))
        .ok_or(CoreError::Solver(SolverError::Infeasible))
}

impl ColumnGenerationSolver for EnumeratingSolver {
    fn name(&self) -> &str { "demo4-small-binary-oracle" }

    fn solve_milp_with_options(&self, model: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<FeasibleSolution> {
        self.0.lock().unwrap().push((true, model.num_variables()));
        enumerate(model)
    }

    fn solve_milp_report_with_options(&self, model: &LinearTriadModel, options: FrameworkSolveOptions) -> CoreResult<SolveReport<f64>> {
        match self.solve_milp_with_options(model, options) {
            Ok(solution) => solution.to_solve_report(self.name()),
            Err(CoreError::Solver(SolverError::Infeasible)) => SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed).build(),
            Err(error) => Err(error),
        }
    }

    fn solve_lp_with_options(&self, model: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<LPResult> {
        assert!(!model.has_integer_variables());
        self.0.lock().unwrap().push((false, model.num_variables()));
        Ok(LPResult::new(enumerate(model)?, LinearDualSolution::new(vec![0.0; model.num_constraints()], vec![])))
    }
}

#[test]
fn nonempty_recovery_application_reuses_gantt_rmp_pricing_final_milp_and_extracts_internal_events() {
    let problem = problem(Reward);
    let solver = EnumeratingSolver::default();
    let calls = solver.0.clone();
    let report = RecoveryApplicationService::new(solver).solve(problem.clone(), RecoveryApplicationConfiguration {
        column_generation: ColumnGenerationPolicy { max_iterations: 1, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert!(!report.is_optimal());
    assert!(report.proof.is_none());
    assert_eq!(report.statistics.best_bound_value, None);
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(-200.0));
    let result = report.solution.unwrap().value.unwrap();
    assert_eq!(*calls.lock().unwrap(), vec![(true, 2), (false, 2), (false, 3), (true, 3)]);
    assert_eq!(result.candidate_ids.len(), 1);
    assert_eq!(result.pricing.len(), 1);
    assert_eq!(result.pricing[0].searches[0].search.diagnostics.total_results, 3);
    assert!(result.schedule.canceled.is_empty());
    let bunch = &result.schedule.selected[0].bunch;
    assert_eq!(bunch.task_ids().collect::<Vec<_>>(), vec!["F1", "F2"]);
    assert!(bunch.arrived_when(&airport("B"), &(at(9, 0), at(9, 0))));
    assert!(bunch.departed_when(&airport("B"), &(at(10, 0), at(10, 0))));
    assert_eq!(bunch.recovery("F1").unwrap().original().plan.time, Some((at(7, 0), at(8, 0))));
    assert_eq!(bunch.recovery("F1").unwrap().time(), (at(8, 0), at(9, 0)));
    assert_eq!(CostCalculator.calculate_bunch(bunch, problem.cost()).unwrap(), -200.0);
}

#[test]
fn initial_recovery_incumbent_does_not_pin_aircraft_or_prevent_column_improvement() {
    let problem = problem(Reward);
    let initial = vec![single(&problem, 0, 0)];
    let report = RecoveryApplicationService::new(EnumeratingSolver::default()).solve(problem, RecoveryApplicationConfiguration {
        initial, column_generation: ColumnGenerationPolicy { max_iterations: 1, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(-200.0));
    let solution = report.solution.unwrap().value.unwrap();
    assert_eq!(solution.schedule.selected[0].bunch.activities().len(), 2);
    assert!(!solution.pricing[0].batch.columns.is_empty());
}

#[test]
fn initial_finite_master_infeasibility_is_a_report_not_full_domain_infeasibility() {
    let base = problem(CostCalculator);
    let mut configuration = base.configuration().clone();
    configuration.cancellation_costs = vec![None; 2];
    let problem = Arc::new(RecoveryProblem::new(configuration, base.domains().to_vec(), vec![], CostCalculator).unwrap());
    let report = RecoveryApplicationService::new(EnumeratingSolver::default()).solve(problem, Default::default()).unwrap();
    assert!(!report.has_incumbent());
    assert_eq!(report.problem_status, ProblemStatus::Unknown);
    assert_eq!(report.termination_reason, TerminationReason::Completed);
    assert_eq!(report.diagnostics.extensions["demo4.finiteMasterStatus"], "Infeasible");
    assert!(report.proof.is_none());
}

fn transfer_problem(amount: u64) -> Arc<RecoveryProblem<CostCalculator>> {
    let base = problem(CostCalculator);
    let mut second = aircraft();
    second.reg_no.0 = "B1002".into();
    let original = base.configuration().tasks[1].clone();
    let variant = |id: &str, start, end| RecoveryActivityCandidate { id: id.into(), recovery: FlightTaskRecovery::new(
        original.clone(), second.clone(), Route { dep: airport("B"), arr: airport("C") }, (start, end),
    ).unwrap() };
    let second_candidates = vec![variant("bad", at(9, 30), at(10, 30)), variant("good", at(10, 0), at(11, 0)), variant("seed", at(12, 0), at(14, 0))];
    let configuration = RecoveryMasterConfiguration { tasks: base.configuration().tasks.clone(),
        aircraft: vec![aircraft(), second.clone()], cancellation_costs: vec![Some(1000.0); 2] };
    Arc::new(RecoveryProblem::new(configuration, vec![
        RecoveryAircraftDomain { aircraft: aircraft(), usability: usability(), previous: None, candidates: vec![candidates()[0].clone()] },
        RecoveryAircraftDomain { aircraft: second, usability: AircraftUsability { location: airport("B"), ..usability() }, previous: None, candidates: second_candidates },
    ], vec![], CostCalculator).unwrap().with_passengers(vec![passenger(amount)]).unwrap())
}

fn single<C: RecoveryCostPolicy>(problem: &RecoveryProblem<C>, aircraft: usize, variant: usize) -> RecoveryBunchCandidate {
    let domain = &problem.domains()[aircraft];
    let activity = &domain.candidates[variant];
    let bunch = FlightTaskBunch::new(format!("idle:{}:{}", activity.id.len(), activity.id), domain.aircraft.clone(), vec![activity.recovery.clone()]).unwrap();
    let cost = CostCalculator.calculate_bunch(&bunch, problem.cost()).unwrap();
    RecoveryBunchCandidate { bunch, cost }
}

#[test]
fn passenger_master_rows_enforce_cross_bunch_transfers_capacity_and_real_cancellation_variables() {
    let problem = transfer_problem(60);
    let mut context = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut model = MetaModel::new("passenger-transfer-master");
    context.register(&mut model).unwrap();
    let all = vec![single(&problem, 0, 0), single(&problem, 1, 0), single(&problem, 1, 1)];
    context.add_candidates(0, all.clone(), &mut model).unwrap();
    let (objective, values) = master_optimum(&model).unwrap();
    assert_eq!(objective, 200.0);
    let schedule = context.extract_solution(&model, &values).unwrap();
    assert!(schedule.canceled.is_empty());
    assert_eq!(schedule.selected.len(), 2);
    assert_eq!(schedule.selected[1].bunch.recovery("F2").unwrap().time().0, at(10, 0));
    let mut forged = vec![0.0; values.len()];
    forged[2] = 1.0;
    forged[3] = 1.0;
    assert!(context.extract_solution(&model, &forged).is_err());
    let mut canceled = vec![0.0; values.len()];
    canceled[..2].fill(1.0);
    assert!(context.extract_solution(&model, &canceled).is_err());
    context.remove_columns(&[2], &mut model).unwrap();
    assert!(master_optimum(&model).is_none());
    context.add_candidates(1, vec![all[2].clone()], &mut model).unwrap();
    let (_, values) = master_optimum(&model).unwrap();
    assert_eq!(context.extract_solution(&model, &values).unwrap().selected.len(), 2);

    let overcapacity = transfer_problem(101);
    let mut context = BunchCompilationContext::with_resolver(overcapacity.configuration().clone(), overcapacity.clone()).unwrap();
    let mut model = MetaModel::new("passenger-capacity-master");
    context.register(&mut model).unwrap();
    context.add_candidates(0, vec![single(&overcapacity, 0, 0), single(&overcapacity, 1, 1)], &mut model).unwrap();
    assert!(master_optimum(&model).is_none());
}

#[test]
fn passenger_extension_uses_identical_signed_coefficients_in_master_and_pricing() {
    let problem = transfer_problem(60);
    let mut context = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut model = MetaModel::new("passenger-row-coefficients");
    context.register(&mut model).unwrap();
    let columns = vec![single(&problem, 0, 0), single(&problem, 1, 0), single(&problem, 1, 1)];
    context.add_candidates(0, columns.clone(), &mut model).unwrap();
    for (index, candidate) in columns.iter().enumerate() {
        let expected = problem.column_coefficients(candidate).unwrap();
        let variable = context.master().compilation.bunch_x_map[&index];
        for row in model.constraints() {
            let actual = row.inequality.polynomial.monomials().iter().filter(|term| term.var_index() == variable).map(|term| *term.coefficient()).sum::<f64>();
            assert_eq!(actual, expected[&row.name], "{}", row.name);
        }
    }
    let mut request = request();
    request.executor_ids.push("B1002".into());
    request.scope.groups.insert("B1002".into());
    request.row_duals = problem.row_names().into_iter().map(|name| (name, 0.0)).collect();
    request.row_duals.insert("recovery_passenger_transfer_0_1".into(), -500.0);
    request.row_duals.insert("recovery_passenger_cover_0".into(), 350.0);
    request.row_duals.insert("recovery_passenger_capacity_1".into(), 5.0);
    let policy = RecoveryPricingPolicy::new(problem.clone(), None);
    let result = policy.price(&request, &PathSearchBudget::default()).unwrap();
    for candidate in columns {
        let entry = problem.entry(&candidate).unwrap();
        let expected = problem.column_coefficients(&candidate).unwrap().iter().fold(candidate.cost, |value, (key, coefficient)| value - request.row_duals[key] * coefficient);
        assert_eq!(result.reduced_costs[entry.id.as_str()], expected);
    }
    request.row_duals.remove("recovery_passenger_capacity_0");
    assert!(policy.price(&request, &PathSearchBudget::default()).is_err());
}

#[test]
fn passenger_rule_refresh_failure_rolls_back_gantt_columns_payloads_and_downstream_extensions() {
    let problem = transfer_problem(60);
    let mut context = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut model = MetaModel::new("passenger-atomic-refresh");
    context.register(&mut model).unwrap();
    model.retain_constraints(|row| row.name != "recovery_passenger_cover_0");
    model.add_eq_constraint(&[], 0.0, "recovery_passenger_cover_0").unwrap();
    let before = model.try_to_linear_triad_model().unwrap();
    let initial = vec![single(&problem, 0, 0), single(&problem, 1, 1)];
    assert!(context.add_candidates(0, initial.clone(), &mut model).is_err());
    let after = model.try_to_linear_triad_model().unwrap();
    assert_eq!(context.column_count(), 0);
    assert_eq!(before.c, after.c);
    assert_eq!(before.A.rows.len(), after.A.rows.len());
    assert_eq!(before.num_variables(), after.num_variables());
    model.retain_constraints(|row| row.name != "recovery_passenger_cover_0");
    context.add_candidates(0, initial, &mut model).unwrap();
    let (_, values) = master_optimum(&model).unwrap();
    let variable = context.master().compilation.bunch_x_map[&0];
    model.add_eq_constraint(&[(variable, 1.0)], 0.0, "downstream_passenger_restriction").unwrap();
    assert!(context.extract_solution(&model, &values).is_err());
    context.remove_columns(&[0], &mut model).unwrap();
    assert!(model.constraints().iter().any(|row| row.name == "downstream_passenger_restriction"));
}

struct ReportSolver(SolveReport<f64>);

impl ColumnGenerationSolver for ReportSolver {
    fn name(&self) -> &str { "demo4-report-contract" }
    fn solve_milp_report_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<SolveReport<f64>> { Ok(self.0.clone()) }
    fn solve_milp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<FeasibleSolution> { FeasibleSolution::try_from_report(&self.0) }
    fn solve_lp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<LPResult> { panic!("invalid initial report must not reach pricing") }
}

#[test]
fn recovery_application_preserves_normal_limits_and_rejects_technical_or_illegal_incumbents() {
    for reason in [TerminationReason::TimeLimit, TerminationReason::Cancelled, TerminationReason::Interrupted] {
        let source = SolveReport::builder(ProblemStatus::Unknown, reason).build().unwrap();
        let report = RecoveryApplicationService::new(ReportSolver(source)).solve(problem(CostCalculator), Default::default()).unwrap();
        assert!(!report.has_incumbent());
        assert_eq!(report.termination_reason, reason);
    }
    for reason in [TerminationReason::BackendFailure, TerminationReason::NumericalFailure] {
        let source = SolveReport::builder(ProblemStatus::Unknown, reason).build().unwrap();
        assert!(RecoveryApplicationService::new(ReportSolver(source)).solve(problem(CostCalculator), Default::default()).is_err());
    }
    let source = FeasibleSolution::new(0.0, vec![0.0, 0.0]).to_solve_report("illegal-uncovered-incumbent").unwrap();
    assert!(RecoveryApplicationService::new(ReportSolver(source)).solve(problem(CostCalculator), Default::default()).is_err());
}

#[cfg(feature = "demo4-gurobi-cg")]
#[test]
#[ignore = "requires native Gurobi runtime and license; run with --include-ignored"]
fn demo4_gurobi_nonempty_internal_events_and_effective_link_match_offline_oracle() {
    use ospf_rust_framework::solver::gurobi_extension::GurobiColumnGenerationSolver;
    let base = problem(CostCalculator);
    let linked = Arc::new(RecoveryProblem::new(base.configuration().clone(), base.domains().to_vec(),
        vec![Link { split_cost: None, from_task: "F1".into(), to_task: "F2".into(), link_type: LinkType::Connecting { min_connection_time: Duration::minutes(45) } }], CostCalculator).unwrap());
    let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(linked.clone(), RecoveryApplicationConfiguration {
        column_generation: ColumnGenerationPolicy { max_iterations: 5, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(300.0));
    assert!(report.proof.is_none());
    let solution = report.solution.unwrap().value.unwrap();
    assert!(!solution.pricing.is_empty());
    assert!(solution.schedule.canceled.is_empty());
    let bunch = &solution.schedule.selected[0].bunch;
    assert!(bunch.arrived_when(&airport("B"), &(at(9, 0), at(9, 0))));
    assert!(bunch.departed_when(&airport("B"), &(at(10, 0), at(10, 0))));
    assert_eq!(bunch.recovery("F1").unwrap().original().plan.time.unwrap().0, at(7, 0));
    let invalid_link = Arc::new(RecoveryProblem::new(base.configuration().clone(), base.domains().to_vec(),
        vec![Link { split_cost: None, from_task: "F1".into(), to_task: "F2".into(), link_type: LinkType::Connecting { min_connection_time: Duration::hours(2) } }], CostCalculator).unwrap());
    let result = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(invalid_link, Default::default()).unwrap();
    assert_eq!(result.solution.unwrap().objective_value, Some(2000.0));
}

#[cfg(feature = "demo4-gurobi-cg")]
#[test]
#[ignore = "requires native Gurobi runtime and license; run with --include-ignored"]
fn demo4_gurobi_cross_aircraft_passengers_drive_real_rmp_duals_and_new_columns() {
    use ospf_rust_framework::solver::gurobi_extension::GurobiColumnGenerationSolver;
    let problem = transfer_problem(60);
    let seed = vec![single(&problem, 0, 0), single(&problem, 1, 2)];
    let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(problem.clone(), RecoveryApplicationConfiguration {
        initial: seed, column_generation: ColumnGenerationPolicy { max_iterations: 5, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(200.0));
    assert!(report.proof.is_none());
    let solution = report.solution.unwrap().value.unwrap();
    assert!(solution.candidate_ids.len() > 2);
    assert!(solution.pricing.iter().any(|round| !round.batch.columns.is_empty()));
    assert_eq!(solution.schedule.selected.len(), 2);
    let bunches = solution.schedule.selected.iter().map(|candidate| candidate.bunch.clone()).collect::<Vec<_>>();
    assert!(FeasibilityJudger.judge_passengers(&bunches, &[passenger(60)]).unwrap().feasible);
    let mut context = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut model = MetaModel::new("all-variants-oracle");
    context.register(&mut model).unwrap();
    context.add_candidates(0, vec![single(&problem, 0, 0), single(&problem, 1, 0), single(&problem, 1, 1), single(&problem, 1, 2)], &mut model).unwrap();
    assert_eq!(master_optimum(&model).unwrap().0, 200.0);
}
