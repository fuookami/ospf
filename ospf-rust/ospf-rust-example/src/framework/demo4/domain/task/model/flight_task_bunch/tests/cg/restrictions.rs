//! 限制条件、完整定价行、列事务和生产复核回归。 / Restriction conditions, full pricing rows, column transactions and production-validation regressions.

use std::collections::BTreeSet;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::variable::{Binary, VariableRange};
use crate::framework::demo4::domain::bunch_compilation::context::RecoveryBunchResolver;
use crate::framework::demo4::domain::bunch_compilation::restrictions::RecoveryRestrictions;
use crate::framework::demo4::domain::rule::model::{
    GeneralRestrictionCondition, GeneralRestrictionPredicate, RelationRestrictionCategory, Restriction,
    RestrictionAssignment, RestrictionCheckingResult, RestrictionRule, RestrictionType,
};
use super::*;

fn set(values: &[&str]) -> BTreeSet<String> { values.iter().map(|value| value.to_string()).collect() }

fn relation(cost: Option<f64>) -> Restriction {
    Restriction { id: "A-B".into(), restriction_type: RestrictionType::Strong, weight: 2.0, cost,
        rule: RestrictionRule::Relation { category: RelationRestrictionCategory::BlackList,
            departure: "A".into(), arrival: "B".into(), aircraft: set(&["B1001"]) } }
}

fn general(condition: GeneralRestrictionCondition) -> Restriction {
    Restriction { id: "general".into(), restriction_type: RestrictionType::Weak, weight: 2.0, cost: Some(100.0),
        rule: RestrictionRule::General { condition: Some(condition), enabled_aircraft: None, disabled_aircraft: None } }
}

fn source(restrictions: Vec<Restriction>) -> Arc<RecoveryProblem<CostCalculator>> {
    Arc::new(fleet::fresh().with_restrictions(restrictions).unwrap())
}

fn context(source: &Arc<RecoveryProblem<CostCalculator>>) -> (BunchCompilationContext, MetaModel<f64>) {
    let mut context = BunchCompilationContext::with_resolver(source.configuration().clone(), source.clone()).unwrap();
    let mut model = MetaModel::new("recovery-restrictions");
    context.register(&mut model).unwrap();
    (context, model)
}

// 只枚举这些小夹具的有限整数范围，包含 2 次违反，不以二元 oracle 冒充非二元计数。 / Enumerate finite integer ranges of tiny fixtures, including two violations rather than using a binary oracle for nonbinary counts.
fn optimum(model: &MetaModel<f64>) -> (f64, Vec<f64>) {
    let matrix = model.try_to_linear_triad_model().unwrap();
    assert!(matrix.num_variables() < 12 && matrix.var_types.iter().all(|kind| kind.is_integer()));
    let mut states = vec![vec![]];
    for index in 0..matrix.num_variables() {
        assert!(matrix.lb[index] >= 0.0 && matrix.ub[index] <= 2.0);
        states = states.into_iter().flat_map(|state| (matrix.lb[index].ceil() as u64..=matrix.ub[index].floor() as u64).map(move |value| {
            let mut next = state.clone(); next.push(value as f64); next
        })).collect();
    }
    states.into_iter().filter(|values| matrix.A.rows.iter().zip(&matrix.b).all(|(row, bound)| row.entries.iter()
        .map(|(index, coefficient)| values[*index] * coefficient).sum::<f64>() <= *bound + 1e-9))
        .map(|values| (values.iter().zip(&matrix.c).map(|(value, coefficient)| value * coefficient).sum::<f64>(), values))
        .min_by(|left, right| left.0.total_cmp(&right.0)).unwrap()
}

