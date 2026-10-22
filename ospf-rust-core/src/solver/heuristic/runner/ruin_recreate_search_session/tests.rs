use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use crate::solver::heuristic::operator::neighborhood::ruin_recreate::{GreedyInsertRepair, RandomRuin};
use crate::solver::heuristic::session::codecs::IndividualCodec;
use crate::solver::heuristic::session::SearchBudget;
use crate::variable::{VariableId, new_standalone_id};
use super::*;

struct Problem {
    order: Vec<VariableId>,
    calls: Rc<Cell<u64>>,
    problem_counter: Cell<u64>,
}

impl SearchProblem<f64, f64> for Problem {
    fn id(&self) -> &str { "ruin-test" }
    fn fingerprint(&self) -> &str { "ruin-test-v1" }
    fn variable_order(&self) -> Vec<VariableId> { self.order.clone() }
    fn encoding(&self) -> SearchEncoding { SearchEncoding::Permutation }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> {
        vec![self.order.iter().enumerate().map(|(index, id)| (*id, index as f64)).collect(); amount]
    }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
        self.calls.set(self.calls.get() + 1); self.problem_counter.set(self.problem_counter.get() + 1);
        Some(self.order.iter().enumerate().map(|(index, id)| (index + 1) as f64 * solution[id]).sum())
    }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn feasible(&self, solution: &Solution<f64>) -> bool {
        let mut values: Vec<_> = self.order.iter().filter_map(|id| solution.get(id).copied()).collect(); values.sort_by(f64::total_cmp);
        values == vec![0.0, 1.0, 2.0, 3.0]
    }
    fn save_state(&self) -> Result<JsonValue> { Ok(serde_json::json!({"counter": self.problem_counter.get()})) }
    fn load_state(&mut self, state: &JsonValue) -> Result<()> {
        self.problem_counter.set(state["counter"].as_u64().ok_or_else(|| invalid("bad problem counter"))?); Ok(())
    }
}

struct Callbacks(Rc<Cell<u64>>);

impl EstimateState for Callbacks {
    fn save(&self) -> Result<JsonValue> { Ok(self.0.get().into()) }
    fn load(&mut self, state: &JsonValue) -> Result<()> {
        let value = state.as_u64().ok_or_else(|| invalid("bad estimate state"))?;
        self.0.set(value);
        if value == u64::MAX { return Err(invalid("estimate load failed after mutation")); }
        Ok(())
    }
}

fn session(
    order: Vec<VariableId>,
    estimates: Rc<Cell<u64>>,
    recover: bool,
) -> RuinRecreateSearchSession<Problem, f64, f64> {
    let observed = estimates.clone();
    let operator = RuinRecreateOperator::new(RandomRuin, GreedyInsertRepair::new(move |_: &[f64], item: &f64, position| {
        observed.set(observed.get() + 1);
        Ok(Some((item - position as f64).abs() + (observed.get() % 5) as f64))
    }));
    let codec = Box::new(IndividualCodec::new(order.clone()));
    let problem = Problem { order, calls: Rc::new(Cell::new(0)), problem_counter: Cell::new(0) };
    let session = RuinRecreateSearchSession::new(problem, operator, codec,
        RuinRecreateSessionConfig { amount: 2, random: SearchRandom::new(42), stateless_estimates: false }).unwrap();
    if recover { session.with_estimate_state(Box::new(Callbacks(estimates))) } else { session }
}

