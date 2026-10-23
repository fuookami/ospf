//! 自动初始列和全生命周期锁定回归。 / Automatic initial columns and full-lifecycle locking regressions.

use crate::framework::demo4::domain::bunch_generation::initial::{RecoveryInitialBunchGenerator, RecoveryInitialConfiguration};
use crate::framework::demo4::domain::rule::model::Lock;
use super::fleet::{complete, fresh};
use super::*;

fn departure_lock(id: &str, task: &str, hour: i64) -> Lock {
    Lock { id: id.into(), task_id: task.into(), departure_time: at(hour, 0) }
}

#[test]
fn departure_locks_use_actual_times_in_initial_pricing_replay_and_production_extraction() {
    let source = fresh();
    let mut domains = source.domains().to_vec();
    let current = &domains[0].candidates[0].recovery;
    let alternative = RecoveryActivityCandidate { id: "earlier-F1".into(), recovery: FlightTaskRecovery::new(
        current.original().clone(), current.aircraft().clone(), current.route().clone(), (at(7, 0), at(8, 0)),
    ).unwrap() };
    domains[0].candidates.push(alternative);
    let problem = Arc::new(RecoveryProblem::new(source.configuration().clone(), domains, vec![], CostCalculator).unwrap()
        .with_departure_locks(vec![departure_lock("fixed-F1", "F1", 7)]).unwrap());
    assert_eq!(problem.locked_departure("F1"), Some(at(7, 0)));
    assert_eq!(problem.locked_departure("F2"), None);
    let original = complete(&source);
    let initial = RecoveryInitialBunchGenerator.generate(&problem, &RecoveryInitialConfiguration { originals: vec![original.clone()] }, &PathSearchBudget::default()).unwrap();
    assert_eq!(initial.batch.columns[0].bunch.recovery("F1").unwrap().time().0, at(7, 0));
    let mut request = request();
    request.row_duals.insert("task_compilation_0".into(), 1000.0);
    let priced = RecoveryPricingPolicy::new(problem.clone(), None).price(&request, &PathSearchBudget::default()).unwrap();
    assert!(priced.batch.columns.iter().any(|entry| entry.task_indices.contains(&0)));
    for entry in &priced.batch.columns {
        let candidate = problem.resolve_candidate(entry).unwrap();
        if let Some(activity) = candidate.bunch.recovery("F1") { assert_eq!(activity.time().0, at(7, 0)); }
    }
    assert!(problem.resolve_candidate(&problem.entry(&original).unwrap()).is_err());
    let mut context = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut model = MetaModel::new("departure-lock-recovery");
    context.register(&mut model).unwrap();
    assert!(context.add_candidates(0, vec![original.clone()], &mut model).is_err());
    context.add_candidates(0, initial.batch.columns, &mut model).unwrap();
    let (_, values) = master_optimum(&model).unwrap();
    let mut schedule = context.extract_solution(&model, &values).unwrap();
    schedule.selected = vec![original];
    use crate::framework::demo4::domain::bunch_compilation::context::RecoveryBunchResolver;
    assert!(problem.validate_schedule(&schedule).is_err());
    schedule.selected.clear();
    assert!(problem.validate_schedule(&schedule).is_err());
}

#[test]
fn departure_and_aircraft_locks_compose_in_either_builder_order_and_clear_independently() {
    let locks = || BTreeMap::from([("B1001".into(), vec!["F2".into()])]);
    let time = || vec![departure_lock("fixed-F1", "F1", 8)];
    let left = fresh().with_locked_tasks(locks()).unwrap().with_departure_locks(time()).unwrap();
    let right = fresh().with_departure_locks(time()).unwrap().with_locked_tasks(locks()).unwrap();
    assert_eq!(left.configuration().cancellation_costs, vec![None, None]);
    assert_eq!(left.configuration().cancellation_costs, right.configuration().cancellation_costs);
    let cleared = left.with_departure_locks(vec![]).unwrap();
    assert_eq!(cleared.configuration().cancellation_costs, vec![Some(1000.0), None]);
    let cleared = right.with_locked_tasks(BTreeMap::new()).unwrap();
    assert_eq!(cleared.configuration().cancellation_costs, vec![None, Some(1000.0)]);
    let changed = cleared.with_departure_locks(vec![departure_lock("fixed-F2", "F2", 10)]).unwrap();
    assert_eq!(changed.configuration().cancellation_costs, vec![Some(1000.0), None]);
    assert_eq!(changed.locked_departure("F1"), None);
    for locks in [vec![departure_lock("", "F1", 8)], vec![departure_lock("unknown", "unknown", 8)],
        vec![departure_lock("same", "F1", 8), departure_lock("same", "F2", 10)],
        vec![departure_lock("one", "F1", 8), departure_lock("two", "F1", 9)]]
    { assert!(fresh().with_departure_locks(locks).is_err()); }
}