#[test]
fn relation_checks_actual_route_aircraft_category_blacklist_and_whitelist_without_panics() {
    let bunch = bunch();
    let activity = &bunch.activities()[0];
    let restricted = relation(None);
    assert_eq!(restricted.check_task(activity.original()), RestrictionCheckingResult::NotMatter);
    assert!(matches!(restricted.check(activity), RestrictionCheckingResult::Violate { .. }));
    assert!(restricted.related(&aircraft()));
    let mut other = aircraft(); other.reg_no.0 = "B1002".into();
    let actual = FlightTaskRecovery::new(activity.original().clone(), other.clone(), activity.route().clone(), activity.time()).unwrap();
    assert_eq!(restricted.check(&actual), RestrictionCheckingResult::NotViolate);
    let white = Restriction { rule: RestrictionRule::Relation { category: RelationRestrictionCategory::WhiteList,
        departure: "A".into(), arrival: "B".into(), aircraft: set(&["B1001"]) }, ..restricted.clone() };
    assert_eq!(white.check(activity), RestrictionCheckingResult::NotViolate);
    assert!(matches!(white.check(&actual), RestrictionCheckingResult::Violate { .. }));
    let mut view = RestrictionAssignment::from_recovery(activity); view.aircraft = None;
    assert_eq!(restricted.check_assignment(&view), RestrictionCheckingResult::NotMatter);
    let mut original = activity.original().clone(); original.task_type.category = FlightTaskCategory::Maintenance;
    let ground = FlightTaskRecovery::new(original, aircraft(), activity.route().clone(), activity.time()).unwrap();
    assert_eq!(restricted.check(&ground), RestrictionCheckingResult::NotMatter);
    let task = FlightTaskImpl { plan: FlightTaskPlanImpl { dep: airport("A"), arr: airport("B"), ..activity.original().plan.clone() }, ..activity.original().clone() };
    assert_eq!(restricted.check_aircraft(&task, &other), RestrictionCheckingResult::NotViolate);
    assert!(matches!(restricted.check_aircraft(&task, &aircraft()), RestrictionCheckingResult::Violate { .. }));
}

#[test]
fn general_conditions_use_actual_events_inclusive_windows_full_intervals_and_aircraft_scopes() {
    let bunch = bunch();
    let activity = &bunch.activities()[0];
    let assigned = RestrictionAssignment::from_recovery(activity);
    for condition in [
        GeneralRestrictionCondition::Flights(set(&["F1"])),
        GeneralRestrictionCondition::Departure { airports: set(&["A"]), time: Some((at(8, 0), at(8, 0))) },
        GeneralRestrictionCondition::Arrival { airports: set(&["B"]), time: Some((at(9, 0), at(9, 0))) },
        GeneralRestrictionCondition::Bidirectional { first: set(&["B"]), second: set(&["A"]), time: Some((at(8, 0), at(9, 0))) },
        GeneralRestrictionCondition::EnabledAircraft(set(&["B1001"])),
        GeneralRestrictionCondition::DisabledAircraft(set(&["B1002"])),
    ] {
        assert!(condition.matches(&assigned));
        assert!(general(condition).check(activity).violated());
    }
    let outside = GeneralRestrictionCondition::Bidirectional { first: set(&["A"]), second: set(&["B"]), time: Some((at(8, 30), at(9, 0))) };
    assert!(!outside.matches(&assigned));
    let mut absent = assigned; absent.time = None;
    let timed = GeneralRestrictionCondition::Arrival { airports: set(&["B"]), time: Some((at(9, 0), at(9, 0))) };
    assert!(!timed.matches(&absent));
    assert_eq!(general(timed).check_task(activity.original()), RestrictionCheckingResult::NotMatter);
    let mut scoped = general(GeneralRestrictionCondition::All(vec![]));
    scoped.rule = RestrictionRule::General { condition: Some(GeneralRestrictionCondition::Flights(set(&["F1"]))),
        enabled_aircraft: Some(set(&["B1001"])), disabled_aircraft: Some(set(&["B1001"])) };
    assert_eq!(scoped.check(activity), RestrictionCheckingResult::NotMatter);
    scoped.rule = RestrictionRule::General { condition: None, enabled_aircraft: None, disabled_aircraft: None };
    assert_eq!(scoped.check(activity), RestrictionCheckingResult::NotMatter);
    scoped.rule = RestrictionRule::General { condition: None, enabled_aircraft: Some(set(&[])), disabled_aircraft: None };
    assert_eq!(scoped.check(activity), RestrictionCheckingResult::NotMatter);
}

