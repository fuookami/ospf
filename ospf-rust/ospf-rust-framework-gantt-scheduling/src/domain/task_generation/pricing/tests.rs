use super::*;
use std::sync::Mutex;
use ospf_rust_core::error::Result as CoreResult;
use ospf_rust_core::model::{ConstraintRelation, ObjectiveCategory};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{ProblemStatus, SolveFingerprints, SolveProof, SolveSolution, TerminationReason};
use ospf_rust_core::variable::{Binary, Continuous, VariableRange};
use ospf_rust_framework::solver::column_generation_solver::{ColumnGenerationSolver, FeasibleSolution};
use ospf_rust_framework::solver::framework_solve_options::FrameworkSolveOptions;
use crate::application::algorithm::policy::ColumnGenerationPolicy;
use crate::application::algorithm::task_column_generation::TaskColumnGenerationAlgorithm;
use crate::domain::task::CostItem;
use crate::domain::task_compilation::context::BasicTaskCompilationContext;

struct Solver(SolveReport<f64>);

#[cfg_attr(feature = "async", async_trait::async_trait)]
impl ColumnGenerationSolver for Solver {
    fn name(&self) -> &str { "task-pricing-fixture" }

    #[cfg(not(feature = "async"))]
    fn solve_milp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<FeasibleSolution> {
        FeasibleSolution::try_from_report(&self.0)
    }

    #[cfg(feature = "async")]
    async fn solve_milp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<FeasibleSolution> {
        FeasibleSolution::try_from_report(&self.0)
    }

    #[cfg(not(feature = "async"))]
    fn solve_lp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<LPResult> {
        LPResult::try_from_solve_report(&self.0)
    }

    #[cfg(feature = "async")]
    async fn solve_lp_with_options(&self, _: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<LPResult> {
        LPResult::try_from_solve_report(&self.0)
    }

    #[cfg(not(feature = "async"))]
    fn solve_lp_report_with_options(&self, model: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<SolveReport<f64>> {
        assert!(!model.has_integer_variables());
        Ok(self.0.clone())
    }

    #[cfg(feature = "async")]
    async fn solve_lp_report_with_options(&self, model: &LinearTriadModel, _: FrameworkSolveOptions) -> CoreResult<SolveReport<f64>> {
        assert!(!model.has_integer_variables());
        Ok(self.0.clone())
    }
}

type Algorithm = TaskColumnGenerationAlgorithm<BasicTaskCompilationContext, Solver>;

fn report(model: &MetaModel<f64>, values: Vec<f64>, duals: Vec<f64>, objective: f64) -> SolveReport<f64> {
    let mut current = model.try_to_linear_triad_model().unwrap();
    current.linear_relax();
    SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
        .solution(SolveSolution { value: None, values, stable_values: Default::default(),
            objective: Some(objective), objective_value: Some(objective), dual_solution: Some(duals),
            quadratic_dual_solution: None, pool: Vec::new() })
        .proof(SolveProof::optimality())
        .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(&current).unwrap()), ..Default::default() })
        .build().unwrap()
}

fn fixture() -> (MetaModel<f64>, Algorithm) {
    let mut model = MetaModel::new("task-pricing");
    let mut context = BasicTaskCompilationContext::new(2, 2, false, false);
    context.register(&mut model).unwrap();
    let x = model.register_auto_variable_with_range::<Continuous>("x", VariableRange::bounded(0.0, 10.0)).unwrap();
    model.register_auto_variable::<Binary>("extra-binary").unwrap();
    model.add_linear_constraint(&[(x, 1.0)], ConstraintRelation::LessEqual, 2.0, "task_compilation_0").unwrap();
    model.add_linear_constraint(&[(x, 1.0)], ConstraintRelation::LessEqual, 3.0, "task_compilation_1").unwrap();
    model.add_linear_constraint(&[(x, 1.0)], ConstraintRelation::GreaterEqual, 0.0, "extension").unwrap();
    model.set_linear_objective(vec![-1.0, 0.0], ObjectiveCategory::Minimum);
    let solver = Solver(report(&model, vec![2.0, 0.0], vec![-1.0, 0.0, 0.0], -2.0));
    (model, Algorithm::new(context, solver, ColumnGenerationPolicy::default()))
}

fn exhausted(scope: ColumnGenerationScope) -> ColumnGenerationBatch<TaskPricingColumn> {
    ColumnGenerationBatch { columns: Vec::new(), completion: ColumnGenerationCompletion::Exhausted,
        scope: Some(scope), no_improving_columns: Some(true), children: Vec::new() }
}

struct Policy<F>(F);

