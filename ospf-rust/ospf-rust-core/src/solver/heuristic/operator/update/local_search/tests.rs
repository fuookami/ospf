use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use crate::solver::heuristic::entropy::SeededEntropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::operator::mutation::SwapMutation;
use crate::solver::heuristic::operator::neighborhood::Neighborhood;
use crate::solver::heuristic::session::adapters::{MultiObjectiveProblemModel, ProblemModel};
use crate::solver::heuristic::session::budget::{SearchBudget, SearchSlice};
use crate::solver::heuristic::session::protocol::{Result, SearchEvaluationResult, SearchProblem};
use crate::solver::heuristic::result::HeuristicError;
use crate::variable::{VariableId, new_standalone_id};
use super::*;

struct Problem { x: VariableId, y: VariableId }

impl Problem {
    fn new() -> Self { Self { x: new_standalone_id(), y: new_standalone_id() } }
    fn solution(&self, x: f64) -> Solution<f64> { Solution::from([(self.x, x), (self.y, 0.0)]) }
}

impl SearchProblem<f64, f64> for Problem {
    fn id(&self) -> &str { "local" }
    fn fingerprint(&self) -> &str { "local-v1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.x, self.y] }
    fn lower_bound(&self, index: usize) -> Option<f64> { (index == 0).then_some(-10.0) }
    fn upper_bound(&self, index: usize) -> Option<f64> { (index == 0).then_some(10.0) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { vec![self.solution(2.0); amount] }
    fn objective(&self, solution: &Solution<f64>) -> Option<f64> { Some(solution.get(&self.x)?.powi(2)) }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
}

struct VectorProblem(Problem);

impl SearchProblem<Vec<f64>, f64> for VectorProblem {
    fn id(&self) -> &str { "vector-local" }
    fn fingerprint(&self) -> &str { "vector-local-v1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.0.x, self.0.y] }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<f64>> { vec![self.0.solution(2.0); amount] }
    fn objective(&self, solution: &Solution<f64>) -> Option<Vec<f64>> {
        let value = *solution.get(&self.0.x)?;
        Some(vec![-100.0 * value, value])
    }
    fn dominates(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> bool { lhs[1] < rhs[1] }
}

struct Shift { amount: f64, threshold: f64, trace: Rc<RefCell<Vec<f64>>> }

impl SolutionUpdate<f64> for Shift {
    fn update(
        &self,
        model: &dyn VariableModel<f64>,
        _iteration: &Iteration,
        parents: &[Solution<f64>],
        _entropy: &mut dyn Entropy,
    ) -> Vec<Solution<f64>> {
        let x = model.variable_order()[0];
        parents.iter().map(|parent| {
            let value = parent[&x]; self.trace.borrow_mut().push(value);
            let mut result = parent.clone();
            if value >= self.threshold { result.insert(x, value + self.amount); }
            result
        }).collect()
    }
}

fn shift(amount: f64, threshold: f64, trace: &Rc<RefCell<Vec<f64>>>) -> Box<dyn SolutionUpdate<f64>> {
    Box::new(Shift { amount, threshold, trace: trace.clone() })
}

#[test]
fn ledger_handles_share_real_budget_reject_cross_ledger_and_never_reevaluate_comparisons() {
    let problem = Problem::new();
    let budget = Arc::new(SearchBudget::new(2));
    let slice = SearchSlice::new(budget.clone(), 10, 1);
    let entries = RefCell::new(Vec::new());
    let model = ProblemModel::metered(&problem, &slice, &entries);
    let ledger = model.local_evaluation().unwrap();
    let first = EvaluatedSolution::evaluate(ledger, problem.solution(2.0)).unwrap();
    let second = EvaluatedSolution::evaluate(ledger, problem.solution(1.0)).unwrap();
    assert_eq!(first.compare(&second), Some(Ordering::Greater));
    assert_eq!(EvaluatedSolution::rank(&[first.clone(), second.clone()]), Some(vec![1, 0]));
    assert!(EvaluatedSolution::evaluate(ledger, problem.solution(0.0)).is_none());
    assert_eq!((slice.used(), budget.used(), budget.objective_calls(), entries.borrow().len()), (2, 2, 2, 2));
    let other_slice = SearchSlice::new(Arc::new(SearchBudget::new(1)), 1, 1);
    let other_entries = RefCell::new(Vec::new());
    let other_model = ProblemModel::metered(&problem, &other_slice, &other_entries);
    let other = EvaluatedSolution::evaluate(other_model.local_evaluation().unwrap(), problem.solution(1.0)).unwrap();
    assert_eq!(first.compare(&other), None);
    assert_eq!(EvaluatedSolution::rank(&[first, other]), None);
    assert_eq!(budget.objective_calls(), 2);
    assert!(ProblemModel::new(&problem).local_evaluation().is_none());
}

#[test]
fn pattern_search_polls_patterns_and_reserves_final_evaluation_without_free_fitness() {
    let problem = Problem::new();
    let budget = Arc::new(SearchBudget::new(8));
    let slice = SearchSlice::new(budget.clone(), 8, 1);
    let entries = RefCell::new(Vec::new());
    let model = ProblemModel::metered(&problem, &slice, &entries);
    let result = PatternSearchUpdate::default().update(&model, &Iteration::new(), &[problem.solution(2.0)], &mut SeededEntropy::new(1));
    assert_eq!(result.len(), 1);
    assert!((result[0][&problem.x] - 1.6).abs() < 1.0e-12);
    let evaluated: Vec<_> = entries.borrow().iter().map(|entry| entry.solution[&problem.x]).collect();
    assert_eq!(evaluated.len(), 6);
    for (actual, expected) in evaluated.iter().zip([2.0, 2.2, 1.8, 1.8, 1.8, 1.6]) { assert!((actual - expected).abs() < 1.0e-12); }
    assert_eq!((budget.used(), budget.objective_calls(), slice.remaining()), (6, 6, 2));
    assert!(problem.evaluate(result[0].clone(), &slice).unwrap().candidate.is_some());
    assert_eq!((budget.used(), budget.objective_calls()), (7, 7));
}

#[test]
fn pattern_search_converges_and_handles_empty_tiny_or_invalid_allowances() {
    let problem = Problem::new();
    for limit in [0, 1, 2, 512] {
        let budget = Arc::new(SearchBudget::new(limit));
        let slice = SearchSlice::new(budget.clone(), limit, 1);
        let entries = RefCell::new(Vec::new());
        let model = ProblemModel::metered(&problem, &slice, &entries);
        let result = PatternSearchUpdate::default().update(&model, &Iteration::new(), &[problem.solution(2.0)], &mut SeededEntropy::new(1));
        assert_eq!(budget.used(), budget.objective_calls());
        assert!(budget.used() <= limit);
        if limit == 0 { assert!(result.is_empty()); }
        else if limit <= 2 { assert_eq!(result[0], problem.solution(2.0)); assert_eq!(budget.used(), 1); }
        else { assert!(result[0][&problem.x].abs() < 1.0e-6); }
    }
    for update in [PatternSearchUpdate { initial_step: f64::NAN, ..Default::default() },
        PatternSearchUpdate { contraction: 0.0, ..Default::default() }, PatternSearchUpdate { sweeps: 0, ..Default::default() }] {
        let budget = Arc::new(SearchBudget::new(10));
        let slice = SearchSlice::new(budget.clone(), 10, 1);
        let entries = RefCell::new(Vec::new());
        assert!(update.update(&ProblemModel::metered(&problem, &slice, &entries), &Iteration::new(), &[problem.solution(2.0)], &mut SeededEntropy::new(1)).is_empty());
        assert_eq!(budget.used(), 0);
    }
}

#[test]
fn pipeline_reevaluates_all_stages_and_orders_next_stage_by_real_objective() {
    let problem = Problem::new();
    let budget = Arc::new(SearchBudget::new(10));
    let slice = SearchSlice::new(budget.clone(), 10, 1);
    let entries = RefCell::new(Vec::new());
    let model = ProblemModel::metered(&problem, &slice, &entries);
    let trace = Rc::new(RefCell::new(Vec::new()));
    let pipeline = PipelineCompositeUpdate::new(vec![Box::new(OppositionUpdate), shift(0.0, 0.0, &trace)]);
    let result = pipeline.update(&model, &Iteration::new(), &[problem.solution(2.0), problem.solution(1.0)], &mut SeededEntropy::new(2));
    assert_eq!(*trace.borrow(), vec![-1.0, -2.0]);
    assert_eq!(result.iter().map(|entry| entry[&problem.x]).collect::<Vec<_>>(), vec![-1.0, -2.0]);
    assert_eq!((budget.used(), budget.objective_calls(), entries.borrow().len()), (6, 6, 6));
}

#[test]
fn vnd_restarts_first_neighborhood_on_improvement_and_honors_trial_limit() {
    for (trials, expected_trace) in [(50, vec![4.0, 3.0, 2.0, 2.0, 1.0, 1.0]), (4, vec![4.0, 3.0, 2.0, 2.0])] {
        let problem = Problem::new();
        let budget = Arc::new(SearchBudget::new(20));
        let slice = SearchSlice::new(budget.clone(), 20, 1);
        let entries = RefCell::new(Vec::new());
        let model = ProblemModel::metered(&problem, &slice, &entries);
        let trace = Rc::new(RefCell::new(Vec::new()));
        let vnd = VNDCompositeUpdate::new(vec![shift(-1.0, 3.0, &trace), shift(-1.0, 2.0, &trace)], trials);
        let result = vnd.update(&model, &Iteration::new(), &[problem.solution(4.0)], &mut SeededEntropy::new(1));
        assert_eq!(result[0][&problem.x], 1.0);
        assert_eq!(*trace.borrow(), expected_trace);
        assert_eq!(budget.objective_calls(), 1 + expected_trace.len() as u64);
    }
}

#[test]
fn vector_pipeline_and_vnd_keep_full_objectives_instead_of_summing_or_reusing_fitness() {
    let problem = VectorProblem(Problem::new());
    let budget = Arc::new(SearchBudget::new(20));
    let slice = SearchSlice::new(budget.clone(), 20, 1);
    let entries = RefCell::new(Vec::new());
    let model = MultiObjectiveProblemModel::metered(&problem, &slice, &entries);
    let trace = Rc::new(RefCell::new(Vec::new()));
    let pipeline = PipelineCompositeUpdate::new(vec![shift(0.0, 0.0, &trace), shift(0.0, 0.0, &trace)]);
    let result = pipeline.update(&model, &Iteration::new(), &[problem.0.solution(3.0), problem.0.solution(1.0)], &mut SeededEntropy::new(1));
    assert_eq!(result[0][&problem.0.x], 1.0);
    assert_eq!(*trace.borrow(), vec![3.0, 1.0, 1.0, 3.0]);
    let vnd = VNDCompositeUpdate::new(vec![shift(-1.0, 2.0, &trace)], 50);
    assert_eq!(vnd.update(&model, &Iteration::new(), &[problem.0.solution(3.0)], &mut SeededEntropy::new(1))[0][&problem.0.x], 1.0);
    assert_eq!((budget.used(), budget.objective_calls(), entries.borrow().len()), (10, 10, 10));
    assert!(entries.borrow().iter().all(|entry| entry.fitness == vec![-100.0 * entry.solution[&problem.0.x], entry.solution[&problem.0.x]]));
}

#[test]
fn opposition_leaves_unbounded_dimensions_and_elite_wrapper_retains_nonelite() {
    let problem = Problem::new();
    let model = ProblemModel::new(&problem);
    let mut parent = problem.solution(3.0); parent.insert(problem.y, 7.0);
    let parents = vec![parent, problem.solution(5.0), problem.solution(6.0)];
    let wrapper = EliteLocalSearchWrapper::new(Box::new(OppositionUpdate), 1);
    let result = wrapper.update(&model, &Iteration::new(), &parents, &mut SeededEntropy::new(1));
    assert_eq!(result[0][&problem.x], -3.0);
    assert_eq!(result[0][&problem.y], 7.0);
    assert_eq!(result[1..], parents[1..]);
    assert!(PatternSearchUpdate::default().update(&model, &Iteration::new(), &parents, &mut SeededEntropy::new(1)).is_empty());
    assert!(PipelineCompositeUpdate::new(vec![Box::new(OppositionUpdate)]).update(&model, &Iteration::new(), &parents, &mut SeededEntropy::new(1)).is_empty());
}

struct SwapNeighborhood;

impl Neighborhood<f64> for SwapNeighborhood {
    fn neighbor(
        &self,
        model: &dyn VariableModel<f64>,
        _iteration: &Iteration,
        solution: &Solution<f64>,
        _entropy: &mut dyn Entropy,
    ) -> Solution<f64> {
        let order = model.variable_order();
        Solution::from([(order[0], solution[&order[1]]), (order[1], solution[&order[0]])])
    }
}

#[test]
fn mutation_and_neighborhood_stages_preserve_permutation_and_do_not_charge_fictitious_objectives() {
    let problem = Problem::new();
    let model = ProblemModel::new(&problem);
    let parents = vec![problem.solution(2.0), problem.solution(3.0)];
    for update in [Box::new(MutationUpdate::new(Box::new(SwapMutation), 1.0)) as Box<dyn SolutionUpdate<f64>>, Box::new(NeighborhoodUpdate::new(Box::new(SwapNeighborhood)))] {
        let result = update.update(&model, &Iteration::new(), &parents, &mut SeededEntropy::new(1));
        for (source, target) in parents.iter().zip(result) {
            assert_eq!(target[&problem.x], 0.0);
            assert_eq!(target[&problem.y], source[&problem.x]);
        }
    }
}

struct ErrorProblem(Problem);

impl SearchProblem<f64, f64> for ErrorProblem {
    fn id(&self) -> &str { "error-local" }
    fn fingerprint(&self) -> &str { "error-local-v1" }
    fn variable_order(&self) -> Vec<VariableId> { vec![self.0.x, self.0.y] }
    fn initial_solutions(&self, _amount: usize) -> Vec<Solution<f64>> { Vec::new() }
    fn objective(&self, _solution: &Solution<f64>) -> Option<f64> { panic!("custom evaluate must be retained") }
    fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs < rhs }
    fn evaluate(&self, _solution: Solution<f64>, slice: &SearchSlice) -> Result<SearchEvaluationResult<f64, f64>> {
        assert!(slice.acquire_shared());
        Err(HeuristicError::InvalidArgument("local evaluation failed".into()))
    }
}

#[test]
fn ledger_retains_custom_evaluation_errors_and_stops_after_first_failure_without_refunding() {
    let problem = ErrorProblem(Problem::new());
    let budget = Arc::new(SearchBudget::new(10));
    let slice = SearchSlice::new(budget.clone(), 10, 1);
    let entries: RefCell<Vec<Individual<f64, f64>>> = RefCell::new(Vec::new());
    let errors = RefCell::new(Vec::new());
    let model = ProblemModel::metered(&problem, &slice, &entries).with_errors(&errors);
    let pipeline = PipelineCompositeUpdate::new(vec![Box::new(OppositionUpdate)]);
    assert!(pipeline.update(&model, &Iteration::new(), &[problem.0.solution(1.0), problem.0.solution(2.0)], &mut SeededEntropy::new(1)).is_empty());
    assert_eq!((budget.used(), budget.objective_calls(), entries.borrow().len(), errors.borrow().len()), (1, 0, 0, 1));
}