#[test]
fn severity_weighted_penalties_nonbinary_counts_and_unpriced_soft_diagnostics_match_oracles() {
    for (severity, cost, objective, selected, penalty) in [
        (RestrictionType::Strong, Some(50.0), 400.0, 1, 100.0),
        (RestrictionType::Strong, Some(1200.0), 2000.0, 0, 0.0),
        (RestrictionType::Weak, None, 300.0, 1, 0.0),
        (RestrictionType::ViolableStrong, None, 300.0, 1, 0.0),
    ] {
        let source = source(vec![Restriction { restriction_type: severity, ..relation(cost) }]);
        let (mut context, mut model) = context(&source);
        context.add_candidates(0, vec![fleet::complete(&source)], &mut model).unwrap();
        let (actual, values) = optimum(&model);
        assert_eq!(actual, objective);
        let result = context.extract_solution(&model, &values).unwrap();
        assert_eq!(result.selected.len(), selected);
        assert_eq!(result.restrictions[0].penalty, penalty);
        assert_eq!(result.restrictions[0].weighted_violations, 2.0 * selected as f64);
        source.validate_schedule(&result).unwrap();
    }
    let source = source(vec![general(GeneralRestrictionCondition::All(vec![]))]);
    let (mut context, mut model) = context(&source);
    context.add_candidates(0, vec![fleet::complete(&source)], &mut model).unwrap();
    let (actual, values) = optimum(&model);
    assert_eq!(actual, 700.0);
    assert_eq!(values[2], 2.0);
    let result = context.extract_solution(&model, &values).unwrap();
    assert_eq!(result.restrictions[0].violated_tasks, ["F1", "F2"]);
    assert_eq!(result.restrictions[0].weighted_violations, 4.0);
    assert_eq!(result.restrictions[0].penalty, 400.0);
}

#[test]
fn hard_restrictions_persist_through_initial_generation_pricing_and_candidate_replay() {
    let source = source(vec![relation(None)]);
    let candidate = fleet::complete(&source);
    assert!(!FeasibilityJudger.judge_restrictions(&candidate.bunch, source.restrictions().unwrap().definitions()).feasible);
    let initial = crate::framework::demo4::domain::bunch_generation::initial::RecoveryInitialBunchGenerator.generate(&source,
        &crate::framework::demo4::domain::bunch_generation::initial::RecoveryInitialConfiguration { originals: vec![candidate.clone()] },
        &PathSearchBudget::default()).unwrap();
    assert!(initial.batch.columns.iter().all(|candidate| candidate.bunch.activities().is_empty()));
    let mut request = request(); request.row_duals = source.row_names().into_iter().map(|name| (name, 5000.0)).collect();
    let priced = RecoveryPricingPolicy::new(source.clone(), None).price(&request, &PathSearchBudget::default()).unwrap();
    assert!(priced.batch.columns.iter().all(|entry| entry.task_indices.is_empty()));
    assert!(priced.batch.no_improving_columns.is_none());
    let entry = source.entry(&candidate).unwrap();
    assert!(source.resolve_candidate(&entry).is_err());
    let (mut context, mut model) = context(&source);
    let before = model.as_basic().num_tokens();
    assert!(context.add_candidates(0, vec![candidate], &mut model).is_err());
    assert_eq!(model.as_basic().num_tokens(), before);
    assert_eq!(context.column_count(), 0);
    assert_eq!(optimum(&model).0, 2000.0);
    let base = problem(CostCalculator);
    let mut configuration = base.configuration().clone(); configuration.cancellation_costs = vec![None; 2];
    let infeasible = Arc::new(RecoveryProblem::new(configuration, base.domains().to_vec(), vec![], CostCalculator).unwrap()
        .with_restrictions(vec![relation(None)]).unwrap());
    let report = RecoveryApplicationService::new(EnumeratingSolver::default()).solve(infeasible, Default::default()).unwrap();
    assert_eq!(report.problem_status, ProblemStatus::Unknown);
    assert!(!report.has_incumbent());
}

#[test]
fn restriction_rows_and_full_signed_duals_match_registered_coefficients_and_require_zeros() {
    let source = source(vec![relation(Some(50.0)), general(GeneralRestrictionCondition::Flights(set(&["F2"])))]);
    let (mut context, mut model) = context(&source);
    let all = vec![single(&source, 0, 0), fleet::complete(&source)];
    context.add_candidates(0, all.clone(), &mut model).unwrap();
    for (index, candidate) in all.iter().enumerate() {
        let expected = source.column_coefficients(candidate).unwrap();
        let variable = context.master().compilation.bunch_x_map[&index];
        for row in model.constraints() {
            let actual = row.inequality.polynomial.monomials().iter().filter(|term| term.var_index() == variable).map(|term| *term.coefficient()).sum::<f64>();
            assert_eq!(actual, expected[&row.name], "{}", row.name);
        }
    }
    let mut request = request(); request.row_duals = source.row_names().into_iter().map(|name| (name, 0.0)).collect();
    request.row_duals.insert("recovery_restriction_0".into(), -91.0);
    let policy = RecoveryPricingPolicy::new(source.clone(), None);
    let priced = policy.price(&request, &PathSearchBudget::default()).unwrap();
    for candidate in all {
        let expected = source.column_coefficients(&candidate).unwrap().iter().fold(candidate.cost, |value, (name, coefficient)| value - request.row_duals[name] * coefficient);
        assert_eq!(priced.reduced_costs[source.entry(&candidate).unwrap().id.as_str()], expected);
    }
    request.row_duals.remove("recovery_restriction_1");
    assert!(policy.price(&request, &PathSearchBudget::default()).is_err());
}

