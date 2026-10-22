//! SHADE / L-SHADE 骨架 / SHADE and L-SHADE skeleton
//!
//! SHADE 的自适应性要求跨迭代累积成功历史，无状态的解更新算子做不到，因此把记忆库建模为状态的一部分，
//! 循环本身仍然复用同一套结构。
//! SHADE's adaptivity requires accumulating success history across iterations, which a stateless solution-update operator
//! cannot do, so the memory bank is modelled as part of the state while the loop keeps the same structure.

use ospf_rust_math::distribution::{Cauchy, Distribution, Gaussian};

use crate::model::callback::Solution;

use crate::solver::heuristic::contract::Runner;
use crate::solver::heuristic::entropy::{Entropy, unit};
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{
    HeuristicValue, ObjectiveModel, from_f64, to_f64,
};
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::clamped_value;
use crate::solver::heuristic::operator::compare_strictly;
use crate::solver::heuristic::operator::evolution::{ConstantSource, ParameterSource};
use crate::solver::heuristic::operator::fitness::{
    DefaultOnMissFitnessEvaluator, FitnessEvaluator,
};
use crate::solver::heuristic::operator::stopping::NeverStop;
use crate::solver::heuristic::operator::{set_value_at, value_at};
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::drive::drive;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// 一次参数采样结果 / One sampled parameter pair
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadeParameters {
    /// 采样自记忆库的槽位 / Slot the sample came from
    pub memory_index: usize,
    /// 缩放因子 F / Scaling factor F
    pub differential_weight: f64,
    /// 交叉率 CR / Crossover rate CR
    pub cross_rate: f64,
}

/// 成功历史自适应记忆 / Success-history adaptive memory
///
/// 把每次成功试验的 `F`、`CR` 记下来，用它们的均值反过来调整采样分布的中心：`F` 用莱默均值（偏向较大的成功值），
/// `CR` 用算术均值。
/// Records the `F` and `CR` of every successful trial and re-centres the sampling distributions by their means: `F` uses the
/// Lehmer mean (biased towards larger successful values) and `CR` the arithmetic mean.
#[derive(Debug, Clone, PartialEq)]
pub struct ShadeMemory {
    /// 缩放因子的记忆库 / Memory bank of the scaling factor
    pub memory_f: Vec<f64>,
    /// 交叉率的记忆库 / Memory bank of the crossover rate
    pub memory_cr: Vec<f64>,
    /// 下一个写入槽位 / Next slot to write
    pub cursor: usize,
}

impl ShadeMemory {
    /// 默认记忆槽数量 / Default number of memory slots
    pub const DEFAULT_SIZE: usize = 5;

    /// 创建记忆库 / Create a memory bank
    pub fn new(size: usize) -> Self {
        Self {
            memory_f: vec![0.5; size.max(1)],
            memory_cr: vec![0.5; size.max(1)],
            cursor: 0,
        }
    }

    /// 采样一组 `(F, CR)` / Sample one `(F, CR)` pair
    ///
    /// `F` 取自柯西分布（重尾，偶尔产生大步长），`CR` 取自正态分布；两者都被裁剪到 `[0, 1]`。
    /// `F` comes from a Cauchy distribution (heavy tailed, occasionally producing a long step) and `CR` from a normal
    /// distribution; both are clamped to `[0, 1]`.
    pub fn sample(&self, entropy: &mut dyn Entropy) -> ShadeParameters {
        let index = ((unit(entropy) * self.memory_f.len() as f64) as usize)
            .min(self.memory_f.len() - 1);
        let mut differential_weight =
            self.memory_f[index] + 0.1 * Cauchy::new(0.0, 1.0).sample(entropy);
        if differential_weight <= 0.0 {
            differential_weight = self.memory_f[index];
        }
        let differential_weight = differential_weight.clamp(0.0, 1.0);

        let cross_rate = (self.memory_cr[index] + 0.1 * Gaussian::new(0.0, 1.0).sample(entropy))
            .clamp(0.0, 1.0);
        ShadeParameters {
            memory_index: index,
            differential_weight,
            cross_rate,
        }
    }

    /// 用本代的成功参数更新记忆 / Update the memory with this generation's successful parameters
    pub fn update(&self, successful_f: &[f64], successful_cr: &[f64]) -> Self {
        if successful_f.is_empty() || successful_cr.is_empty() {
            return self.clone();
        }

        let weighted_sum: f64 = successful_f.iter().map(|value| value * value).sum();
        let total_sum: f64 = successful_f.iter().sum();
        let cross_rate_sum: f64 = successful_cr.iter().sum();

        let lehmer_mean = if total_sum > 0.0 {
            (weighted_sum / total_sum).clamp(0.0, 1.0)
        } else {
            self.memory_f[self.cursor]
        };
        let arithmetic_mean_cr =
            (cross_rate_sum / successful_cr.len() as f64).clamp(0.0, 1.0);

        let mut memory_f = self.memory_f.clone();
        let mut memory_cr = self.memory_cr.clone();
        memory_f[self.cursor] = lehmer_mean;
        memory_cr[self.cursor] = arithmetic_mean_cr;
        Self {
            memory_f,
            memory_cr,
            cursor: (self.cursor + 1) % self.memory_f.len(),
        }
    }
}

