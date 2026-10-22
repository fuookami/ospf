//! 种群级搜索骨架 / Population-level search skeleton
//!
//! 一代 = 更新（交叉或位置更新）→ 变异 → 环境选择。所有基于种群的元启发式都是这三步的不同参数化，因此骨架只固定
//! 顺序，不固定算子。
//! One generation is update (crossover or position update) → mutation → survivor selection. Every population-based
//! metaheuristic is a different parameterisation of the three steps, so the skeleton fixes only the order, never the
//! operators.

use crate::model::callback::Solution;

use crate::solver::heuristic::contract::Runner;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel};
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::compare_strictly;
use crate::solver::heuristic::operator::fitness::{DefaultOnMissFitnessEvaluator, FitnessEvaluator};
use crate::solver::heuristic::operator::initializer::{Initializer, ModelInitializer};
use crate::solver::heuristic::operator::mutation::Mutation;
use crate::solver::heuristic::operator::stopping::NeverStop;
use crate::solver::heuristic::operator::survivor::SurvivorSelection;
use crate::solver::heuristic::operator::topology::{GlobalTopology, Topology};
use crate::solver::heuristic::operator::update::SolutionUpdate;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::drive::drive;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// 种群状态 / Population state
pub type Population<V> = Vec<Individual<V, V>>;

/// 种群搜索的算子组合 / Operator composition of the population search
struct PopulationOps<'a, V>
where
    V: HeuristicValue,
{
    evaluator: &'a dyn FitnessEvaluator<V>,
    update: &'a dyn SolutionUpdate<V>,
    mutation: &'a dyn Mutation<V>,
    survivor: &'a dyn SurvivorSelection<V, V>,
    topology: &'a dyn Topology,
    mutation_rate: f64,
    size: usize,
}

fn initialize_population<V, M>(
    initializer: &dyn Initializer<V>,
    evaluator: &dyn FitnessEvaluator<V>,
    model: &M,
    size: usize,
    entropy: &mut dyn Entropy,
) -> Population<V>
where
    V: HeuristicValue,
    M: ObjectiveModel<V, V>,
{
    initializer
        .initialize(model, size, entropy)
        .into_iter()
        .filter_map(|solution| evaluator.evaluate(model, &solution))
        .take(size)
        .collect()
}

fn advance_population<V, M>(
    ops: &PopulationOps<'_, V>,
    model: &M,
    iteration: &Iteration,
    state: &[Individual<V, V>],
    entropy: &mut dyn Entropy,
) -> Population<V>
where
    V: HeuristicValue,
    M: ObjectiveModel<V, V>,
{
    if state.is_empty() {
        return Vec::new();
    }

    let mut offspring: Vec<Individual<V, V>> = Vec::new();
    for index in 0..state.len() {
        // 邻域内的父代先按优度升序排列：交叉本身不关心顺序，而位置更新类算子（GWO 的 α/β/δ 等）
        // 依赖「前 k 名即头狼」的约定，因此由骨架统一保证。
        // Neighbourhood parents are ranked by ascending quality first: crossover does not care about the order, whereas
        // position-update operators rely on "the first k are the leaders", so the skeleton guarantees it uniformly.
        let neighbours = ops.topology.neighbours(index, state.len(), entropy);
        let mut parents: Vec<&Individual<V, V>> =
            neighbours.iter().filter_map(|i| state.get(*i)).collect();
        parents.sort_by(|lhs, rhs| compare_strictly(model, &lhs.fitness, &rhs.fitness));
        let parent_solutions: Vec<Solution<V>> = parents
            .iter()
            .map(|individual| individual.solution.clone())
            .collect();

        for solution in ops.update.update(model, iteration, &parent_solutions, entropy) {
            if let Some(individual) = ops.evaluator.evaluate(model, &solution) {
                offspring.push(individual);
            }
        }
    }

    for individual in state {
        let solution = ops.mutation.mutate(
            model,
            iteration,
            &individual.solution,
            ops.mutation_rate,
            entropy,
        );
        if let Some(mutant) = ops.evaluator.evaluate(model, &solution) {
            offspring.push(mutant);
        }
    }

    ops.survivor
        .select(model, state, &offspring, ops.size)
}

/// 种群级搜索骨架 / Population-level search skeleton
pub struct PopulationRunner<V>
where
    V: HeuristicValue,
{
    core: RunnerCore<V, V>,
    size: usize,
    mutation_rate: f64,
    initializer: Box<dyn Initializer<V>>,
    fitness_evaluator: Box<dyn FitnessEvaluator<V>>,
    update: Box<dyn SolutionUpdate<V>>,
    mutation: Box<dyn Mutation<V>>,
    survivor: Box<dyn SurvivorSelection<V, V>>,
    topology: Box<dyn Topology>,
    entropy: Box<dyn Entropy>,
}