#[test]
fn restriction_transactions_preserve_extensions_separate_contexts_and_refresh_retired_columns() {
    let source = source(vec![relation(Some(50.0))]);
    let (mut context, mut model) = context(&source);
    let extension = model.register_auto_variable_with_range::<Binary>("restriction_extension", VariableRange::fixed(1.0)).unwrap();
    model.add_eq_constraint(&[(extension, 1.0)], 1.0, "restriction_extension_rule").unwrap();
    model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(7.0, extension)], 0.0), "restriction_extension_cost"));
    let candidate = fleet::complete(&source);
    context.add_candidates(0, vec![candidate.clone()], &mut model).unwrap();
    let first = context.master().compilation.bunch_x_map[&0];
    assert_eq!(optimum(&model).0, 407.0);
    let count = model.as_basic().num_tokens(); context.register(&mut model).unwrap();
    assert_eq!(count, model.as_basic().num_tokens());
    context.remove_columns(&[0], &mut model).unwrap();
    assert_eq!(optimum(&model).0, 2007.0);
    context.add_candidates(1, vec![candidate.clone()], &mut model).unwrap();
    assert_ne!(first, context.master().compilation.bunch_x_map[&1]);
    assert_eq!(optimum(&model).0, 407.0);
    assert!(model.constraints().iter().any(|row| row.name == "restriction_extension_rule"));
    let (mut other, mut other_model) = self::context(&source);
    assert!(context.add_candidates(2, vec![candidate.clone()], &mut other_model).is_err());
    other_model.retain_constraints(|row| row.name != "recovery_restriction_0");
    other_model.add_eq_constraint(&[], 0.0, "recovery_restriction_0").unwrap();
    let before = other_model.try_to_linear_triad_model().unwrap();
    assert!(other.add_candidates(0, vec![candidate], &mut other_model).is_err());
    let after = other_model.try_to_linear_triad_model().unwrap();
    assert_eq!(before.c, after.c);
    assert_eq!(before.num_variables(), after.num_variables());
    assert_eq!(before.num_constraints(), after.num_constraints());
    assert_eq!(other.column_count(), 0);
}

#[derive(Debug)]
struct UncheckedResolver {
    base: Arc<RecoveryProblem<CostCalculator>>,
    restrictions: RecoveryRestrictions,
}

impl RecoveryBunchResolver for UncheckedResolver {
    fn resolve(&self, entry: &ospf_rust_framework_gantt_scheduling::domain::bunch_compilation::BunchEntry) -> ospf_rust_framework_gantt_scheduling::GanttResult<RecoveryBunchCandidate> {
        self.base.resolve_candidate(entry)
    }
    fn restrictions(&self) -> Option<&RecoveryRestrictions> { Some(&self.restrictions) }
}

#[test]
fn production_extraction_independently_rejects_illegal_counts_and_hard_hits_after_row_removal() {
    let source = source(vec![general(GeneralRestrictionCondition::All(vec![]))]);
    let (mut context, mut model) = context(&source);
    context.add_candidates(0, vec![fleet::complete(&source)], &mut model).unwrap();
    let (_, values) = optimum(&model);
    model.retain_constraints(|row| row.name != "recovery_restriction_0");
    for illegal in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 0.0, 1.0, 1.5, 3.0] {
        let mut forged = values.clone(); forged[2] = illegal;
        assert!(context.extract_solution(&model, &forged).is_err());
    }
    let mut result = context.extract_solution(&model, &values).unwrap(); result.restrictions[0].penalty = 0.0;
    assert!(source.validate_schedule(&result).is_err());
    let hard = self::source(vec![relation(None)]);
    let base = problem(CostCalculator);
    let resolver = Arc::new(UncheckedResolver { base: base.clone(), restrictions: hard.restrictions().unwrap().clone() });
    let mut context = BunchCompilationContext::with_resolver(base.configuration().clone(), resolver).unwrap();
    let mut model = MetaModel::new("unchecked-restriction-resolver"); context.register(&mut model).unwrap();
    context.add_candidates(0, vec![fleet::complete(&base)], &mut model).unwrap();
    model.retain_constraints(|row| row.name != "recovery_restriction_0");
    let (_, values) = optimum(&model);
    assert!(context.extract_solution(&model, &values).is_err());
}

#[derive(Debug)]
struct ActualDeparture;

