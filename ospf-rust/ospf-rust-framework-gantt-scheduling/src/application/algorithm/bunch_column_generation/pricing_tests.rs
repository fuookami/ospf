use super::*;
use std::sync::Mutex;
use ospf_rust_core::error::Result as CoreResult;
use ospf_rust_core::model::{ConstraintRelation, ObjectiveCategory};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{ProblemStatus, SolveFingerprints, SolveProof, SolveSolution, TerminationReason};
use ospf_rust_core::variable::{Continuous, VariableRange};
use ospf_rust_framework::model::ColumnGenerationCompletion;
use crate::domain::bunch_compilation::context::BasicBunchCompilationContext;

struct Solver(SolveReport<f64>);

#[cfg_attr(feature = "async", async_trait::async_trait)]
impl ColumnGenerationSolver for Solver {
    fn name(&self) -> &str { "bunch-certificate-fixture" }

    #[cfg(not(feature = "async"))]
    fn solve_milp_report_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<SolveReport<f64>> {
        Ok(self.0.clone())
    }

    #[cfg(feature = "async")]
    async fn solve_milp_report_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<SolveReport<f64>> {
        Ok(self.0.clone())
    }

    #[cfg(not(feature = "async"))]
    fn solve_milp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<FeasibleSolution> {
        FeasibleSolution::try_from_report(&self.0)
    }

    #[cfg(feature = "async")]
    async fn solve_milp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<FeasibleSolution> {
        FeasibleSolution::try_from_report(&self.0)
    }

