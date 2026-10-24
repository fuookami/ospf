//! 实际转移策略在整个受管列生命周期中的回归。 / Actual transition-policy regressions across the managed column lifecycle.

use crate::framework::demo4::domain::bunch_compilation::context::RecoveryBunchResolver;
use crate::framework::demo4::domain::bunch_generation::initial::{RecoveryInitialBunchGenerator, RecoveryInitialConfiguration};
use crate::framework::demo4::domain::rule::service::RecoveryTransitionPolicy;
use super::fleet::{complete, fresh};
use super::*;

struct Turnaround(Duration);

impl RecoveryTransitionPolicy for Turnaround {
    fn connection_time(
        &self,
        _: &Aircraft,
        _: &FlightTaskRecovery,
        _: &FlightTaskRecovery,
    ) -> Result<Duration, RecoveryError> { Ok(self.0) }

    fn allows(
        &self,
        _: &Aircraft,
        previous: Option<&FlightTaskRecovery>,
        activity: &FlightTaskRecovery,
    ) -> Result<bool, RecoveryError> {
        Ok(previous.is_some() || activity.original().plan.id == "F1")
    }
}

fn configured() -> (Arc<RecoveryProblem<CostCalculator>>, RecoveryBunchCandidate) {
    let source = fresh();
    let original = complete(&source);
    let mut domains = source.domains().to_vec();
    let early = &domains[0].candidates[1].recovery;
    let late = FlightTaskRecovery::new(early.original().clone(), early.aircraft().clone(), early.route().clone(), (at(12, 0), at(14, 0))).unwrap();
    domains[0].candidates.push(RecoveryActivityCandidate { id: "late-F2".into(), recovery: late });
    let problem = RecoveryProblem::new(source.configuration().clone(), domains, vec![], CostCalculator).unwrap()
        .with_transition_policy(Arc::new(Turnaround(Duration::hours(2))));
    (Arc::new(problem), original)
}

#[test]
fn actual_transition_policy_is_shared_by_initial_pricing_replay_and_production_validation() {
    let (problem, original) = configured();
    let initial = RecoveryInitialBunchGenerator.generate(&problem, &RecoveryInitialConfiguration { originals: vec![original.clone()] }, &PathSearchBudget::default()).unwrap();
    assert_eq!(initial.batch.columns.len(), 1);
    let candidate = &initial.batch.columns[0];
    assert_eq!(candidate.bunch.task_ids().collect::<Vec<_>>(), ["F1", "F2"]);
    assert_eq!(candidate.bunch.recovery("F2").unwrap().time().0, at(12, 0));
    assert!(problem.resolve_candidate(&problem.entry(&original).unwrap()).is_err());
    let priced = RecoveryPricingPolicy::new(problem.clone(), None).price(&request(), &PathSearchBudget::default()).unwrap();
    assert_eq!(priced.batch.no_improving_columns, None);
    for search in &priced.searches {
        assert!(search.batch.columns.iter().all(|candidate| candidate.bunch.activities().is_empty() || candidate.bunch.contains("F1")));
        assert!(search.batch.columns.iter().all(|candidate| candidate.bunch.recovery("F2").is_none_or(|activity| activity.time().0 == at(12, 0))));
    }
    let mut context = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut model = MetaModel::new("custom-recovery-transition");
    context.register(&mut model).unwrap();
    let tokens = model.as_basic().num_tokens();
    assert!(context.add_candidates(0, vec![candidate.clone(), original.clone()], &mut model).is_err());
    assert_eq!(model.as_basic().num_tokens(), tokens);
    assert!(context.master().compilation.bunch_x_map.is_empty());
    context.add_candidates(0, initial.batch.columns, &mut model).unwrap();
    let (objective, values) = master_optimum(&model).unwrap();
    assert_eq!(objective, 300.0);
    let mut schedule = context.extract_solution(&model, &values).unwrap();
    schedule.selected = vec![original];
    assert!(problem.validate_schedule(&schedule).is_err());
}