#[test]
fn absent_departure_variant_is_normal_no_incumbent_not_an_input_error_or_feasibility_proof() {
    let problem = fresh().with_departure_locks(vec![departure_lock("unavailable-F1", "F1", 9)]).unwrap();
    let report = RecoveryApplicationService::new(EnumeratingSolver::default()).solve(Arc::new(problem), RecoveryApplicationConfiguration {
        initial_generation: Some(Default::default()), ..Default::default()
    }).unwrap();
    assert!(!report.has_incumbent());
    assert_eq!(report.problem_status, ProblemStatus::Unknown);
    assert!(report.proof.is_none());
}

#[test]
fn executed_predecessor_cannot_be_registered_as_a_selectable_recovery_task_or_generated_again() {
    let source = fresh();
    let mut domains = source.domains().to_vec();
    let previous = FlightTaskRecovery::new(domains[0].candidates[0].recovery.original().clone(), aircraft(),
        Route { dep: airport("X"), arr: airport("A") }, (at(5, 0), at(6, 0))).unwrap();
    domains[0].previous = Some(previous.clone());
    assert!(RecoveryProblem::new(source.configuration().clone(), domains, vec![], CostCalculator).is_err());
    let usability = usability();
    let request = RecoveryPathRequest {
        aircraft: &aircraft(), candidates: &candidates(),
        rules: BunchRecoveryRules { usability: &usability, previous: Some(&previous), links: &[] },
        cost: &CostCalculator, scope: scope(), maximum_results: None,
    };
    let checked = FeasibilityJudger.judge_bunch(&bunch(), &request.rules).unwrap();
    assert!(!checked.feasible);
    assert_eq!(checked.violations, ["executed predecessor is repeated in recovery activities"]);
    let generated = RecoveryBunchGenerationContext.generate(&request, &PathSearchBudget::default()).unwrap();
    assert!(generated.batch.columns.iter().all(|candidate| !candidate.bunch.contains("F1")));
    assert_eq!(generated.batch.no_improving_columns, None);
}

#[test]
fn executed_predecessor_identity_is_unique_across_aircraft_but_valid_history_is_not_pending_demand() {
    let source = transfer_problem(0);
    let mut domains = source.domains().to_vec();
    for domain in &mut domains {
        let executed = activity("executed", "X", "A", 5, 6);
        domain.previous = Some(FlightTaskRecovery::new(executed.original().clone(), domain.aircraft.clone(),
            executed.route().clone(), executed.time()).unwrap());
    }
    assert!(RecoveryProblem::new(source.configuration().clone(), domains.clone(), vec![], CostCalculator).is_err());
    domains[1].previous = None;
    let valid = RecoveryProblem::new(source.configuration().clone(), domains, vec![], CostCalculator).unwrap();
    assert!(!valid.configuration().tasks.iter().any(|task| task.plan.id == "executed"));
    let mut domains = valid.domains().to_vec();
    let executed = domains[0].previous.as_ref().unwrap();
    let mut original = executed.original().clone();
    original.plan.id.clear();
    domains[0].previous = Some(FlightTaskRecovery::new(original, aircraft(), executed.route().clone(), executed.time()).unwrap());
    assert!(RecoveryProblem::new(valid.configuration().clone(), domains, vec![], CostCalculator).is_err());
}

