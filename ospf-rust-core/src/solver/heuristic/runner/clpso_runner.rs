//! 综合学习粒子群（CLPSO）/ Comprehensive learning particle swarm optimization
//!
//! 标准 PSO 的每颗粒子只向自己的历史最优与全局最优学习，一旦两者都在局部最优上，整群一起停滞。CLPSO 让
//! **每个维度各自选一个学习对象**（来自别的粒子的历史最优），并定期刷新，因此同一颗粒子的不同维度可以朝不同方向学习。
//! A standard PSO particle learns only from its own best and the global best, so once both sit on a local optimum the whole
//! swarm stalls together. CLPSO lets **every dimension pick its own exemplar** from other particles' personal bests and
//! refreshes it periodically, so different dimensions of one particle can learn towards different targets.
//!
//! `v_d = w·v_d + c·r_d·(pbest_{f(d), d} − x_d)`

use crate::solver::heuristic::contract::Runner;
use crate::solver::heuristic::entropy::{Entropy, unit};
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel, to_f64};
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::clamped_value;
use crate::solver::heuristic::operator::compare_strictly;
use crate::solver::heuristic::operator::evolution::{LinearSource, ParameterSource};
use crate::solver::heuristic::operator::fitness::{
    DefaultOnMissFitnessEvaluator, FitnessEvaluator,
};
use crate::solver::heuristic::operator::initializer::{Initializer, ModelInitializer};
use crate::solver::heuristic::operator::stopping::NeverStop;
use crate::solver::heuristic::operator::{set_value_at, value_at};
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// CLPSO 粒子状态 / CLPSO particle state
#[derive(Debug, Clone)]
pub struct ClpsoParticle<V>
where
    V: HeuristicValue,
{
    /// 当前位置 / Current position
    pub individual: Individual<V, V>,
    /// 当前速度 / Current velocity
    pub velocity: Vec<f64>,
    /// 个体历史最优 / Personal historical best
    pub best: Individual<V, V>,
    /// 每个维度各自的学习对象下标 / Exemplar index per dimension
    pub exemplars: Vec<usize>,
    /// 连续未改进的代数 / Generations without improvement
    pub stagnation: u32,
}

/// CLPSO 配置 / CLPSO configuration
pub struct ClpsoConfig {
    /// 粒子数量 / Particle amount
    pub size: usize,
    /// 学习因子 c / Learning factor c
    pub learning_weight: f64,
    /// 最大速度 / Maximum velocity
    pub max_velocity: f64,
    /// 学习对象刷新间隔（按停滞代数）/ Exemplar refresh interval in stagnation generations
    pub refresh_interval: u32,
    /// 重置 pbest 的停滞阈值 / Stagnation threshold for resetting pbest
    pub stagnation_limit: u32,
}

impl ClpsoConfig {
    /// 创建 CLPSO 配置 / Create a CLPSO configuration
    pub fn new(size: usize) -> Self {
        Self {
            size,
            learning_weight: 1.494_45,
            max_velocity: 10_000.0,
            refresh_interval: 7,
            stagnation_limit: 5,
        }
    }
}

/// CLPSO 搜索骨架 / CLPSO search skeleton
pub struct ClpsoRunner<V>
where
    V: HeuristicValue,
{
    core: RunnerCore<V, V>,
    config: ClpsoConfig,
    inertia: Box<dyn ParameterSource>,
    initializer: Box<dyn Initializer<V>>,
    fitness_evaluator: Box<dyn FitnessEvaluator<V>>,
    entropy: Box<dyn Entropy>,
}

