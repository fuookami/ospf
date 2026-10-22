//! 粒子群搜索骨架 / Particle swarm search skeleton
//!
//! 粒子状态携带速度与个体历史最优，二者都是跨迭代信息，无法写进无状态的解更新算子。
//! The particle state carries a velocity and a personal best, both of which are cross-iteration information that cannot be
//! squeezed into a stateless solution-update operator.
//!
//! `v' = w·v + c₁·r₁·(pBest − x) + c₂·r₂·(gBest − x)`，`x' = x + v'`
//! `v' = w·v + c₁·r₁·(pBest − x) + c₂·r₂·(gBest − x)`, `x' = x + v'`

use crate::solver::heuristic::contract::Runner;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{
    HeuristicValue, ObjectiveModel, to_f64,
};
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::clamped_value;
use crate::solver::heuristic::operator::compare_strictly;
use crate::solver::heuristic::operator::evolution::{ConstantSource, ParameterSource};
use crate::solver::heuristic::operator::fitness::{DefaultOnMissFitnessEvaluator, FitnessEvaluator};
use crate::solver::heuristic::operator::initializer::{Initializer, ModelInitializer};
use crate::solver::heuristic::operator::set_value_at;
use crate::solver::heuristic::operator::stopping::NeverStop;
use crate::solver::heuristic::operator::unit;
use crate::solver::heuristic::operator::value_at;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::drive::drive;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// 粒子状态 / Particle state
#[derive(Debug, Clone)]
pub struct ParticleState<V>
where
    V: HeuristicValue,
{
    /// 当前位置 / Current position
    pub individual: Individual<V, V>,
    /// 当前速度 / Current velocity
    pub velocity: Vec<f64>,
    /// 个体历史最优 / Personal historical best
    pub best: Individual<V, V>,
}

/// 粒子群搜索骨架 / Particle swarm search skeleton
pub struct ParticleSwarmRunner<V>
where
    V: HeuristicValue,
{
    core: RunnerCore<V, V>,
    size: usize,
    inertia: Box<dyn ParameterSource>,
    personal_weight: Box<dyn ParameterSource>,
    social_weight: Box<dyn ParameterSource>,
    max_velocity: f64,
    initializer: Box<dyn Initializer<V>>,
    fitness_evaluator: Box<dyn FitnessEvaluator<V>>,
    entropy: Box<dyn Entropy>,
}

impl<V> ParticleSwarmRunner<V>
where
    V: HeuristicValue,
{
    /// 创建粒子群骨架 / Create a particle swarm skeleton
    pub fn new(size: usize, entropy: Box<dyn Entropy>) -> Self {
        Self {
            core: RunnerCore::new(Box::new(BestArchive::new(1)), Box::new(NeverStop)),
            size,
            inertia: Box::new(ConstantSource::new(0.4)),
            personal_weight: Box::new(ConstantSource::new(2.0)),
            social_weight: Box::new(ConstantSource::new(2.0)),
            max_velocity: 10_000.0,
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

    /// 设置个体学习因子源 / Set the personal learning factor source
    pub fn with_personal_weight(mut self, personal_weight: Box<dyn ParameterSource>) -> Self {
        self.personal_weight = personal_weight;
        self
    }

    /// 设置社会学习因子源 / Set the social learning factor source
    pub fn with_social_weight(mut self, social_weight: Box<dyn ParameterSource>) -> Self {
        self.social_weight = social_weight;
        self
    }

    /// 设置最大速度 / Set the maximum velocity
    pub fn with_max_velocity(mut self, max_velocity: f64) -> Self {
        self.max_velocity = max_velocity;
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

impl<V> Runner<V, V> for ParticleSwarmRunner<V>
where
    V: HeuristicValue,
{
    fn run<M>(&mut self, model: &mut M, iteration: &mut Iteration) -> Result<Vec<Individual<V, V>>>
    where
        M: ObjectiveModel<V, V>,
    {
        let Self {
            core,
            size,
            inertia,
            personal_weight,
            social_weight,
            max_velocity,
            initializer,
            fitness_evaluator,
            entropy,
        } = self;

        let state: Vec<ParticleState<V>> = initializer
            .initialize(model, *size, entropy.as_mut())
            .into_iter()
            .filter_map(|solution| fitness_evaluator.evaluate(model, &solution))
            .take(*size)
            .map(|individual| ParticleState {
                velocity: vec![0.0; individual.solution.len()],
                best: individual.clone(),
                individual,
            })
            .collect();

        drive(
            core,
            model,
            iteration,
            state,
            |model, iteration, state, entropy| {
                if state.is_empty() {
                    return Ok(state);
                }

                let mut best_index = 0;
                for index in 1..state.len() {
                    if compare_strictly(
                        model,
                        &state[index].individual.fitness,
                        &state[best_index].individual.fitness,
                    ) == std::cmp::Ordering::Less
                    {
                        best_index = index;
                    }
                }
                let global_best = state[best_index].individual.solution.clone();

                let weight = inertia.value(iteration, entropy);
                let personal = personal_weight.value(iteration, entropy);
                let social = social_weight.value(iteration, entropy);
                let velocity_limit = *max_velocity;
                let order = model.variable_order();

                let advanced = state
                    .iter()
                    .map(|particle| {
                        let mut velocity = vec![0.0; order.len()];
                        let mut solution = particle.individual.solution.clone();

                        for index in 0..order.len() {
                            let current = value_at(&particle.individual.solution, &order, index)
                                .map(|value| to_f64(&value))
                                .unwrap_or(0.0);
                            let personal_target = value_at(&particle.best.solution, &order, index)
                                .map(|value| to_f64(&value))
                                .unwrap_or(current);
                            let global_target = value_at(&global_best, &order, index)
                                .map(|value| to_f64(&value))
                                .unwrap_or(current);

                            let mut speed = weight
                                * particle.velocity.get(index).copied().unwrap_or(0.0)
                                + personal * unit(entropy) * (personal_target - current)
                                + social * unit(entropy) * (global_target - current);
                            speed = speed.clamp(-velocity_limit, velocity_limit);
                            velocity[index] = speed;

                            let (lower, upper) = model.bounds_at(index);
                            set_value_at(
                                &mut solution,
                                &order,
                                index,
                                clamped_value(current + speed, lower, upper),
                            );
                        }

                        match fitness_evaluator.evaluate(model, &solution) {
                            Some(candidate) => {
                                let improved = compare_strictly(
                                    model,
                                    &candidate.fitness,
                                    &particle.best.fitness,
                                ) == std::cmp::Ordering::Less;
                                let best = if improved {
                                    candidate.clone()
                                } else {
                                    particle.best.clone()
                                };
                                ParticleState {
                                    individual: candidate,
                                    velocity,
                                    best,
                                }
                            }
                            None => particle.clone(),
                        }
                    })
                    .collect();

                Ok(advanced)
            },
            |state| {
                state
                    .iter()
                    .map(|particle| particle.individual.clone())
                    .collect()
            },
            entropy.as_mut(),
        )
    }
}