impl<V> PopulationRunner<V>
where
    V: HeuristicValue,
{
    /// 创建种群骨架 / Create a population skeleton
    pub fn new(
        size: usize,
        update: Box<dyn SolutionUpdate<V>>,
        mutation: Box<dyn Mutation<V>>,
        survivor: Box<dyn SurvivorSelection<V, V>>,
        entropy: Box<dyn Entropy>,
    ) -> Self {
        Self {
            core: RunnerCore::new(Box::new(BestArchive::new(1)), Box::new(NeverStop)),
            size,
            mutation_rate: 0.1,
            initializer: Box::new(ModelInitializer),
            fitness_evaluator: Box::new(DefaultOnMissFitnessEvaluator),
            update,
            mutation,
            survivor,
            topology: Box::new(GlobalTopology),
            entropy,
        }
    }

    /// 设置变异率 / Set the mutation rate
    pub fn with_mutation_rate(mut self, mutation_rate: f64) -> Self {
        self.mutation_rate = mutation_rate;
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

    /// 设置拓扑 / Set the topology
    pub fn with_topology(mut self, topology: Box<dyn Topology>) -> Self {
        self.topology = topology;
        self
    }

    /// 设置搜索核心（档案、终止、观测者）/ Set the search core (archive, stopping, observers)
    pub fn with_core(mut self, core: RunnerCore<V, V>) -> Self {
        self.core = core;
        self
    }

    /// 搜索核心引用 / Search core reference
    pub fn core(&self) -> &RunnerCore<V, V> {
        &self.core
    }

    /// 种群规模 / Population size
    pub fn size(&self) -> usize {
        self.size
    }

    /// 生成一个岛屿的初始种群 / Build the initial population of one island
    pub fn initialize_island<M>(
        &self,
        model: &M,
        entropy: &mut dyn Entropy,
    ) -> Population<V>
    where
        M: ObjectiveModel<V, V>,
    {
        initialize_population(
            self.initializer.as_ref(),
            self.fitness_evaluator.as_ref(),
            model,
            self.size,
            entropy,
        )
    }

    /// 推进一个岛屿一代 / Advance one island by one generation
    pub fn step_island<M>(
        &self,
        model: &M,
        iteration: &Iteration,
        state: &[Individual<V, V>],
        entropy: &mut dyn Entropy,
    ) -> Population<V>
    where
        M: ObjectiveModel<V, V>,
    {
        let ops = PopulationOps {
            evaluator: self.fitness_evaluator.as_ref(),
            update: self.update.as_ref(),
            mutation: self.mutation.as_ref(),
            survivor: self.survivor.as_ref(),
            topology: self.topology.as_ref(),
            mutation_rate: self.mutation_rate,
            size: self.size,
        };
        advance_population(&ops, model, iteration, state, entropy)
    }
}

impl<V> Runner<V, V> for PopulationRunner<V>
where
    V: HeuristicValue,
{
    fn run<M>(&mut self, model: &mut M, iteration: &mut Iteration) -> Result<Population<V>>
    where
        M: ObjectiveModel<V, V>,
    {
        let Self {
            core,
            size,
            mutation_rate,
            initializer,
            fitness_evaluator,
            update,
            mutation,
            survivor,
            topology,
            entropy,
        } = self;

        let state =
            initialize_population(initializer.as_ref(), fitness_evaluator.as_ref(), model, *size, entropy.as_mut());
        let ops = PopulationOps {
            evaluator: fitness_evaluator.as_ref(),
            update: update.as_ref(),
            mutation: mutation.as_ref(),
            survivor: survivor.as_ref(),
            topology: topology.as_ref(),
            mutation_rate: *mutation_rate,
            size: *size,
        };

        drive(
            core,
            model,
            iteration,
            state,
            |model, iteration, state, entropy| {
                Ok(advance_population(&ops, model, iteration, &state, entropy))
            },
            |state| state.clone(),
            entropy.as_mut(),
        )
    }
}

impl<V> crate::solver::heuristic::runner::native::NativeRunner<V> for PopulationRunner<V>
where
    V: HeuristicValue,
{
    type State = Population<V>;

    fn initialize_native<M>(&mut self, model: &M, entropy: &mut dyn Entropy) -> Result<Self::State>
    where
        M: ObjectiveModel<V, V>,
    {
        Ok(self.initialize_island(model, entropy))
    }

    fn advance_native<M>(
        &mut self,
        model: &M,
        iteration: &Iteration,
        state: Self::State,
        entropy: &mut dyn Entropy,
    ) -> Result<Self::State>
    where
        M: ObjectiveModel<V, V>,
    {
        Ok(self.step_island(model, iteration, &state, entropy))
    }

    fn members_native(state: &Self::State) -> Vec<Individual<V, V>> {
        state.clone()
    }

    fn core_native(&mut self) -> &mut RunnerCore<V, V> {
        &mut self.core
    }
}
