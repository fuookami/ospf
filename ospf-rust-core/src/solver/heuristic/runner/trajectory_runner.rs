//! 轨迹级搜索骨架 / Trajectory-level search skeleton
//!
//! 一步 = 邻域候选 → 接受判据，状态始终是一个个体，适用于模拟退火、爬山、禁忌搜索与变邻域搜索。
//! One step is neighbourhood candidate → acceptance, with a single individual as the state; suits simulated annealing,
//! hill climbing, tabu search, and variable neighbourhood search.

use crate::solver::heuristic::contract::Runner;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel};
use crate::solver::heuristic::operator::acceptance::Acceptance;
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::fitness::{DefaultOnMissFitnessEvaluator, FitnessEvaluator};
use crate::solver::heuristic::operator::initializer::{Initializer, ModelInitializer};
use crate::solver::heuristic::operator::neighborhood::Neighborhood;
use crate::solver::heuristic::operator::stopping::NeverStop;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::drive::drive;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// 轨迹级搜索骨架 / Trajectory-level search skeleton
pub struct TrajectoryRunner<V>
where
    V: HeuristicValue,
{
    core: RunnerCore<V, V>,
    initializer: Box<dyn Initializer<V>>,
    neighborhood: Box<dyn Neighborhood<V>>,
    acceptance: Box<dyn Acceptance<V, V>>,
    fitness_evaluator: Box<dyn FitnessEvaluator<V>>,
    entropy: Box<dyn Entropy>,
}

impl<V> TrajectoryRunner<V>
where
    V: HeuristicValue,
{
    /// 创建轨迹骨架 / Create a trajectory skeleton
    pub fn new(
        neighborhood: Box<dyn Neighborhood<V>>,
        acceptance: Box<dyn Acceptance<V, V>>,
        entropy: Box<dyn Entropy>,
    ) -> Self {
        Self {
            core: RunnerCore::new(Box::new(BestArchive::new(1)), Box::new(NeverStop)),
            initializer: Box::new(ModelInitializer),
            neighborhood,
            acceptance,
            fitness_evaluator: Box::new(DefaultOnMissFitnessEvaluator),
            entropy,
        }
    }

    /// 设置接受判据 / Set the acceptance criterion
    pub fn with_acceptance(mut self, acceptance: Box<dyn Acceptance<V, V>>) -> Self {
        self.acceptance = acceptance;
        self
    }

    /// 设置邻域算子 / Set the neighbourhood operator
    pub fn with_neighborhood(mut self, neighborhood: Box<dyn Neighborhood<V>>) -> Self {
        self.neighborhood = neighborhood;
        self
    }

    /// 设置初始化算子 / Set the initializer
    pub fn with_initializer(mut self, initializer: Box<dyn Initializer<V>>) -> Self {
        self.initializer = initializer;
        self
    }

    /// 设置适应度评估算子 / Set the fitness evaluator
    pub fn with_fitness_evaluator(mut self, evaluator: Box<dyn FitnessEvaluator<V>>) -> Self {
        self.fitness_evaluator = evaluator;
        self
    }

    /// 设置搜索核心 / Set the search core
    pub fn with_core(mut self, core: RunnerCore<V, V>) -> Self {
        self.core = core;
        self
    }

    /// 搜索核心引用 / Search core reference
    pub fn core(&self) -> &RunnerCore<V, V> {
        &self.core
    }
}

impl<V> Runner<V, V> for TrajectoryRunner<V>
where
    V: HeuristicValue,
{
    fn run<M>(&mut self, model: &mut M, iteration: &mut Iteration) -> Result<Vec<Individual<V, V>>>
    where
        M: ObjectiveModel<V, V>,
    {
        let Self {
            core,
            initializer,
            neighborhood,
            acceptance,
            fitness_evaluator,
            entropy,
        } = self;

        let state = initializer
            .initialize(model, 1, entropy.as_mut())
            .into_iter()
            .next()
            .and_then(|solution| fitness_evaluator.evaluate(model, &solution));

        drive(
            core,
            model,
            iteration,
            state,
            |model, iteration, state, entropy| {
                let current = match state {
                    Some(current) => current,
                    None => return Ok(None),
                };
                let neighbour = neighborhood.neighbor(model, iteration, &current.solution, entropy);
                let candidate = match fitness_evaluator.evaluate(model, &neighbour) {
                    Some(candidate) => candidate,
                    None => return Ok(Some(current)),
                };
                if acceptance.accept(model, iteration, &current, &candidate, entropy) {
                    Ok(Some(candidate))
                } else {
                    Ok(Some(current))
                }
            },
            |state| state.iter().cloned().collect::<Vec<_>>(),
            entropy.as_mut(),
        )
    }
}
