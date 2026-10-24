//! SHADE / L-SHADE 骨架 / SHADE and L-SHADE skeleton
//!
//! SHADE 的自适应性要求跨迭代累积成功历史，无状态的解更新算子做不到，因此把记忆库建模为状态的一部分，
//! 循环本身仍然复用同一套结构。
//! SHADE's adaptivity requires accumulating success history across iterations, which a stateless solution-update operator
//! cannot do, so the memory bank is modelled as part of the state while the loop keeps the same structure.

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
use crate::solver::heuristic::operator::compare_individuals;
use crate::solver::heuristic::operator::evolution::{ConstantSource, ParameterSource};
use crate::solver::heuristic::operator::fitness::{
    DefaultOnMissFitnessEvaluator, FitnessEvaluator,
};
use crate::solver::heuristic::operator::stopping::NeverStop;
use crate::solver::heuristic::operator::{set_value_at, value_at};
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// SHADE 采样数值协议，原生模式保留历史轨迹 / SHADE sampling numerical contract; native mode preserves historical trajectories
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ShadeNumerics {
    /// 平台原生正切 / Platform-native tangent
    #[default]
    Native,
    /// fdlibm 正切与固定运算顺序；协议 shade-fdlibm-v1 / fdlibm tangent and fixed operation order; protocol shade-fdlibm-v1
    FdlibmV1,
}