impl<F> TaskCGPolicy for Policy<F>
where
    F: Fn(&TaskPricingRequest) -> ColumnGenerationBatch<TaskPricingColumn> + Send + Sync,
{
    fn generate_tasks(&self, _: &TaskPricingRequest) -> GanttResult<Vec<TaskPricingColumn>> { Ok(Vec::new()) }

    fn generate_task_batch(&self, request: &TaskPricingRequest) -> GanttResult<ColumnGenerationBatch<TaskPricingColumn>> {
        Ok((self.0)(request))
    }
}

fn complete(request: &TaskPricingRequest) -> ColumnGenerationBatch<TaskPricingColumn> {
    exhausted(request.scope.clone())
}

fn generate(algorithm: &mut Algorithm, model: &MetaModel<f64>) -> GanttResult<ColumnGenerationBatch<TaskPricingColumn>> {
    algorithm.generate_task_batch(model, TaskPricingDomain::unslotted(2, 2), &Policy(complete))
}

#[test]
fn verified_current_lp_and_complete_no_improvement_produce_only_an_lp_bound() {
    let (model, mut algorithm) = fixture();
    algorithm.solve_rmp_lp(&model).unwrap();
    algorithm.shadow_prices.insert(0, 999.0);
    let policy = Policy(|request: &TaskPricingRequest| {
        assert_eq!(request.shadow_prices[&0], -1.0);
        assert_eq!(request.shadow_prices[&1], 0.0);
        assert_eq!(request.row_duals.len(), 3);
        assert_eq!(request.row_duals["extension"], 0.0);
        assert!(request.verified_duals.is_some());
        assert!(request.scope.global);
        complete(request)
    });
    algorithm.generate_task_batch(&model, TaskPricingDomain::unslotted(2, 2), &policy).unwrap();
    assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), Some(-2.0));
    assert_eq!(algorithm.pricing_completion(), Some(ColumnGenerationCompletion::Exhausted));
}

#[test]
fn legacy_empty_generation_and_bare_duals_remain_heuristic() {
    struct Legacy;
    impl TaskCGPolicy for Legacy {
        fn generate_tasks(&self, request: &TaskPricingRequest) -> GanttResult<Vec<TaskPricingColumn>> {
            assert!(request.verified_duals.is_none());
            assert!(!request.scope.global);
            Ok(Vec::new())
        }
    }
    let (model, mut algorithm) = fixture();
    let result = algorithm.solve_rmp_lp(&model).unwrap();
    algorithm.extract_shadow_prices(&result, &[("task_compilation_0".into(), 0)].into());
    let batch = algorithm.generate_task_batch(&model, TaskPricingDomain::unslotted(2, 2), &Legacy).unwrap();
    assert_eq!(batch.completion, ColumnGenerationCompletion::Heuristic);
    assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    let mut raw = result;
    raw.report = None;
    raw.result.source_report = None;
    assert!(algorithm.accept_pricing_lp(&model, &raw).is_err());
}

#[test]
fn budgets_interruptions_and_missing_no_improvement_do_not_certify() {
    for completion in [ColumnGenerationCompletion::BudgetLimit, ColumnGenerationCompletion::Interrupted, ColumnGenerationCompletion::Heuristic] {
        let (model, mut algorithm) = fixture();
        algorithm.solve_rmp_lp(&model).unwrap();
        let policy = Policy(|request: &TaskPricingRequest| {
            let mut batch = complete(request);
            batch.completion = completion;
            batch
        });
        algorithm.generate_task_batch(&model, TaskPricingDomain::unslotted(2, 2), &policy).unwrap();
        assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    }
    let (model, mut algorithm) = fixture();
    algorithm.solve_rmp_lp(&model).unwrap();
    let policy = Policy(|request: &TaskPricingRequest| {
        let mut batch = complete(request);
        batch.no_improving_columns = None;
        batch
    });
    algorithm.generate_task_batch(&model, TaskPricingDomain::unslotted(2, 2), &policy).unwrap();
    assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
}

#[test]
fn restricted_domains_fixing_and_retained_columns_do_not_certify_global_bounds() {
    let (model, mut algorithm) = fixture();
    algorithm.solve_rmp_lp(&model).unwrap();
    algorithm.generate_task_batch(&model, TaskPricingDomain::unslotted(1, 2), &Policy(complete)).unwrap();
    assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    for fixed in [true, false] {
        if fixed { algorithm.fixed_columns.insert(0); } else { algorithm.fixed_columns.clear(); algorithm.kept_columns.insert(0); }
        generate(&mut algorithm, &model).unwrap();
        assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    }
}

