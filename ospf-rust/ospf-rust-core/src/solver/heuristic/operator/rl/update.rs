//! Q 学习算子选择 / Q-learning operator selection

use std::cell::RefCell;
use std::rc::Rc;

use crate::model::callback::Solution;

use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel};
use crate::solver::heuristic::operator::update::SolutionUpdate;
use crate::solver::heuristic::runner::iteration_observer::IterationObserver;

use super::agent::QlearningAgent;

/// 用 Q 学习在多个候选解更新算子之间选择 / Selects among several candidate solution-update operators by Q-learning
pub struct QlearningUpdate<V>
where
    V: HeuristicValue,
{
    candidates: Vec<Box<dyn SolutionUpdate<V>>>,
    agent: Rc<RefCell<QlearningAgent<V>>>,
}

impl<V> QlearningUpdate<V>
where
    V: HeuristicValue,
{
    /// 创建 Q 学习解更新 / Create a Q-learning solution update
    pub fn new(
        candidates: Vec<Box<dyn SolutionUpdate<V>>>,
        agent: Rc<RefCell<QlearningAgent<V>>>,
    ) -> Self {
        Self { candidates, agent }
    }

    /// 候选算子数量 / Number of candidate operators
    pub fn candidate_amount(&self) -> usize {
        self.candidates.len()
    }
}

impl<V> SolutionUpdate<V> for QlearningUpdate<V>
where
    V: HeuristicValue,
{
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        parents: &[Solution<V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        if self.candidates.is_empty() {
            return Vec::new();
        }
        let index = self.agent.borrow().action().min(self.candidates.len() - 1);
        self.candidates[index].update(model, iteration, parents, entropy)
    }
}

/// 把每轮的「是否改进」喂给 Q 学习代理的观测者 / Observer feeding each round's improvement into the Q-learning agent
pub struct QlearningObserver<V>
where
    V: HeuristicValue,
{
    agent: Rc<RefCell<QlearningAgent<V>>>,
}

impl<V> QlearningObserver<V>
where
    V: HeuristicValue,
{
    /// 创建观测者 / Create an observer
    pub fn new(agent: Rc<RefCell<QlearningAgent<V>>>) -> Self {
        Self { agent }
    }
}

impl<V> IterationObserver<V, V> for QlearningObserver<V>
where
    V: HeuristicValue,
{
    fn observe(
        &mut self,
        iteration: &Iteration,
        members: &[Individual<V, V>],
        improved: bool,
    ) {
        let solutions: Vec<Solution<V>> = members
            .iter()
            .map(|individual| individual.solution.clone())
            .collect();
        self.agent
            .borrow_mut()
            .step(iteration, &solutions, improved);
    }
}