/// SHADE 状态 / SHADE state
#[derive(Debug, Clone)]
pub struct ShadeState<V>
where
    V: HeuristicValue,
{
    /// 当前种群 / Current population
    pub population: Vec<Individual<V, V>>,
    /// 自适应记忆 / Adaptive memory
    pub memory: ShadeMemory,
    /// 被淘汰父代的外部档案 / External archive of replaced parents
    pub archive: Vec<Solution<V>>,
}

/// SHADE 配置 / SHADE configuration
pub struct ShadeConfig {
    /// 初始种群规模 / Initial population size
    pub size: usize,
    /// 记忆槽数量 / Number of memory slots
    pub memory_size: usize,
    /// p-best 比例 / p-best fraction
    pub p_best_rate: f64,
    /// 种群规模的缩减源 / Shrinking source of the population size
    pub size_ratio: Box<dyn ParameterSource>,
    /// 缩减后的种群规模下界 / Lower bound of the shrunk population size
    pub minimum_size: usize,
}

impl ShadeConfig {
    /// 创建 SHADE 配置 / Create a SHADE configuration
    pub fn new(size: usize) -> Self {
        Self {
            size,
            memory_size: ShadeMemory::DEFAULT_SIZE,
            p_best_rate: 0.2,
            size_ratio: Box::new(ConstantSource::new(1.0)),
            minimum_size: 4,
        }
    }
}

/// SHADE 搜索骨架 / SHADE search skeleton
pub struct ShadeRunner<V>
where
    V: HeuristicValue,
{
    core: RunnerCore<V, V>,
    config: ShadeConfig,
    fitness_evaluator: Box<dyn FitnessEvaluator<V>>,
    entropy: Box<dyn Entropy>,
}