#[test]
fn ruin_session_restores_independent_rng_estimates_and_separate_problem_state_without_evaluating() {
    let order: Vec<_> = (0..4).map(|_| new_standalone_id()).collect();
    let estimates = Rc::new(Cell::new(0));
    let mut run = session(order.clone(), estimates.clone(), true);
    let budget = Arc::new(SearchBudget::new(8));
    let report = run.step(&mut SearchSlice::new(budget.clone(), 3, 3)).unwrap();
    assert_eq!((report.steps, report.evaluations, budget.objective_calls()), (3, 3, 3));
    let saved = run.snapshot().unwrap(); let problem_state = run.problem().save_state().unwrap();
    let saved_estimates = estimates.get(); assert!(saved_estimates > 0);
    let actual_report = run.step(&mut SearchSlice::new(budget.clone(), 5, 5)).unwrap();
    let expected = run.snapshot().unwrap();
    let restored_estimates = Rc::new(Cell::new(0));
    let mut restored = session(order, restored_estimates.clone(), true);
    restored.problem_mut().load_state(&problem_state).unwrap(); restored.restore(&saved).unwrap();
    assert_eq!(restored.problem().calls.get(), 0);
    assert_eq!(restored_estimates.get(), saved_estimates);
    let restored_budget = Arc::new(SearchBudget::with_counts(8, 3, 3));
    let replay_report = restored.step(&mut SearchSlice::new(restored_budget.clone(), 5, 5)).unwrap();
    assert_eq!(restored.snapshot().unwrap(), expected);
    assert_eq!(restored.problem().save_state().unwrap(), run.problem().save_state().unwrap());
    assert_eq!(restored_estimates.get(), estimates.get());
    assert_eq!(actual_report.evaluations, replay_report.evaluations);
    assert_eq!(actual_report.candidates.iter().map(|entry| entry.fitness).collect::<Vec<_>>(), replay_report.candidates.iter().map(|entry| entry.fitness).collect::<Vec<_>>());
    assert_eq!((budget.used(), restored_budget.used(), restored_budget.objective_calls()), (8, 8, 8));
    assert_eq!(restored.problem().calls.get(), 5);
}

#[test]
fn ruin_session_imports_share_budget_but_never_advance_iteration_or_random_and_reject_invalid_permutations() {
    let order: Vec<_> = (0..4).map(|_| new_standalone_id()).collect();
    let mut run = session(order.clone(), Rc::new(Cell::new(0)), true);
    let budget = Arc::new(SearchBudget::new(4));
    run.step(&mut SearchSlice::new(budget.clone(), 1, 1)).unwrap();
    let before = run.snapshot().unwrap();
    let better = order.iter().enumerate().map(|(index, id)| (*id, (3 - index) as f64)).collect::<Solution<_>>();
    let duplicate = order.iter().map(|id| (*id, 1.0)).collect();
    let report = run.import_solutions(vec![duplicate, better.clone(), better.clone()], &mut SearchSlice::new(budget.clone(), 2, 99)).unwrap();
    assert_eq!((report.evaluations, report.steps, report.candidates.len()), (2, 0, 1));
    assert_eq!(report.termination, SearchTermination::EvaluationLimit);
    assert_eq!(run.current().unwrap().solution, better);
    assert_eq!(run.iteration().iteration(), 1);
    assert_eq!(run.snapshot().unwrap().state["random"], before.state["random"]);
    assert_eq!((budget.used(), budget.objective_calls()), (3, 2));
}

#[test]
fn ruin_session_requires_recoverable_estimates_and_restoration_is_transactional_on_bad_state() {
    let order: Vec<_> = (0..4).map(|_| new_standalone_id()).collect();
    assert!(session(order.clone(), Rc::new(Cell::new(0)), false).snapshot().is_err());
    let estimates = Rc::new(Cell::new(0));
    let mut run = session(order, estimates.clone(), true);
    run.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(2)), 2, 2)).unwrap();
    let saved = run.snapshot().unwrap(); let before = estimates.get();
    let mut bad = saved.clone(); bad.state["callbackState"] = u64::MAX.into();
    assert!(run.restore(&bad).is_err());
    assert_eq!(estimates.get(), before); assert_eq!(run.snapshot().unwrap(), saved);
    for mutate in [0, 1, 2, 3, 4] {
        let mut bad = saved.clone();
        match mutate {
            0 => bad.version = 2,
            1 => bad.initialized = false,
            2 => bad.state["random"] = "bad".into(),
            3 => bad.state["unknown"] = 0.into(),
            _ => bad.archive = serde_json::json!([1]),
        }
        assert!(run.restore(&bad).is_err()); assert_eq!(run.snapshot().unwrap(), saved);
    }
}

#[test]
fn ruin_session_zero_budget_never_calls_full_objectives_and_stateless_estimates_can_snapshot() {
    let order: Vec<_> = (0..4).map(|_| new_standalone_id()).collect();
    let mut run = session(order, Rc::new(Cell::new(0)), false); run.config.stateless_estimates = true;
    let before = run.snapshot().unwrap();
    let budget = Arc::new(SearchBudget::new(0));
    let report = run.step(&mut SearchSlice::new(budget.clone(), 0, 3)).unwrap();
    assert_eq!((report.evaluations, report.steps, run.problem().calls.get()), (0, 0, 0));
    assert_eq!(report.termination, SearchTermination::EvaluationLimit);
    assert_eq!(run.snapshot().unwrap(), before);
}