#[test]
fn initial_generation_retains_original_order_but_uses_actual_domain_records_and_costs() {
    let problem = fresh();
    let current = complete(&problem);
    let original = FlightTaskBunch::new("original-before-recovery".into(), aircraft(), current.bunch.activities().iter().map(|activity| {
        FlightTaskRecovery::new(activity.original().clone(), aircraft(), activity.route().clone(),
            (activity.time().0 - Duration::hours(1), activity.time().1 - Duration::hours(1))).unwrap()
    }).collect()).unwrap();
    let generated = RecoveryInitialBunchGenerator.generate(&problem, &RecoveryInitialConfiguration {
        originals: vec![RecoveryBunchCandidate { bunch: original, cost: 300.0 }],
    }, &PathSearchBudget::default()).unwrap();
    assert_eq!(generated.batch.completion, ColumnGenerationCompletion::Exhausted);
    assert_eq!(generated.batch.no_improving_columns, None);
    assert_eq!(generated.batch.columns.len(), 1);
    let selected = &generated.batch.columns[0];
    assert_eq!(selected.bunch.task_ids().collect::<Vec<_>>(), vec!["F1", "F2"]);
    assert_eq!(selected.bunch.recovery("F1").unwrap().time().0, at(8, 0));
    assert_eq!(selected.cost, 300.0);
    assert_eq!(generated.searches[0].search.diagnostics.total_results, 3);
    problem.resolve_candidate(&problem.entry(selected).unwrap()).unwrap();
}

#[test]
fn initial_fallback_preserves_locks_and_all_later_pricing_and_registration_use_the_same_locked_domain() {
    let source = fresh();
    let original = complete(&source);
    let mut domains = source.domains().to_vec();
    domains[0].candidates.truncate(1);
    let problem = Arc::new(RecoveryProblem::new(source.configuration().clone(), domains, vec![], CostCalculator).unwrap()
        .with_locked_tasks(BTreeMap::from([("B1001".into(), vec!["F1".into()])])).unwrap());
    let generated = RecoveryInitialBunchGenerator.generate(&problem, &RecoveryInitialConfiguration { originals: vec![original] }, &PathSearchBudget::default()).unwrap();
    assert_eq!(generated.batch.columns[0].bunch.task_ids().collect::<Vec<_>>(), vec!["F1"]);
    assert_eq!(problem.configuration().cancellation_costs, vec![None, Some(1000.0)]);
    let mut request = request();
    request.row_duals.insert("task_compilation_0".into(), 1000.0);
    let priced = RecoveryPricingPolicy::new(problem.clone(), None).price(&request, &PathSearchBudget::default()).unwrap();
    assert!(priced.batch.columns.iter().all(|entry| entry.task_indices.contains(&0)));
    let mut context = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut model = MetaModel::new("locked-initial-and-pricing");
    context.register(&mut model).unwrap();
    let idle = RecoveryBunchCandidate { bunch: FlightTaskBunch::new("idle".into(), aircraft(), vec![]).unwrap(), cost: 0.0 };
    assert!(context.add_candidates(0, vec![idle], &mut model).is_err());
    context.add_candidates(0, generated.batch.columns, &mut model).unwrap();
    let (objective, values) = master_optimum(&model).unwrap();
    assert_eq!(objective, 1100.0);
    assert_eq!(context.extract_solution(&model, &values).unwrap().canceled[0].plan.id, "F2");
}

#[test]
fn locked_task_cannot_move_to_another_aircraft_even_with_a_negative_reduced_cost() {
    let source = fresh();
    let transfer = transfer_problem(0);
    let mut configuration = source.configuration().clone();
    configuration.aircraft.push(transfer.domains()[1].aircraft.clone());
    let mut domains = source.domains().to_vec();
    domains.push(transfer.domains()[1].clone());
    let problem = Arc::new(RecoveryProblem::new(configuration, domains, vec![], CostCalculator).unwrap()
        .with_locked_tasks(BTreeMap::from([("B1001".into(), vec!["F2".into()])])).unwrap());
    let wrong = single(&problem, 1, 0);
    assert!(problem.resolve_candidate(&problem.entry(&wrong).unwrap()).is_err());
    let mut request = request();
    request.executor_ids.push("B1002".into());
    request.scope.groups.insert("B1002".into());
    request.row_duals = problem.row_names().into_iter().map(|name| (name, 0.0)).collect();
    request.row_duals.insert("task_compilation_1".into(), 2000.0);
    let priced = RecoveryPricingPolicy::new(problem, None).price(&request, &PathSearchBudget::default()).unwrap();
    assert!(!priced.batch.columns.is_empty());
    assert!(priced.batch.columns.iter().all(|entry| entry.executor_id.as_str() == "B1001" && entry.task_indices.contains(&1)));
}