#[test]
fn lp_projection_tampering_is_not_hidden_by_compatibility_conversion() {
    for mutation in 0..5 {
        let (model, mut algorithm) = fixture();
        let mut result = algorithm.solve_rmp_lp(&model).unwrap();
        match mutation {
            0 => result.dual_solution.constraints[0] = 0.0,
            1 => result.result.solution[0] = 1.0,
            2 => result.result.obj = -1.0,
            3 => result.dual_solution.constraints[0] = f64::NAN,
            _ => result.result.source_report.as_mut().unwrap().solution.as_mut().unwrap().dual_solution = Some(vec![0.0; 3]),
        }
        assert!(algorithm.accept_pricing_lp(&model, &result).is_err());
        assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    }
}

#[test]
fn changed_objective_bounds_rows_and_columns_reject_stale_certificates() {
    for mutation in 0..4 {
        let (mut model, mut algorithm) = fixture();
        algorithm.solve_rmp_lp(&model).unwrap();
        generate(&mut algorithm, &model).unwrap();
        match mutation {
            0 => model.set_linear_objective(vec![-2.0, 0.0], ObjectiveCategory::Minimum),
            1 => model.set_variable_range_by_index(0, VariableRange::bounded(0.0, 1.0)).unwrap(),
            2 => { model.add_linear_constraint(&[(0, 1.0)], ConstraintRelation::LessEqual, 1.0, "downstream").unwrap(); },
            _ => { model.register_auto_variable::<Continuous>("downstream").unwrap(); },
        }
        assert!(algorithm.certified_lower_bound(&model).is_err());
        assert!(generate(&mut algorithm, &model).is_err());
    }
}

#[test]
fn identical_compiled_models_cannot_replay_certificates_across_meta_identities() {
    let (model, mut algorithm) = fixture();
    let (other, _) = fixture();
    algorithm.solve_rmp_lp(&model).unwrap();
    assert!(generate(&mut algorithm, &other).is_err());
}

#[test]
fn task_context_changes_invalidate_provenance_even_without_model_mutation() {
    let (mut model, mut algorithm) = fixture();
    algorithm.add_initial_columns(vec![(0, 0, Cost::empty())], &mut model).unwrap();
    let values = vec![2.0, 0.0, 0.0];
    algorithm.solver.0 = report(&model, values, vec![-1.0, 0.0, 0.0], -2.0);
    algorithm.solve_rmp_lp(&model).unwrap();
    generate(&mut algorithm, &model).unwrap();
    algorithm.context.remove_columns(&[0]);
    assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    assert!(generate(&mut algorithm, &model).is_err());
}

#[test]
fn child_batches_must_cover_task_executor_pairs_not_only_each_axis() {
    for full in [false, true] {
        let (model, mut algorithm) = fixture();
        algorithm.solve_rmp_lp(&model).unwrap();
        let policy = Policy(|request: &TaskPricingRequest| {
            let mut parent = complete(request);
            for task in 0..2 {
                let mut scope = request.scope.clone();
                scope.request = format!("{}:{task}", scope.request);
                scope.groups = [format!("task:{task}")].into();
                if !full { scope.executor_slots = [format!("{task}:0")].into(); }
                parent.children.push(exhausted(scope));
            }
            parent
        });
        algorithm.generate_task_batch(&model, TaskPricingDomain::unslotted(2, 2), &policy).unwrap();
        assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), if full { Some(-2.0) } else { None });
    }
}

#[test]
fn wrong_scope_and_previous_request_replays_are_rejected() {
    let previous = Mutex::new(None);
    let (model, mut algorithm) = fixture();
    algorithm.solve_rmp_lp(&model).unwrap();
    let policy = Policy(|request: &TaskPricingRequest| {
        let mut old = previous.lock().unwrap();
        if let Some(scope) = old.clone() { exhausted(scope) }
        else { *old = Some(request.scope.clone()); complete(request) }
    });
    algorithm.generate_task_batch(&model, TaskPricingDomain::unslotted(2, 2), &policy).unwrap();
    assert!(algorithm.generate_task_batch(&model, TaskPricingDomain::unslotted(2, 2), &policy).is_err());
    assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    for mismatch in 0..4 {
        let policy = Policy(|request: &TaskPricingRequest| {
            let mut batch = complete(request);
            let scope = batch.scope.as_mut().unwrap();
            match mismatch {
                0 => scope.model.push_str("-stale"),
                1 => scope.dual.push_str("-stale"),
                2 => scope.branch.push_str("-stale"),
                _ => scope.executor_slots.clear(),
            }
            batch
        });
        assert!(algorithm.generate_task_batch(&model, TaskPricingDomain::unslotted(2, 2), &policy).is_err());
    }
}