#[test]
fn ruin_session_reports_failed_estimate_without_losing_committed_prefix_or_refunding_requests() {
    let order: Vec<_> = (0..4).map(|_| new_standalone_id()).collect();
    let operator = RuinRecreateOperator::new(RandomRuin, GreedyInsertRepair::new(|_: &[f64], _: &f64, _| Err(invalid("estimate failed"))));
    let problem = Problem { order: order.clone(), calls: Rc::new(Cell::new(0)), problem_counter: Cell::new(0) };
    let mut run = RuinRecreateSearchSession::new(problem, operator, Box::new(IndividualCodec::new(order)),
        RuinRecreateSessionConfig { amount: 2, random: SearchRandom::new(42), stateless_estimates: true }).unwrap();
    let budget = Arc::new(SearchBudget::new(5));
    let report = run.step(&mut SearchSlice::new(budget.clone(), 5, 5)).unwrap();
    assert_eq!(report.termination, SearchTermination::Failed);
    assert_eq!((report.steps, report.evaluations, report.candidates.len()), (1, 1, 1));
    assert_eq!((budget.used(), budget.objective_calls()), (1, 1));
    assert!(report.diagnostics[0].contains("estimate failed"));
    assert!(run.current().is_some()); assert_eq!(run.snapshot().unwrap().iteration, 1);
}

struct FailingObjective(Problem);

impl SearchProblem<f64, f64> for FailingObjective {
    fn id(&self) -> &str { "failed-ruin-test" }
    fn fingerprint(&self) -> &str { "failed-ruin-test-v1" }
    fn encoding(&self) -> SearchEncoding { SearchEncoding::Permutation }
    fn variable_order(&self) -> Vec<VariableId> { self.0.variable_order() }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { self.0.initial_solutions(amount) }
    fn feasible(&self, solution: &Solution<f64>) -> bool { self.0.feasible(solution) }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
        if self.0.calls.get() == 1 { panic!("full objective failed after committed prefix"); }
        self.0.objective(solution)
    }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
}

#[test]
fn ruin_session_step_and_import_keep_partial_candidates_after_full_objective_panics() {
    for import in [false, true] {
        let order: Vec<_> = (0..4).map(|_| new_standalone_id()).collect();
        let problem = FailingObjective(Problem { order: order.clone(), calls: Rc::new(Cell::new(0)), problem_counter: Cell::new(0) });
        let seeds = problem.initial_solutions(3);
        let operator = RuinRecreateOperator::new(RandomRuin, GreedyInsertRepair::new(|_: &[f64], item: &f64, position| Ok(Some(item * position as f64))));
        let mut run = RuinRecreateSearchSession::new(problem, operator, Box::new(IndividualCodec::new(order)),
            RuinRecreateSessionConfig { amount: 2, random: SearchRandom::new(42), stateless_estimates: true }).unwrap();
        let budget = Arc::new(SearchBudget::new(5));
        let mut slice = SearchSlice::new(budget.clone(), 5, 5);
        let report = if import { run.import_solutions(seeds, &mut slice) } else { run.step(&mut slice) }.unwrap();
        assert_eq!(report.termination, SearchTermination::Failed);
        assert_eq!((report.evaluations, report.candidates.len(), report.steps), (2, 1, if import { 0 } else { 1 }));
        assert_eq!((budget.used(), budget.objective_calls()), (2, 2));
        assert!(run.current().is_some()); assert!(!report.diagnostics.is_empty());
    }
}

struct VectorProblem(Problem);

struct FailingComparison(Problem);

impl SearchProblem<f64, f64> for FailingComparison {
    fn id(&self) -> &str { "failed-comparison-ruin" }
    fn fingerprint(&self) -> &str { "failed-comparison-ruin-v1" }
    fn encoding(&self) -> SearchEncoding { SearchEncoding::Permutation }
    fn variable_order(&self) -> Vec<VariableId> { self.0.variable_order() }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { self.0.initial_solutions(amount) }
    fn feasible(&self, solution: &Solution<f64>) -> bool { self.0.feasible(solution) }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> { self.0.objective(solution) }
    fn dominates(&self, _lhs: &f64, _rhs: &f64) -> bool { panic!("comparison failed") }
}