impl<V> ClpsoRunner<V>
where
    V: HeuristicValue,
{
    /// 创建 CLPSO 骨架 / Create a CLPSO skeleton
    pub fn new(config: ClpsoConfig, entropy: Box<dyn Entropy>) -> Self {
        Self {
            core: RunnerCore::new(Box::new(BestArchive::new(1)), Box::new(NeverStop)),
            config,
            inertia: Box::new(LinearSource::new(0.9, 0.4, 100)),
            initializer: Box::new(ModelInitializer),
            fitness_evaluator: Box::new(DefaultOnMissFitnessEvaluator),
            entropy,
        }
    }

    /// 设置惯性权重源 / Set the inertia weight source
    pub fn with_inertia(mut self, inertia: Box<dyn ParameterSource>) -> Self {
        self.inertia = inertia;
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

impl<V> Runner<V, V> for ClpsoRunner<V>
where
    V: HeuristicValue,
{
    fn run<M>(&mut self, model: &mut M, iteration: &mut Iteration) -> Result<Vec<Individual<V, V>>>
    where
        M: ObjectiveModel<V, V>,
    {
        let mut entropy = std::mem::replace(
            &mut self.entropy,
            Box::new(crate::solver::heuristic::entropy::SeededEntropy::default()),
        );
        let result = crate::solver::heuristic::runner::native::run_native(self, model, iteration, entropy.as_mut());
        self.entropy = entropy;
        result
    }
}

impl<V> crate::solver::heuristic::runner::native::NativeRunner<V> for ClpsoRunner<V>
where
    V: HeuristicValue,
{
    type State = Vec<ClpsoParticle<V>>;

    #[allow(unused_variables)]
    fn initialize_native<M>(&mut self, model: &M, entropy: &mut dyn Entropy) -> Result<Self::State>
    where
        M: ObjectiveModel<V, V>,
    {
        let Self {
            core: _,
            entropy: _,
            config,
            inertia,
            initializer,
            fitness_evaluator,
        } = self;
        let size = config.size;
        let learning_weight = config.learning_weight;
        let max_velocity = config.max_velocity;
        let refresh_interval = config.refresh_interval;
        let stagnation_limit = config.stagnation_limit;

        let random_index = |amount: usize, entropy: &mut dyn Entropy| -> usize {
            if amount <= 1 {
                0
            } else {
                ((unit(entropy) * amount as f64) as usize).min(amount - 1)
            }
        };

        let state: Vec<ClpsoParticle<V>> = {
            let order = model.variable_order();
            initializer
                .initialize(model, size, entropy)
                .into_iter()
                .filter_map(|solution| fitness_evaluator.evaluate(model, &solution))
                .take(size)
                .map(|individual| {
                    let exemplars = (0..order.len())
                        .map(|_| random_index(size, entropy))
                        .collect();
                    ClpsoParticle {
                        velocity: vec![0.0; order.len()],
                        best: individual.clone(),
                        individual,
                        exemplars,
                        stagnation: 0,
                    }
                })
                .collect()
        };

        Ok(state)
    }

    #[allow(unused_variables)]
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
        let Self {
            core: _,
            entropy: _,
            config,
            inertia,
            initializer,
            fitness_evaluator,
        } = self;
        let size = config.size;
        let learning_weight = config.learning_weight;
        let max_velocity = config.max_velocity;
        let refresh_interval = config.refresh_interval;
        let stagnation_limit = config.stagnation_limit;

        let random_index = |amount: usize, entropy: &mut dyn Entropy| -> usize {
            if amount <= 1 {
                0
            } else {
                ((unit(entropy) * amount as f64) as usize).min(amount - 1)
            }
        };

        if state.is_empty() {
            return Ok(state);
        }

        let weight = inertia.value(iteration, entropy);
        let order = model.variable_order();
        // 学习对象来自「上一代」的历史最优，因此先快照，避免同代内互相污染。
        // Exemplars come from the previous generation's personal bests, so snapshot them first.
        let bests: Vec<Individual<V, V>> =
            state.iter().map(|particle| particle.best.clone()).collect();

        let advanced: Vec<ClpsoParticle<V>> = state
            .iter()
            .map(|particle| {
                let dimension = order.len();
                let refresh = particle.stagnation >= refresh_interval;
                let mut velocity = vec![0.0; dimension];
                let mut solution = particle.individual.solution.clone();

                for index in 0..dimension {
                    let exemplar_index = if refresh {
                        random_index(state.len(), entropy)
                    } else {
                        particle
                            .exemplars
                            .get(index)
                            .copied()
                            .unwrap_or(0)
                            .min(state.len() - 1)
                    };
                    let current = value_at(&particle.individual.solution, &order, index)
                        .map(|value| to_f64(&value))
                        .unwrap_or(0.0);
                    let target = value_at(&bests[exemplar_index].solution, &order, index)
                        .map(|value| to_f64(&value))
                        .unwrap_or(current);
                    let mut speed = weight
                        * particle.velocity.get(index).copied().unwrap_or(0.0)
                        + learning_weight
                            * unit(entropy)
                            * (target - current);
                    speed = speed.clamp(-max_velocity, max_velocity);
                    velocity[index] = speed;
                    let (lower, upper) = model.bounds_at(index);
                    set_value_at(
                        &mut solution,
                        &order,
                        index,
                        clamped_value(current + speed, lower, upper),
                    );
                }

                let candidate = match fitness_evaluator.evaluate(model, &solution) {
                    None => {
                        return ClpsoParticle {
                            individual: particle.individual.clone(),
                            velocity,
                            best: particle.best.clone(),
                            exemplars: particle.exemplars.clone(),
                            stagnation: particle.stagnation + 1,
                        };
                    }
                    Some(candidate) => candidate,
                };

                let improved = compare_strictly(
                    model,
                    &candidate.fitness,
                    &particle.best.fitness,
                ) == std::cmp::Ordering::Less;

                if improved {
                    ClpsoParticle {
                        best: candidate.clone(),
                        individual: candidate,
                        velocity,
                        exemplars: particle.exemplars.clone(),
                        stagnation: 0,
                    }
                } else if particle.stagnation + 1 > stagnation_limit {
                    // 长期停滞：把 pbest 重置到当前位置，迫使粒子换一个学习方向。
                    // Prolonged stagnation: reset pbest to the current position.
                    ClpsoParticle {
                        exemplars: (0..dimension)
                            .map(|_| random_index(state.len(), entropy))
                            .collect(),
                        best: candidate.clone(),
                        individual: candidate,
                        velocity,
                        stagnation: 0,
                    }
                } else {
                    ClpsoParticle {
                        exemplars: if refresh {
                            (0..dimension)
                                .map(|_| random_index(state.len(), entropy))
                                .collect()
                        } else {
                            particle.exemplars.clone()
                        },
                        best: particle.best.clone(),
                        individual: candidate,
                        velocity,
                        stagnation: particle.stagnation + 1,
                    }
                }
            })
            .collect();

        Ok(advanced)
    }

    fn members_native(state: &Self::State) -> Vec<Individual<V, V>> {
        state.iter().map(|particle| particle.best.clone()).collect()
    }

    fn core_native(&mut self) -> &mut RunnerCore<V, V> {
        &mut self.core
    }
}