#[test]
fn malformed_candidates_are_rejected_before_model_or_context_mutation() {
    let (mut model, mut algorithm) = fixture();
    let count = model.try_to_linear_triad_model().unwrap().num_variables();
    for mutation in 0..6 {
        let mut column = TaskPricingColumn { task_index: 0, executor_index: 0, cost: Cost::empty() };
        match mutation {
            0 => column.task_index = 2,
            1 => column.executor_index = 2,
            2 => column.cost.cost_sum = Some(f64::INFINITY),
            3 => column.cost.items.push(CostItem::with_quantity("invalid", f64::NAN)),
            4 => column.cost.cost_sum = Some(f64::NEG_INFINITY),
            _ => column.cost.items.push(CostItem::new("missing", None, None)),
        }
        let policy = Policy(|_: &TaskPricingRequest| ColumnGenerationBatch::heuristic(vec![column.clone()]));
        assert!(algorithm.generate_task_batch(&model, TaskPricingDomain::unslotted(2, 2), &policy).is_err());
        assert!(algorithm.add_priced_columns(vec![column], &mut model).is_err());
        assert_eq!(algorithm.context.column_count(), 0);
        assert_eq!(model.try_to_linear_triad_model().unwrap().num_variables(), count);
    }
    assert!(algorithm.add_initial_columns(vec![(0, 0, Cost::empty()), (9, 0, Cost::empty())], &mut model).is_err());
    assert_eq!(algorithm.context.column_count(), 0);
    assert_eq!(model.try_to_linear_triad_model().unwrap().num_variables(), count);
}

#[test]
fn signed_greater_equal_and_equality_duals_are_folded_once() {
    for (relation, cost, duals, expected) in [
        (ConstraintRelation::GreaterEqual, 1.0, vec![-1.0], 1.0),
        (ConstraintRelation::Equal, -1.0, vec![-1.0, 0.0], -1.0),
        (ConstraintRelation::Equal, 1.0, vec![0.0, -1.0], 1.0),
    ] {
        let mut model = MetaModel::new("signed-task-rows");
        let x = model.register_auto_variable_with_range::<Continuous>("x", VariableRange::bounded(0.0, 10.0)).unwrap();
        model.add_linear_constraint(&[(x, 1.0)], relation, 2.0, "task").unwrap();
        model.set_linear_objective(vec![cost], ObjectiveCategory::Minimum);
        let evidence = PricingEvidence::new(&model, &report(&model, vec![2.0], duals, 2.0 * cost)).unwrap();
        assert_eq!(evidence.row_duals["task"], expected);
    }
}

#[test]
fn duplicate_row_names_and_missing_duals_cannot_create_pricing_evidence() {
    let (mut model, algorithm) = fixture();
    let mut source = algorithm.solver.0.clone();
    source.solution.as_mut().unwrap().dual_solution = None;
    assert!(PricingEvidence::new(&model, &source).is_err());
    assert!(model.add_linear_constraint(&[(0, 1.0)], ConstraintRelation::LessEqual, 9.0, "extension").is_err());
    assert!(PricingEvidence::new(&model, &algorithm.solver.0).is_ok());
}

#[test]
fn nonoptimal_terminal_reports_remain_structured_without_pricing_proof() {
    for (status, termination) in [
        (ProblemStatus::Infeasible, TerminationReason::Completed),
        (ProblemStatus::Unknown, TerminationReason::TimeLimit),
        (ProblemStatus::Unknown, TerminationReason::Cancelled),
        (ProblemStatus::Unknown, TerminationReason::BackendFailure),
    ] {
        let (model, mut algorithm) = fixture();
        algorithm.solver.0 = SolveReport::builder(status, termination).build().unwrap();
        let output = algorithm.solve_rmp_lp_report(&model).unwrap();
        assert_eq!(output.termination_reason, termination);
        assert!(!output.has_incumbent());
        generate(&mut algorithm, &model).unwrap();
        assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    }
}

#[test]
fn fixing_and_flush_clear_previously_certified_bounds() {
    let (model, mut algorithm) = fixture();
    algorithm.solve_rmp_lp(&model).unwrap();
    generate(&mut algorithm, &model).unwrap();
    algorithm.kept_columns.insert(0);
    assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    algorithm.kept_columns.clear();
    algorithm.globally_fix(&[0].into());
    assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    algorithm.flush();
    assert_eq!(algorithm.pricing_completion(), None);
    assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
}