impl GeneralRestrictionPredicate for ActualDeparture {
    fn matches(&self, assignment: &RestrictionAssignment<'_>) -> bool {
        assignment.departure.icao.0 == "A" && assignment.time.is_some_and(|time| time.0 == at(8, 0))
    }
}

#[test]
fn custom_restriction_conditions_participate_without_editing_application_solver() {
    let source = source(vec![general(GeneralRestrictionCondition::Custom(Arc::new(ActualDeparture)))]);
    let (mut context, mut model) = context(&source);
    context.add_candidates(0, vec![fleet::complete(&source)], &mut model).unwrap();
    let (value, values) = optimum(&model);
    assert_eq!(value, 500.0);
    assert_eq!(context.extract_solution(&model, &values).unwrap().restrictions[0].violated_tasks, ["F1"]);
}

#[test]
fn restriction_configuration_rejects_unknown_references_duplicates_invalid_windows_and_cost_overflow() {
    let mut bad = vec![relation(Some(-1.0)), relation(Some(f64::NAN)), relation(Some(f64::INFINITY))];
    for weight in [-1.0, f64::NAN, f64::INFINITY, f64::MAX] { bad.push(Restriction { weight, ..relation(Some(2.0)) }); }
    bad.extend([
        Restriction { id: String::new(), ..relation(None) },
        general(GeneralRestrictionCondition::Flights(set(&["unknown"]))),
        general(GeneralRestrictionCondition::EnabledAircraft(set(&["unknown"]))),
        general(GeneralRestrictionCondition::Departure { airports: set(&["unknown"]), time: None }),
        general(GeneralRestrictionCondition::Arrival { airports: set(&["B"]), time: Some((at(9, 0), at(8, 0))) }),
        Restriction { rule: RestrictionRule::Relation { category: RelationRestrictionCategory::BlackList,
            departure: "unknown".into(), arrival: "B".into(), aircraft: set(&["B1001"]) }, ..relation(None) },
    ]);
    for restriction in bad { assert!(fleet::fresh().with_restrictions(vec![restriction]).is_err()); }
    assert!(fleet::fresh().with_restrictions(vec![relation(None), relation(Some(0.0))]).is_err());
    let base = problem(CostCalculator);
    let mut domains = base.domains().to_vec(); domains[0].previous = Some(activity("PREV", "X", "A", 5, 6));
    assert!(RecoveryProblem::new(base.configuration().clone(), domains, vec![], CostCalculator).unwrap()
        .with_restrictions(vec![general(GeneralRestrictionCondition::Flights(set(&["PREV"])))]).is_ok());
}

#[cfg(feature = "demo4-gurobi-cg")]
#[test]
#[ignore = "requires native Gurobi runtime and license; run with --include-ignored"]
fn demo4_gurobi_restrictions_use_actual_seed_times_rmp_duals_and_nonbinary_final_counts() {
    use ospf_rust_framework::solver::gurobi_extension::GurobiColumnGenerationSolver;
    let base = transfer_problem(1);
    let late = general(GeneralRestrictionCondition::Departure { airports: set(&["B"]), time: Some((at(12, 0), at(14, 0))) });
    let source = Arc::new(RecoveryProblem::new(base.configuration().clone(), base.domains().to_vec(), vec![], CostCalculator).unwrap()
        .with_restrictions(vec![late]).unwrap());
    let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(source.clone(), RecoveryApplicationConfiguration {
        initial: vec![single(&source, 0, 0), single(&source, 1, 2)],
        column_generation: ColumnGenerationPolicy { max_iterations: 5, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(200.0));
    assert!(report.proof.is_none());
    let solution = report.solution.unwrap().value.unwrap();
    assert!(solution.candidate_ids.len() > 2);
    assert!(solution.pricing.iter().any(|round| !round.batch.columns.is_empty()));
    assert!(solution.schedule.restrictions[0].violated_tasks.is_empty());
    source.validate_schedule(&solution.schedule).unwrap();
    let source = self::source(vec![general(GeneralRestrictionCondition::All(vec![]))]);
    let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(source.clone(), RecoveryApplicationConfiguration {
        initial: vec![fleet::complete(&source)],
        column_generation: ColumnGenerationPolicy { max_iterations: 5, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(700.0));
    let solution = report.solution.unwrap().value.unwrap();
    assert_eq!(solution.schedule.restrictions[0].violated_tasks, ["F1", "F2"]);
    assert_eq!(solution.schedule.restrictions[0].penalty, 400.0);
    source.validate_schedule(&solution.schedule).unwrap();
}