    #[cfg(not(feature = "async"))]
    fn solve_lp_with_options(&self, model: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<LPResult> {
        assert!(!model.has_integer_variables());
        LPResult::try_from_solve_report(&self.0)
    }

    #[cfg(feature = "async")]
    async fn solve_lp_with_options(&self, model: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<LPResult> {
        assert!(!model.has_integer_variables());
        LPResult::try_from_solve_report(&self.0)
    }
}

struct Policy<F>(F);

impl<F> BunchCGPolicy for Policy<F>
where
    F: Fn(&BunchPricingRequest) -> ColumnGenerationBatch<BunchEntry> + Send + Sync,
{
    fn build_shadow_price_map(&self) -> HashMap<usize, f64> { HashMap::new() }
    fn reduced_cost(&self, _: &HashMap<usize, f64>, bunch: &BunchEntry) -> f64 { bunch.cost }
    fn generate_bunches(&self, _: usize, _: &[ExecutorId], _: &HashMap<usize, f64>) -> Vec<BunchEntry> { Vec::new() }
    fn generate_bunch_batch(&self, request: &BunchPricingRequest) -> GanttResult<ColumnGenerationBatch<BunchEntry>> {
        Ok((self.0)(request))
    }
}

fn complete(request: &BunchPricingRequest) -> ColumnGenerationBatch<BunchEntry> {
    ColumnGenerationBatch { columns: Vec::new(), completion: ColumnGenerationCompletion::Exhausted,
        scope: Some(request.scope.clone()), no_improving_columns: Some(true), children: Vec::new() }
}

#[test]
fn production_milp_entry_preserves_normal_no_incumbent_reports_and_rejects_technical_failure() {
    for reason in [TerminationReason::Completed, TerminationReason::TimeLimit, TerminationReason::Cancelled, TerminationReason::Interrupted,
        TerminationReason::BackendFailure, TerminationReason::NumericalFailure]
    {
        let (mut model, mut algorithm) = fixture(complete);
        let status = if reason == TerminationReason::Completed { ProblemStatus::Infeasible } else { ProblemStatus::Unknown };
        algorithm.solver.0 = SolveReport::builder(status, reason).build().unwrap();
        let result = algorithm.solve_milp_report(&mut model);
        if matches!(reason, TerminationReason::BackendFailure | TerminationReason::NumericalFailure) {
            assert!(result.is_err());
        } else {
            let report = result.unwrap();
            assert_eq!(report.termination_reason, reason);
            assert_eq!(report.problem_status, status);
            assert!(!report.has_incumbent());
        }
    }
}

#[test]
fn report_loop_preserves_initial_normal_termination_and_does_not_replay_a_stale_report() {
    let (mut model, mut algorithm) = fixture(complete);
    for reason in [TerminationReason::Completed, TerminationReason::TimeLimit, TerminationReason::Cancelled, TerminationReason::Interrupted] {
        let status = if reason == TerminationReason::Completed { ProblemStatus::Infeasible } else { ProblemStatus::Unknown };
        algorithm.solver.0 = SolveReport::builder(status, reason).build().unwrap();
        let report = algorithm.run_report(&mut model).unwrap();
        assert_eq!(report.problem_status, status);
        assert_eq!(report.termination_reason, reason);
        assert!(!report.has_incumbent());
    }
    for reason in [TerminationReason::BackendFailure, TerminationReason::NumericalFailure] {
        algorithm.solver.0 = SolveReport::builder(ProblemStatus::Unknown, reason).build().unwrap();
        assert!(algorithm.run_report(&mut model).is_err());
        assert!(algorithm.incumbent_report().is_none());
    }
}

#[test]
fn production_milp_entry_rejects_invalid_incumbents_and_downstream_rule_violations() {
    for (values, objective) in [(vec![], -2.0), (vec![0.0, 2.0, 0.0], -2.0), (vec![0.0, 3.0], -3.0),
        (vec![0.5, 2.0], -2.0), (vec![0.0, 2.0], 99.0)]
    {
        let (mut model, mut algorithm) = fixture(complete);
        algorithm.solver.0 = FeasibleSolution::new(objective, values).to_solve_report("invalid-fixture").unwrap();
        assert!(algorithm.solve_milp_report(&mut model).is_err());
    }
    let (mut model, mut algorithm) = fixture(complete);
    assert!(algorithm.solve_milp_report(&mut model).unwrap().has_incumbent());
    model.add_ge_constraint(&[(1, 1.0)], 3.0, "downstream_rule").unwrap();
    assert!(algorithm.solve_milp_report(&mut model).is_err());
}

#[test]
fn column_indices_follow_existing_sparse_history_not_generator_local_indices() {
    let (mut model, mut algorithm) = fixture(complete);
    let initial = BunchEntry { id: "initial".into(), index: 40, executor_id: "e".into(), task_indices: vec![0], cost: 1.0, iteration: 0, slot_index: None };
    algorithm.context.add_columns(0, vec![initial.clone()], &mut model).unwrap();
    let next = BunchEntry { id: "next".into(), index: usize::MAX, cost: 2.0, ..initial.clone() };
    assert_eq!(algorithm.add_columns(1, vec![next.clone()], &mut model).unwrap(), vec![41]);
    assert_eq!(algorithm.bunch_index_counter, 42);
    let tokens = model.as_basic().num_tokens();
    assert!(algorithm.add_columns(2, vec![next], &mut model).unwrap().is_empty());
    assert_eq!(algorithm.bunch_index_counter, 42);
    assert_eq!(model.as_basic().num_tokens(), tokens);
    assert!(algorithm.add_columns(2, vec![BunchEntry { id: "bad".into(), cost: f64::NAN, ..initial }], &mut model).is_err());
    assert_eq!(algorithm.bunch_index_counter, 42);
    assert_eq!(model.as_basic().num_tokens(), tokens);
}

fn fixture<F>(policy: F) -> (MetaModel<f64>, BunchBranchAndPriceAlgorithm<BasicBunchCompilationContext, Solver, Policy<F>>)
where
    F: Fn(&BunchPricingRequest) -> ColumnGenerationBatch<BunchEntry> + Send + Sync,
{
    let mut model = MetaModel::new("bunch-pricing-certificate");
    let mut context = BasicBunchCompilationContext::new(1, vec!["e"], false);
    context.register(&mut model).unwrap();
    let x = model.register_auto_variable_with_range::<Continuous>("x", VariableRange::bounded(0.0, 10.0)).unwrap();
    model.add_linear_constraint(&[(x, 1.0)], ConstraintRelation::Equal, 2.0, "task_compilation_0").unwrap();
    model.add_linear_constraint(&[(x, 1.0)], ConstraintRelation::GreaterEqual, 0.0, "extension").unwrap();
    model.set_linear_objective(vec![0.0, -1.0], ObjectiveCategory::Minimum);
    let mut current = model.try_to_linear_triad_model().unwrap();
    current.linear_relax();
    let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
        .solution(SolveSolution { value: None, values: vec![0.0, 2.0], stable_values: Default::default(),
            objective: Some(-2.0), objective_value: Some(-2.0), dual_solution: Some(vec![-1.0, 0.0, 0.0]),
            quadratic_dual_solution: None, pool: Vec::new() })
        .proof(SolveProof::optimality())
        .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(&current).unwrap()), ..Default::default() })
        .build().unwrap();
    (model, BunchBranchAndPriceAlgorithm::new(context, Solver(report), Policy(policy), vec!["e"], ColumnGenerationPolicy::default()))
}

#[test]
fn bunch_requests_rederive_signed_prices_from_source_and_include_extension_zeros() {
    let (mut model, mut algorithm) = fixture(|request| {
        assert_eq!(request.shadow_prices[&0], -1.0);
        assert_eq!(request.row_duals["extension"], 0.0);
        assert_eq!(request.row_duals.len(), 2);
        assert!(request.verified_duals.is_some());
        assert!(request.scope.global);
        complete(request)
    });
    algorithm.solve_rmp_lp(&mut model).unwrap();
    algorithm.shadow_prices.insert(0, 999.0);
    algorithm.generate_bunches(&[ExecutorId::from("e")], &model).unwrap();
    assert!(algorithm.last_pricing_complete);
    assert_eq!(algorithm.last_certified_bound, Some(-2.0));
}