#[test]
fn optimal_task_reports_with_nonfinite_or_forged_values_are_rejected() {
    for mutation in 0..5 {
        let (model, mut algorithm) = fixture();
        let solution = algorithm.solver.0.solution.as_mut().unwrap();
        match mutation {
            0 => solution.values[0] = f64::NAN,
            1 => solution.dual_solution.as_mut().unwrap()[0] = f64::INFINITY,
            2 => solution.objective_value = Some(f64::NEG_INFINITY),
            3 => solution.values[0] = 11.0,
            _ => solution.dual_solution = Some(vec![0.0; 3]),
        }
        assert!(algorithm.solve_rmp_lp_report(&model).is_err());
        assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    }
}

#[test]
fn task_add_remove_readd_uses_new_variables_and_preserves_downstream_model_terms() {
    let (mut model, mut algorithm) = fixture();
    algorithm.solve_rmp_lp(&model).unwrap();
    generate(&mut algorithm, &model).unwrap();
    let column = TaskPricingColumn { task_index: 0, executor_index: 0, cost: Cost::empty() };
    let first = algorithm.add_priced_columns(vec![column.clone()], &mut model).unwrap().remove(0);
    assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
    algorithm.context.remove_columns(&[first.index]);
    let second = algorithm.add_priced_columns(vec![column], &mut model).unwrap().remove(0);
    assert_ne!(first.index, second.index);
    assert_ne!(first.x_model_index, second.x_model_index);
    assert_eq!(algorithm.context.column_count(), 1);
    let current = model.try_to_linear_triad_model().unwrap();
    assert_eq!(current.c[0], -1.0);
    assert!(model.constraints().iter().any(|row| row.name == "extension"));
}

#[test]
fn maximization_certificates_do_not_become_minimization_lower_bounds() {
    let (mut model, mut algorithm) = fixture();
    model.set_linear_objective(vec![1.0, 0.0], ObjectiveCategory::Maximum);
    algorithm.solver.0 = report(&model, vec![2.0, 0.0], vec![1.0, 0.0, 0.0], 2.0);
    algorithm.solve_rmp_lp(&model).unwrap();
    generate(&mut algorithm, &model).unwrap();
    assert_eq!(algorithm.pricing_completion(), Some(ColumnGenerationCompletion::Exhausted));
    assert_eq!(algorithm.certified_lower_bound(&model).unwrap(), None);
}

#[test]
fn hidden_compiled_function_rows_do_not_silently_disappear_from_pricing() {
    use std::sync::Arc;
    use ospf_rust_core::model::FunctionExpansionPolicy;
    use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
    use ospf_rust_core::symbol::function::NotFunction;

    let mut model = MetaModel::new("hidden-pricing-rows");
    model.set_function_expansion_policy(FunctionExpansionPolicy::DeferredNativeFirst);
    let x = model.register_auto_variable::<Binary>("x").unwrap();
    model.add_symbol(Arc::new(NotFunction::new(900_001, "hidden-not",
        Linear::new(vec![LinearMonomial::new(1.0, x)], 0.0)))).unwrap();
    let mut current = model.try_to_linear_triad_model().unwrap();
    current.linear_relax();
    assert!(model.constraints().is_empty());
    assert_eq!(current.num_variables(), 2);
    let source = report(&model, vec![0.0, 1.0], vec![0.0; current.num_constraints()], 0.0);
    assert!(VerifiedPricingDuals::new(&current, &source).is_ok());
    assert!(PricingEvidence::new(&model, &source).is_err());
}

#[cfg(feature = "scip")]
#[test]
#[ignore = "需要真实 SCIP 运行时 / requires the actual SCIP runtime"]
fn native_scip_task_lp_supplies_current_model_bound_pricing_evidence() {
    let (model, previous) = fixture();
    let solver = ospf_rust_framework::ScipColumnGenerationSolver::new().with_lp_subproblem_defaults();
    let mut algorithm = TaskColumnGenerationAlgorithm::new(previous.context, solver, ColumnGenerationPolicy::default());
    let report = algorithm.solve_rmp_lp_report(&model).unwrap();
    assert!(report.is_optimal());
    let policy = Policy(|request: &TaskPricingRequest| {
        assert_eq!(request.shadow_prices[&1], 0.0);
        assert_eq!(request.row_duals["extension"], 0.0);
        assert!(request.verified_duals.is_some());
        complete(request)
    });
    algorithm.generate_task_batch(&model, TaskPricingDomain::unslotted(2, 2), &policy).unwrap();
    assert!((algorithm.certified_lower_bound(&model).unwrap().unwrap() + 2.0).abs() < 1e-8);
}
