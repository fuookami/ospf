//! 独占问题句柄与可恢复随机流 / Owned problem handles and restorable random streams

use std::cell::RefCell;
use std::rc::Rc;
use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::session::{JsonValue, SearchEncoding, SearchProblem, SearchRandom};
use crate::variable::VariableId;

/// 仅在一个 worker 内共享的问题所有权 / Problem ownership shared only within one worker
pub struct ProblemHandle<O, V> {
    inner: Rc<RefCell<Box<dyn SearchProblem<O, V>>>>,
    id: String,
    fingerprint: String,
}

impl<O, V> Clone for ProblemHandle<O, V> {
    fn clone(&self) -> Self { Self { inner: self.inner.clone(), id: self.id.clone(), fingerprint: self.fingerprint.clone() } }
}

impl<O: Clone, V: HeuristicValue> ProblemHandle<O, V> {
    /// 接管一个独立问题实例 / Take ownership of an independent problem instance
    pub fn new(problem: Box<dyn SearchProblem<O, V>>) -> Self {
        Self { id: problem.id().into(), fingerprint: problem.fingerprint().into(), inner: Rc::new(RefCell::new(problem)) }
    }
}

impl<O: Clone, V: HeuristicValue> SearchProblem<O, V> for ProblemHandle<O, V> {
    fn id(&self) -> &str { &self.id }
    fn fingerprint(&self) -> &str { &self.fingerprint }
    fn variable_order(&self) -> Vec<VariableId> { self.inner.borrow().variable_order() }
    fn lower_bound(&self, index: usize) -> Option<f64> { self.inner.borrow().lower_bound(index) }
    fn upper_bound(&self, index: usize) -> Option<f64> { self.inner.borrow().upper_bound(index) }
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<V>> { self.inner.borrow().initial_solutions(amount) }
    fn objective(&self, solution: &Solution<V>) -> Option<O> { self.inner.borrow().objective(solution) }
    fn dominates(&self, lhs: &O, rhs: &O) -> bool { self.inner.borrow().dominates(lhs, rhs) }
    fn equivalent(&self, lhs: &O, rhs: &O) -> bool { self.inner.borrow().equivalent(lhs, rhs) }
    fn valid_objective(&self, objective: &O) -> bool { self.inner.borrow().valid_objective(objective) }
    fn feasible(&self, solution: &Solution<V>) -> bool { self.inner.borrow().feasible(solution) }
    fn admissible(&self, solution: &Solution<V>) -> bool { self.inner.borrow().admissible(solution) }
    fn constraint_admissible(&self, solution: &Solution<V>) -> bool { self.inner.borrow().constraint_admissible(solution) }
    fn evaluate_constraint_admissible(
        &self,
        solution: Solution<V>,
        slice: &crate::solver::heuristic::session::SearchSlice,
    ) -> Result<crate::solver::heuristic::session::SearchEvaluationResult<O, V>> {
        self.inner.borrow().evaluate_constraint_admissible(solution, slice)
    }
    fn has_constraints(&self) -> bool { self.inner.borrow().has_constraints() }
    fn stochastic_constraints(&self) -> bool { self.inner.borrow().stochastic_constraints() }
    fn observe_constraints(&self, solution: &Solution<V>) -> Result<()> { self.inner.borrow().observe_constraints(solution) }
    fn constraint_failure(&self) -> Option<String> { self.inner.borrow().constraint_failure() }
    fn constraint_tier(&self, solution: &Solution<V>) -> Option<(u8, f64)> { self.inner.borrow().constraint_tier(solution) }
    fn begin_constraints(&self) { self.inner.borrow().begin_constraints(); }
    fn commit_constraints(&self) -> Result<()> { self.inner.borrow().commit_constraints() }
    fn save_constraints(&self) -> Result<JsonValue> { self.inner.borrow().save_constraints() }
    fn load_constraints(&self, state: &JsonValue) -> Result<()> { self.inner.borrow().load_constraints(state) }
    fn compare_individuals(&self, lhs: &crate::solver::heuristic::Individual<O, V>, rhs: &crate::solver::heuristic::Individual<O, V>) -> std::cmp::Ordering {
        self.inner.borrow().compare_individuals(lhs, rhs)
    }
    fn rank_indices(&self, entries: &[crate::solver::heuristic::Individual<O, V>]) -> Vec<usize> { self.inner.borrow().rank_indices(entries) }
    fn rank_constraints(
        &self,
        solutions: &[Solution<V>],
        objective: &dyn Fn(usize, usize) -> std::cmp::Ordering,
    ) -> Option<Vec<usize>> { self.inner.borrow().rank_constraints(solutions, objective) }
    fn has_objective_coordinates(&self) -> bool { self.inner.borrow().has_objective_coordinates() }
    fn objective_coordinates(&self, objective: &O) -> Result<Vec<f64>> { self.inner.borrow().objective_coordinates(objective) }
    fn checked_objective_coordinates(&self, objective: &O) -> Result<Vec<f64>> { self.inner.borrow().checked_objective_coordinates(objective) }
    fn has_constraint_violation(&self) -> bool { self.inner.borrow().has_constraint_violation() }
    fn constraint_violation(&self, solution: &Solution<V>) -> Result<f64> { self.inner.borrow().constraint_violation(solution) }
    fn checked_constraint_violation(&self, solution: &Solution<V>) -> Result<f64> { self.inner.borrow().checked_constraint_violation(solution) }
    fn multi_objective(&self) -> bool { self.inner.borrow().multi_objective() }
    fn valid_solution(&self, solution: &Solution<V>) -> bool { self.inner.borrow().valid_solution(solution) }
    fn encoding(&self) -> SearchEncoding { self.inner.borrow().encoding() }
    fn flush(&mut self) { self.inner.borrow_mut().flush(); }
    fn stateless(&self) -> bool { self.inner.borrow().stateless() }
    fn save_state(&self) -> Result<JsonValue> { self.inner.borrow().save_state() }
    fn load_state(&mut self, state: &JsonValue) -> Result<()> { self.inner.borrow_mut().load_state(state) }
    fn evaluate(
        &self,
        solution: Solution<V>,
        slice: &crate::solver::heuristic::session::SearchSlice,
    ) -> Result<crate::solver::heuristic::session::protocol::SearchEvaluationResult<O, V>> {
        self.inner.borrow().evaluate(solution, slice)
    }
}

/// 与骨架实际使用的同一个可恢复随机流 / The same restorable random stream actually used by the skeleton
#[derive(Clone)]
pub struct RandomHandle(Rc<RefCell<SearchRandom>>);

impl RandomHandle {
    /// 创建独立随机流 / Create an independent stream
    pub fn new(random: SearchRandom) -> Self { Self(Rc::new(RefCell::new(random))) }
    /// 导出随机状态位模式 / Export the random state bit pattern
    pub fn state(&self) -> i64 { self.0.borrow().state() as i64 }
    /// 恢复随机状态 / Restore random state
    pub fn restore(&self, state: i64) { *self.0.borrow_mut() = SearchRandom::new(state as u64); }
}

impl Entropy for RandomHandle {
    fn next_unit(&mut self) -> f64 { self.0.borrow_mut().next_unit() }
}
