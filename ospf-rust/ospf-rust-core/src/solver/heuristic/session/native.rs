//! 原生骨架会话钩子 / Native skeleton session hooks

use std::cell::RefCell;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, to_f64};
use crate::solver::heuristic::runner::native::NativeRunner;
use crate::solver::heuristic::session::adapters::ProblemModel;
use crate::solver::heuristic::session::budget::SearchSlice;
use crate::solver::heuristic::session::protocol::{Result, SearchProblem};
use crate::solver::heuristic::session::runner_session::SessionHooks;

/// 使用原生初始化与推进钩子，不重复执行完整 solver / Use native lifecycle hooks rather than rerunning complete solvers
pub struct NativeHooks<R, V: HeuristicValue> {
    runner: R,
    entropy: Box<dyn Entropy>,
    candidates: RefCell<Vec<Individual<f64, V>>>,
    errors: RefCell<Vec<String>>,
}

impl<R, V: HeuristicValue> NativeHooks<R, V> {
    /// 绑定骨架与会话随机流 / Bind the skeleton and session random stream
    pub fn new(runner: R, entropy: Box<dyn Entropy>) -> Self {
        Self { runner, entropy, candidates: RefCell::new(Vec::new()), errors: RefCell::new(Vec::new()) }
    }
}

impl<R, P, V> SessionHooks<P, f64, V, R::State> for NativeHooks<R, V>
where
    R: NativeRunner<V>,
    P: SearchProblem<f64, V>,
    V: HeuristicValue,
{
    fn initialize(&mut self, problem: &P, slice: &SearchSlice) -> Result<R::State> {
        let model = ProblemModel::metered(problem, slice, &self.candidates).with_errors(&self.errors);
        self.runner.initialize_native(&model, self.entropy.as_mut())
    }

    fn advance(
        &mut self,
        problem: &mut P,
        iteration: &Iteration,
        state: &R::State,
        slice: &SearchSlice,
    ) -> Result<R::State> {
        let model = ProblemModel::metered(problem, slice, &self.candidates).with_errors(&self.errors);
        self.runner.advance_native(&model, iteration, state.clone(), self.entropy.as_mut())
    }

    fn members(&self, state: &R::State) -> Vec<Individual<f64, V>> {
        R::members_native(state).into_iter()
            .map(|individual| Individual::new(individual.solution, to_f64(&individual.fitness)))
            .collect()
    }

    fn drain_candidates(&mut self) -> Vec<Individual<f64, V>> {
        self.candidates.replace(Vec::new())
    }

    fn drain_errors(&mut self) -> Vec<String> { self.errors.replace(Vec::new()) }
}