#[test]
fn initial_generation_budget_interrupt_and_unrecoverable_locks_never_create_a_feasibility_proof() {
    let problem = fresh().with_locked_tasks(BTreeMap::from([("B1001".into(), vec!["F1".into()])])).unwrap();
    let budget = PathSearchBudget { maximum_labels: Some(1), ..Default::default() };
    let generated = RecoveryInitialBunchGenerator.generate(&problem, &Default::default(), &budget).unwrap();
    assert_eq!(generated.batch.completion, ColumnGenerationCompletion::BudgetLimit);
    assert!(generated.batch.columns.is_empty());
    assert_eq!(generated.batch.no_improving_columns, None);
    let cancelled = || true;
    let generated = RecoveryInitialBunchGenerator.generate(&problem, &Default::default(), &PathSearchBudget { interrupted: Some(&cancelled), ..Default::default() }).unwrap();
    assert_eq!(generated.batch.completion, ColumnGenerationCompletion::Interrupted);
    assert_eq!(generated.batch.no_improving_columns, None);
    let report = RecoveryApplicationService::new(EnumeratingSolver::default()).solve(Arc::new(problem), RecoveryApplicationConfiguration {
        initial_generation: Some(Default::default()), maximum_labels: Some(1), ..Default::default()
    }).unwrap();
    assert!(!report.has_incumbent());
    assert_eq!(report.problem_status, ProblemStatus::Unknown);
    assert_eq!(report.diagnostics.extensions["demo4.initialGenerationCompletion"], "BudgetLimit");
    assert_eq!(report.diagnostics.extensions["demo4.initialGenerationColumns"], "0");
}