#[test]
fn negative_custom_turnaround_is_an_error_without_mutating_registered_model() {
    let source = fresh();
    let candidate = complete(&source);
    let problem = Arc::new(source.with_transition_policy(Arc::new(Turnaround(-Duration::SECOND))));
    assert!(RecoveryInitialBunchGenerator.generate(&problem, &RecoveryInitialConfiguration::default(), &PathSearchBudget::default()).is_err());
    let mut context = BunchCompilationContext::with_resolver(problem.configuration().clone(), problem.clone()).unwrap();
    let mut model = MetaModel::new("invalid-custom-turnaround");
    context.register(&mut model).unwrap();
    let before = model.try_to_linear_triad_model().unwrap();
    assert!(context.add_candidates(0, vec![candidate], &mut model).is_err());
    let after = model.try_to_linear_triad_model().unwrap();
    assert_eq!(before.c, after.c);
    assert_eq!(before.b, after.b);
    assert_eq!(before.num_variables(), after.num_variables());
}

#[test]
fn custom_transition_reads_actual_history_and_cannot_bypass_link_minima() {
    struct ActualHistory;
    impl RecoveryTransitionPolicy for ActualHistory {
        fn allows(
            &self,
            _: &Aircraft,
            previous: Option<&FlightTaskRecovery>,
            activity: &FlightTaskRecovery,
        ) -> Result<bool, RecoveryError> {
            Ok(previous.is_some_and(|previous| previous.route().arr == activity.route().dep && previous.time().1 <= activity.time().0))
        }
    }
    let previous = activity("executed", "X", "A", 5, 6);
    let bunch = bunch();
    let usability = usability();
    let mut rules = BunchRecoveryRules { usability: &usability, previous: Some(&previous), links: &[] };
    assert!(FeasibilityJudger.judge_bunch_with_policy(&bunch, &rules, &ActualHistory).unwrap().feasible);
    rules.previous = None;
    assert!(!FeasibilityJudger.judge_bunch_with_policy(&bunch, &rules, &ActualHistory).unwrap().feasible);
    let links = [Link { from_task: "F1".into(), to_task: "F2".into(), link_type: LinkType::ConnectionTimeIgnoring, split_cost: None }];
    rules.links = &links;
    assert!(FeasibilityJudger.judge_bunch_with_policy(&bunch, &rules, &Turnaround(Duration::hours(2))).unwrap().feasible);
    let links = [Link { link_type: LinkType::Connecting { min_connection_time: Duration::hours(2) }, ..links[0].clone() }];
    rules.links = &links;
    assert!(!FeasibilityJudger.judge_bunch_with_policy(&bunch, &rules, &Turnaround(Duration::ZERO)).unwrap().feasible);
}

#[cfg(feature = "demo4-gurobi-cg")]
#[test]
#[ignore = "requires native Gurobi runtime and license; run with --include-ignored"]
fn demo4_gurobi_custom_turnaround_uses_the_late_actual_variant_through_the_real_gantt_loop() {
    use ospf_rust_framework::solver::gurobi_extension::GurobiColumnGenerationSolver;
    let (problem, original) = configured();
    let report = RecoveryApplicationService::new(GurobiColumnGenerationSolver::new()).solve(problem, RecoveryApplicationConfiguration {
        initial_generation: Some(RecoveryInitialConfiguration { originals: vec![original] }),
        column_generation: ColumnGenerationPolicy { max_iterations: 3, ..Default::default() }, ..Default::default()
    }).unwrap();
    assert_eq!(report.solution.as_ref().unwrap().objective_value, Some(300.0));
    assert!(report.proof.is_none());
    let solution = report.solution.unwrap().value.unwrap();
    assert!(!solution.pricing.is_empty());
    assert!(solution.schedule.canceled.is_empty());
    let bunch = &solution.schedule.selected[0].bunch;
    assert_eq!(bunch.recovery("F2").unwrap().time().0, at(12, 0));
    assert_eq!(bunch.flight_hour(), FlightHour::new(Duration::hours(3)));
    assert!(bunch.arrived_when(&airport("B"), &(at(9, 0), at(9, 0))));
}