#[test]
fn ruin_session_comparison_failure_retains_newly_evaluated_candidate_and_old_current() {
    for import in [false, true] {
        let order: Vec<_> = (0..4).map(|_| new_standalone_id()).collect();
        let problem = FailingComparison(Problem { order: order.clone(), calls: Rc::new(Cell::new(0)), problem_counter: Cell::new(0) });
        let seeds = problem.initial_solutions(3);
        let operator = RuinRecreateOperator::new(RandomRuin, GreedyInsertRepair::new(|_: &[f64], _: &f64, _| Ok(Some(0.0))));
        let mut run = RuinRecreateSearchSession::new(problem, operator, Box::new(IndividualCodec::new(order)),
            RuinRecreateSessionConfig { amount: 2, random: SearchRandom::new(42), stateless_estimates: true }).unwrap();
        let budget = Arc::new(SearchBudget::new(5));
        let mut slice = SearchSlice::new(budget.clone(), 5, 5);
        let report = if import { run.import_solutions(seeds.clone(), &mut slice) } else { run.step(&mut slice) }.unwrap();
        assert_eq!(report.termination, SearchTermination::Failed);
        assert_eq!((report.evaluations, report.candidates.len(), report.steps), (2, 2, if import { 0 } else { 1 }));
        assert_eq!(run.current().unwrap().solution, seeds[0]);
        assert_eq!((budget.used(), budget.objective_calls()), (2, 2));
    }
}

impl SearchProblem<Vec<f64>, f64> for VectorProblem {
    fn id(&self) -> &str { "vector-ruin-test" }
    fn fingerprint(&self) -> &str { "vector-ruin-test-v1" }
    fn encoding(&self) -> SearchEncoding { SearchEncoding::Permutation }
    fn variable_order(&self) -> Vec<VariableId> { self.0.variable_order() }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { self.0.initial_solutions(amount) }
    fn feasible(&self, solution: &Solution<f64>) -> bool { self.0.feasible(solution) }
    fn objective(&self, solution: &Solution<f64>) -> Option<Vec<f64>> {
        let value = self.0.objective(solution)?; Some(vec![value, 100.0 - value])
    }
    fn dominates(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> bool { lhs.iter().zip(rhs).all(|(a, b)| a <= b) && lhs.iter().zip(rhs).any(|(a, b)| a < b) }
    fn multi_objective(&self) -> bool { true }
}

#[test]
fn ruin_session_keeps_original_vector_objectives_incomparable_acceptance_and_lossless_codec() {
    use crate::solver::heuristic::session::codecs::MultiObjectiveIndividualCodec;
    let order: Vec<_> = (0..4).map(|_| new_standalone_id()).collect();
    let make = || {
        let problem = VectorProblem(Problem { order: order.clone(), calls: Rc::new(Cell::new(0)), problem_counter: Cell::new(0) });
        let operator = RuinRecreateOperator::new(RandomRuin, GreedyInsertRepair::new(|_: &[f64], item: &f64, position| Ok(Some(item * position as f64))));
        RuinRecreateSearchSession::new(problem, operator, Box::new(MultiObjectiveIndividualCodec::new(order.clone())),
            RuinRecreateSessionConfig { amount: 4, random: SearchRandom::new(42), stateless_estimates: true }).unwrap()
    };
    let mut run = make();
    let report = run.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(3)), 3, 3)).unwrap();
    assert!(report.candidates.iter().all(|entry| entry.fitness.len() == 2 && entry.fitness[0] + entry.fitness[1] == 100.0));
    let saved = run.snapshot().unwrap(); let mut restored = make(); restored.restore(&saved).unwrap();
    assert_eq!(restored.problem().0.calls.get(), 0);
    run.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(3)), 3, 3)).unwrap();
    restored.step(&mut SearchSlice::new(Arc::new(SearchBudget::new(3)), 3, 3)).unwrap();
    assert_eq!(run.snapshot().unwrap(), restored.snapshot().unwrap());
    assert_eq!(run.current().unwrap().fitness.len(), 2);
}