#[test]
fn forged_optimal_bunch_report_is_an_error_not_a_heuristic_downgrade() {
    let (mut model, mut algorithm) = fixture(complete);
    algorithm.solver.0.solution.as_mut().unwrap().dual_solution = Some(vec![0.0; 3]);
    assert!(algorithm.solve_rmp_lp(&mut model).is_err());
    assert!(algorithm.pricing_certificate.is_none());
    assert!(!algorithm.last_pricing_complete);
}

#[test]
fn bunch_certificates_cannot_replay_after_model_or_context_changes() {
    for change in 0..3 {
        let (mut model, mut algorithm) = fixture(complete);
        algorithm.solve_rmp_lp(&mut model).unwrap();
        match change {
            0 => model.set_linear_objective(vec![0.0, -2.0], ObjectiveCategory::Minimum),
            1 => algorithm.context.compilation.cost_terms.push((1, 4.0)),
            _ => { model = fixture(complete).0; },
        }
        assert!(algorithm.generate_bunches(&[ExecutorId::from("e")], &model).is_err());
        assert!(!algorithm.last_pricing_complete);
    }
}

#[test]
fn narrowed_executor_domain_and_budget_stops_do_not_certify_bunches() {
    let (mut model, mut algorithm) = fixture(|request| {
        assert!(!request.scope.global);
        complete(request)
    });
    algorithm.solve_rmp_lp(&mut model).unwrap();
    algorithm.executor_ids.clear();
    algorithm.generate_bunches(&[], &model).unwrap();
    assert!(!algorithm.last_pricing_complete);
    for completion in [ColumnGenerationCompletion::BudgetLimit, ColumnGenerationCompletion::Interrupted, ColumnGenerationCompletion::Heuristic] {
        let (mut model, mut algorithm) = fixture(|request| {
            let mut batch = complete(request);
            batch.completion = completion;
            batch
        });
        algorithm.solve_rmp_lp(&mut model).unwrap();
        algorithm.generate_bunches(&[ExecutorId::from("e")], &model).unwrap();
        assert!(!algorithm.last_pricing_complete);
        assert_eq!(algorithm.last_certified_bound, None);
    }
}

#[test]
fn bunch_scope_replay_and_malformed_candidates_are_rejected() {
    let previous = Mutex::new(None);
    let (mut model, mut algorithm) = fixture(|request| {
        let mut previous = previous.lock().unwrap();
        let mut batch = complete(request);
        if let Some(scope) = previous.clone() { batch.scope = Some(scope); }
        else { *previous = Some(request.scope.clone()); }
        batch
    });
    algorithm.solve_rmp_lp(&mut model).unwrap();
    algorithm.generate_bunches(&[ExecutorId::from("e")], &model).unwrap();
    assert!(algorithm.generate_bunches(&[ExecutorId::from("e")], &model).is_err());
    assert!(!algorithm.last_pricing_complete);
    assert_eq!(algorithm.last_certified_bound, None);
    assert_eq!(algorithm.iteration.lower_bound, None);
    assert_eq!(algorithm.iteration.best_dual_obj, None);
    for invalid in 0..4 {
        let (mut model, mut algorithm) = fixture(|request| {
            let mut batch = complete(request);
            batch.columns.push(BunchEntry { id: "bunch-0".into(), index: 0, iteration: 0, executor_id: ExecutorId::from("e"),
                slot_index: None, task_indices: vec![0], cost: 0.0 });
            match invalid {
                0 => batch.columns[0].cost = f64::NAN,
                1 => batch.columns[0].task_indices = vec![99],
                2 => batch.columns[0].executor_id = ExecutorId::from("unknown"),
                _ => batch.columns[0].slot_index = Some(99),
            }
            batch
        });
        algorithm.solve_rmp_lp(&mut model).unwrap();
        assert!(algorithm.generate_bunches(&[ExecutorId::from("e")], &model).is_err());
        assert!(!algorithm.last_pricing_complete);
    }
}

#[cfg(feature = "scip")]
#[test]
#[ignore = "需要真实 SCIP 运行时 / requires the actual SCIP runtime"]
fn native_scip_bunch_lp_supplies_signed_complete_pricing_evidence() {
    let (mut model, previous) = fixture(complete);
    let solver = ospf_rust_framework::ScipColumnGenerationSolver::new().with_lp_subproblem_defaults();
    let mut algorithm = BunchBranchAndPriceAlgorithm::new(previous.context, solver, previous.policy,
        vec![ExecutorId::from("e")], ColumnGenerationPolicy::default());
    assert!(algorithm.solve_rmp_lp(&mut model).unwrap().is_some());
    algorithm.generate_bunches(&[ExecutorId::from("e")], &model).unwrap();
    assert!(algorithm.last_pricing_complete);
    assert!((algorithm.last_certified_bound.unwrap() + 2.0).abs() < 1e-8);
}