impl<V> ShadeRunner<V>
where
    V: HeuristicValue,
{
    /// 创建 SHADE 骨架 / Create a SHADE skeleton
    pub fn new(config: ShadeConfig, entropy: Box<dyn Entropy>) -> Self {
        Self {
            core: RunnerCore::new(Box::new(BestArchive::new(1)), Box::new(NeverStop)),
            config,
            fitness_evaluator: Box::new(DefaultOnMissFitnessEvaluator),
            entropy,
        }
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

fn keep_best<V>(
    model: &dyn ObjectiveModel<V, V>,
    individuals: &[Individual<V, V>],
    amount: usize,
) -> Vec<Individual<V, V>>
where
    V: HeuristicValue,
{
    let mut remaining = individuals.to_vec();
    let mut kept = Vec::with_capacity(amount.min(remaining.len()));
    while kept.len() < amount && !remaining.is_empty() {
        let mut best_index = 0;
        for index in 1..remaining.len() {
            if compare_strictly(model, &remaining[index].fitness, &remaining[best_index].fitness)
                == std::cmp::Ordering::Less
            {
                best_index = index;
            }
        }
        kept.push(remaining.remove(best_index));
    }
    kept
}

impl<V> Runner<V, V> for ShadeRunner<V>
where
    V: HeuristicValue,
{
    fn run<M>(&mut self, model: &mut M, iteration: &mut Iteration) -> Result<Vec<Individual<V, V>>>
    where
        M: ObjectiveModel<V, V>,
    {
        let Self {
            core,
            config,
            fitness_evaluator,
            entropy,
        } = self;
        let size = config.size;
        let memory_size = config.memory_size;
        let p_best_rate = config.p_best_rate;
        let minimum_size = config.minimum_size;
        let size_ratio = config.size_ratio.as_mut();

        let state = {
            let population: Vec<Individual<V, V>> = model
                .initial_solutions(size)
                .into_iter()
                .filter_map(|solution| fitness_evaluator.evaluate(model, &solution))
                .take(size)
                .collect();
            ShadeState {
                population,
                memory: ShadeMemory::new(memory_size),
                archive: Vec::new(),
            }
        };

        drive(
            core,
            model,
            iteration,
            state,
            |model, iteration, state, entropy| {
                if state.population.is_empty() {
                    return Ok(state);
                }

                let order = model.variable_order();
                let dimension = order.len();
                let mut sorted = state.population.clone();
                sorted.sort_by(|lhs, rhs| compare_strictly(model, &lhs.fitness, &rhs.fitness));
                let p_best_amount = ((sorted.len() as f64 * p_best_rate) as usize).max(1);
                let p_best_pool = &sorted[..p_best_amount.min(sorted.len())];

                let mut next_population = Vec::with_capacity(state.population.len());
                let mut next_archive = state.archive.clone();
                let mut successful_f = Vec::new();
                let mut successful_cr = Vec::new();

                for index in 0..state.population.len() {
                    let target = &state.population[index];
                    let parameters = state.memory.sample(entropy);
                    let p_best_index = ((unit(entropy) * p_best_pool.len() as f64) as usize)
                        .min(p_best_pool.len() - 1);
                    let p_best = &p_best_pool[p_best_index];

                    // r1 取当前种群，r2 取「种群 + 外部档案」的并集——外部档案保留了已被淘汰的父代位置，
                    // 让差分向量在更长的历史里取点。
                    // r1 comes from the current population and r2 from "population + external archive".
                    let mut first = ((unit(entropy) * state.population.len() as f64) as usize)
                        .min(state.population.len() - 1);
                    if first == index {
                        first = (index + 1) % state.population.len();
                    }
                    let second_pool_size = state.population.len() + next_archive.len();
                    let second = ((unit(entropy) * second_pool_size as f64) as usize)
                        .min(second_pool_size.saturating_sub(1));

                    let mut mutant = Solution::<V>::new();
                    for dim in 0..dimension {
                        let base = value_at(&target.solution, &order, dim)
                            .map(|value| to_f64(&value))
                            .unwrap_or(0.0);
                        let toward_best = value_at(&p_best.solution, &order, dim)
                            .map(|value| to_f64(&value))
                            .unwrap_or(base)
                            - base;
                        let second_value = if second < state.population.len() {
                            value_at(&state.population[second].solution, &order, dim)
                                .map(|value| to_f64(&value))
                                .unwrap_or(0.0)
                        } else {
                            value_at(&next_archive[second - state.population.len()], &order, dim)
                                .map(|value| to_f64(&value))
                                .unwrap_or(0.0)
                        };
                        let difference = value_at(
                            &state.population[first].solution,
                            &order,
                            dim,
                        )
                        .map(|value| to_f64(&value))
                        .unwrap_or(0.0)
                            - second_value;
                        let value = base
                            + parameters.differential_weight * toward_best
                            + parameters.differential_weight * difference;
                        let (lower, upper) = model.bounds_at(dim);
                        set_value_at(
                            &mut mutant,
                            &order,
                            dim,
                            clamped_value(value, lower, upper),
                        );
                    }

                    // 二项式交叉：至少有一个维度取自变异向量。
                    // Binomial crossover: at least one dimension must come from the mutant.
                    let mut trial = Solution::<V>::new();
                    let forced = ((unit(entropy) * dimension as f64) as usize)
                        .min(dimension.saturating_sub(1));
                    for dim in 0..dimension {
                        let take_mutant =
                            unit(entropy) < parameters.cross_rate || dim == forced;
                        let value = if take_mutant {
                            mutant.get(&order[dim]).copied().unwrap_or_else(|| {
                                from_f64(0.0)
                            })
                        } else {
                            target
                                .solution
                                .get(&order[dim])
                                .copied()
                                .unwrap_or_else(|| from_f64(0.0))
                        };
                        trial.insert(order[dim], value);
                    }

                    match fitness_evaluator.evaluate(model, &trial) {
                        None => next_population.push(target.clone()),
                        Some(candidate) => {
                            if compare_strictly(
                                model,
                                &candidate.fitness,
                                &target.fitness,
                            ) != std::cmp::Ordering::Greater
                            {
                                next_population.push(candidate);
                                next_archive.push(target.solution.clone());
                                successful_f.push(parameters.differential_weight);
                                successful_cr.push(parameters.cross_rate);
                            } else {
                                next_population.push(target.clone());
                            }
                        }
                    }
                }

                // L-SHADE：种群规模随迭代线性缩减，缩减时保留最优个体。
                // L-SHADE: the population shrinks linearly with iterations, keeping the best individuals.
                let ratio = size_ratio.value(iteration, entropy);
                let target_size = ((size as f64 * ratio) as usize)
                    .clamp(minimum_size.min(size), size);
                let survived = if next_population.len() > target_size {
                    keep_best(model, &next_population, target_size)
                } else {
                    next_population
                };

                while next_archive.len() > target_size {
                    next_archive.remove(0);
                }

                Ok(ShadeState {
                    population: survived,
                    memory: state.memory.update(&successful_f, &successful_cr),
                    archive: next_archive,
                })
            },
            |state| state.population.clone(),
            entropy.as_mut(),
        )
    }
}