impl ShadeNumerics {
    fn tangent(self, angle: f64) -> f64 {
        match self { Self::Native => angle.tan(), Self::FdlibmV1 => libm::tan(angle) }
    }
}

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
    /// `F` 使用柯西采样；`CR` 与 Kotlin 协议一致，使用三次均匀采样的近似；两者裁剪到 `[0, 1]`。
    /// `F` uses a Cauchy sample; `CR` uses the Kotlin protocol's three-uniform approximation; both are clamped to `[0, 1]`.
    pub fn sample(&self, entropy: &mut dyn Entropy) -> ShadeParameters {
        self.sample_with_numerics(entropy, ShadeNumerics::Native)
    }

    /// 按显式数值协议采样，不改变随机数消费顺序 / Sample using an explicit numerical contract without changing random consumption
    pub fn sample_with_numerics(
        &self,
        entropy: &mut dyn Entropy,
        numerics: ShadeNumerics,
    ) -> ShadeParameters {
        let index = ((unit(entropy) * self.memory_f.len() as f64) as usize)
            .min(self.memory_f.len() - 1);
        let mut differential_weight = self.memory_f[index]
            + 0.1 * numerics.tangent(std::f64::consts::PI * (unit(entropy) - 0.5));
        if differential_weight <= 0.0 {
            differential_weight = self.memory_f[index];
        }
        let differential_weight = differential_weight.clamp(0.0, 1.0);

        let cross_rate = (self.memory_cr[index]
            + 0.1 * (unit(entropy) + unit(entropy) + unit(entropy) - 1.5) * 2.0).clamp(0.0, 1.0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::heuristic::entropy::FnEntropy;

    #[test]
    fn parameter_sampling_matches_kotlin_values_and_consumes_five_draws() {
        let samples = [0.7, 0.75, 0.1, 0.2, 0.9];
        let mut consumed = 0;
        let mut entropy = FnEntropy::new(|| {
            let value = samples[consumed];
            consumed += 1;
            value
        });
        let actual = ShadeMemory::new(5).sample(&mut entropy);
        assert_eq!(actual.memory_index, 3);
        assert!((actual.differential_weight - 0.6).abs() < 1e-15);
        assert_eq!(actual.cross_rate, 0.5 + 0.1 * (0.1 + 0.2 + 0.9 - 1.5) * 2.0);
        assert_eq!(consumed, 5);
    }

    #[test]
    fn fdlibm_tangent_matches_shared_binary64_contract_vectors() {
        for (input, expected) in [
            (0xc02ebe05be113456, 0x3fd6692cd48a42aa),
            (0xc029e9f8300b6a04, 0xbfda5a3cdd51edd9),
            (0xc019534956763f6c, 0xbfa8abe2c5ee870e),
            (0x4022a3ea01341b64, 0xbfbae2617d4619be),
            (0xc030100029042b02, 0xbfd7b10bf21a884f),
            (0xc03171f3cff42048, 0x4017d22d731c64df),
            (0x4033183d13007fea, 0x3fd002909c5eab58),
            (0xc0238e9444814bcc, 0xbfd7a15cb5c047f0),
            (0x0000000000000000, 0x0000000000000000),
            (0x8000000000000000, 0x8000000000000000),
        ] {
            assert_eq!(ShadeNumerics::FdlibmV1.tangent(f64::from_bits(input)).to_bits(), expected);
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
    /// 参数采样数值协议 / Parameter-sampling numerical contract
    pub numerics: ShadeNumerics,
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
            numerics: ShadeNumerics::Native,
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
    if model.stochastic_constraints() {
        return crate::solver::heuristic::operator::rank_individuals(model, individuals).into_iter().take(amount).collect();
    }
    let mut remaining = individuals.to_vec();
    let mut kept = Vec::with_capacity(amount.min(remaining.len()));
    while kept.len() < amount && !remaining.is_empty() {
        let mut best_index = 0;
        for index in 1..remaining.len() {
            if compare_individuals(model, &remaining[index], &remaining[best_index])
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
        let mut entropy = std::mem::replace(
            &mut self.entropy,
            Box::new(crate::solver::heuristic::entropy::SeededEntropy::default()),
        );
        let result = crate::solver::heuristic::runner::native::run_native(self, model, iteration, entropy.as_mut());
        self.entropy = entropy;
        result
    }
}

impl<V> crate::solver::heuristic::runner::native::NativeRunner<V> for ShadeRunner<V>
where
    V: HeuristicValue,
{
    type State = ShadeState<V>;

    #[allow(unused_variables)]
    fn initialize_native<M>(&mut self, model: &M, entropy: &mut dyn Entropy) -> Result<Self::State>
    where
        M: ObjectiveModel<V, V>,
    {
        let Self {
            core: _,
            entropy: _,
            config,
            fitness_evaluator,
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
            fitness_evaluator,
        } = self;
        let size = config.size;
        let memory_size = config.memory_size;
        let p_best_rate = config.p_best_rate;
        let minimum_size = config.minimum_size;
        let size_ratio = config.size_ratio.as_mut();

        if state.population.is_empty() {
            return Ok(state);
        }

        let order = model.variable_order();
        let dimension = order.len();
        let sorted = crate::solver::heuristic::operator::rank_individuals(model, &state.population);
        let p_best_amount = ((sorted.len() as f64 * p_best_rate) as usize).max(1);
        let p_best_pool = &sorted[..p_best_amount.min(sorted.len())];

        let mut next_population = Vec::with_capacity(state.population.len());
        let mut next_archive = state.archive.clone();
        let mut successful_f = Vec::new();
        let mut successful_cr = Vec::new();

        for index in 0..state.population.len() {
            let target = &state.population[index];
            let parameters = state.memory.sample_with_numerics(entropy, config.numerics);
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
            for (dim, id) in order.iter().enumerate() {
                let take_mutant =
                    unit(entropy) < parameters.cross_rate || dim == forced;
                let value = if take_mutant {
                    mutant.get(id).copied().unwrap_or_else(|| {
                        from_f64(0.0)
                    })
                } else {
                    target
                        .solution
                        .get(id)
                        .copied()
                        .unwrap_or_else(|| from_f64(0.0))
                };
                trial.insert(*id, value);
            }

            match fitness_evaluator.evaluate(model, &trial) {
                None => next_population.push(target.clone()),
                Some(candidate) => {
                    if compare_individuals(model, &candidate, target) != std::cmp::Ordering::Greater
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
    }

    fn members_native(state: &Self::State) -> Vec<Individual<V, V>> {
        state.population.clone()
    }

    fn core_native(&mut self) -> &mut RunnerCore<V, V> {
        &mut self.core
    }
}