#[test]
fn initial_empty_domains_are_explicit_and_invalid_originals_or_locks_fail_without_registration() {
    let empty = RecoveryProblem::new(RecoveryMasterConfiguration { tasks: vec![], aircraft: vec![], cancellation_costs: vec![] }, vec![], vec![], CostCalculator).unwrap();
    let generated = RecoveryInitialBunchGenerator.generate(&empty, &Default::default(), &PathSearchBudget::default()).unwrap();
    assert!(generated.batch.columns.is_empty());
    assert_eq!(generated.batch.completion, ColumnGenerationCompletion::Exhausted);
    assert_eq!(generated.batch.no_improving_columns, None);
    let report = RecoveryApplicationService::new(EnumeratingSolver::default()).solve(Arc::new(empty), RecoveryApplicationConfiguration {
        initial_generation: Some(Default::default()), ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(0.0));
    assert!(report.solution.unwrap().value.unwrap().schedule.selected.is_empty());
    for locks in [BTreeMap::from([("unknown".into(), vec!["F1".into()])]),
        BTreeMap::from([("B1001".into(), vec!["unknown".into()])]),
        BTreeMap::from([("B1001".into(), vec!["F1".into(), "F1".into()])])]
    { assert!(fresh().with_locked_tasks(locks).is_err()); }
    let cleared = fresh().with_locked_tasks(BTreeMap::from([("B1001".into(), vec!["F1".into()])])).unwrap().with_locked_tasks(BTreeMap::new()).unwrap();
    assert_eq!(cleared.configuration().cancellation_costs[0], Some(1000.0));
    let original = complete(&fresh());
    assert!(RecoveryInitialBunchGenerator.generate(&fresh(), &RecoveryInitialConfiguration { originals: vec![original.clone(), original] }, &PathSearchBudget::default()).is_err());
}

#[test]
fn automatic_initial_columns_use_the_existing_gantt_initial_milp_and_preserve_search_evidence() {
    let source = fresh();
    let original = complete(&source);
    let problem = Arc::new(source.with_original_fleet_balance(std::slice::from_ref(&original), None).unwrap()
        .with_locked_tasks(BTreeMap::from([("B1001".into(), vec!["F1".into()])])).unwrap());
    let solver = EnumeratingSolver::default();
    let calls = solver.0.clone();
    let report = RecoveryApplicationService::new(solver).solve(problem, RecoveryApplicationConfiguration {
        initial_generation: Some(RecoveryInitialConfiguration { originals: vec![original] }),
        column_generation: ColumnGenerationPolicy { max_iterations: 1, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(300.0));
    assert!(report.proof.is_none());
    let result = report.solution.unwrap().value.unwrap();
    assert_eq!(calls.lock().unwrap()[0], (true, 3));
    assert_eq!(result.schedule.fleet_balance[0].available, 1);
    let initial = result.initial_generation.unwrap();
    assert_eq!(initial.batch.columns.len(), 1);
    assert_eq!(initial.batch.no_improving_columns, None);
    assert!(!initial.searches[0].search.paths.is_empty());
}

#[cfg(feature = "demo4-gurobi-cg")]
#[test]
#[ignore = "requires native Gurobi runtime and license; run with --include-ignored"]
fn demo4_gurobi_automatic_seed_original_fleet_and_locks_share_the_real_gantt_loop() {
    use ospf_rust_framework::solver::gurobi_extension::GurobiColumnGenerationSolver;
    let source = fresh();
    let original = complete(&source);
    let problem = Arc::new(source.with_original_fleet_balance(std::slice::from_ref(&original), Some(500.0)).unwrap()
        .with_locked_tasks(BTreeMap::from([("B1001".into(), vec!["F1".into()])])).unwrap()
        .with_departure_locks(vec![departure_lock("actual-F1", "F1", 8)]).unwrap());
    let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(problem, RecoveryApplicationConfiguration {
        initial_generation: Some(RecoveryInitialConfiguration { originals: vec![original] }),
        column_generation: ColumnGenerationPolicy { max_iterations: 3, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(300.0));
    assert!(report.proof.is_none());
    let result = report.solution.unwrap().value.unwrap();
    assert_eq!(result.schedule.fleet_balance[0].shortage, 0);
    assert!(result.schedule.canceled.is_empty());
    assert_eq!(result.schedule.selected[0].bunch.recovery("F1").unwrap().time().0, at(8, 0));
    assert_eq!(result.initial_generation.unwrap().batch.columns.len(), 1);
    assert!(!result.pricing.is_empty());
}

#[cfg(feature = "demo4-gurobi-cg")]
#[test]
#[ignore = "requires native Gurobi runtime and license; run with --include-ignored"]
fn demo4_gurobi_executed_history_sets_actual_origin_without_reselecting_or_recharging_flight_allowances() {
    use ospf_rust_framework::solver::gurobi_extension::GurobiColumnGenerationSolver;
    use crate::framework::demo4::domain::rule::model::{FlowControl, FlowControlCapacity, FlowControlScene};
    let source = fresh();
    let original = complete(&source);
    let mut domains = source.domains().to_vec();
    domains[0].usability.location = airport("X");
    domains[0].usability.flight_cycle_periods = vec![FlightCyclePeriod { expiration_time: at(13, 0),
        remaining_flight_hour: Some(FlightHour::new(Duration::hours(3))), remaining_flight_cycle: Some(FlightCycle::new(2)) }];
    domains[0].previous = Some(activity("executed", "X", "A", 5, 6));
    let problem = Arc::new(RecoveryProblem::new(source.configuration().clone(), domains, vec![], CostCalculator).unwrap()
        .with_flow_controls(vec![FlowControl { id: "actual-origin-stay".into(), airport: "A".into(),
            time_range: (at(5, 0), at(8, 0)), scene: FlowControlScene::Stay, condition: None,
            capacity: FlowControlCapacity { max_capacity: 1, current_usage: 0, interval: Duration::hours(3) }, over_cost: None,
        }]).unwrap());
    let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(problem, RecoveryApplicationConfiguration {
        initial_generation: Some(RecoveryInitialConfiguration { originals: vec![original] }),
        column_generation: ColumnGenerationPolicy { max_iterations: 3, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(300.0));
    assert!(report.proof.is_none());
    let solution = report.solution.unwrap().value.unwrap();
    assert!(!solution.pricing.is_empty());
    assert_eq!(solution.schedule.selected.len(), 1);
    let bunch = &solution.schedule.selected[0].bunch;
    assert_eq!(bunch.task_ids().collect::<Vec<_>>(), ["F1", "F2"]);
    assert_eq!(bunch.dep(), Some(&airport("A")));
    assert_eq!(bunch.flight_hour(), FlightHour::new(Duration::hours(3)));
    assert_eq!(bunch.flight_cycle(), FlightCycle::new(2));
    assert_eq!(solution.schedule.flow_controls[0].recovery_usage, 1);
}
