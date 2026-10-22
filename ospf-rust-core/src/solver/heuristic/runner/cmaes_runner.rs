//! CMA-ES 骨架 / CMA-ES skeleton
//!
//! 分布型算法：状态不是一群解，而是一个多元正态分布 `N(m, σ²C)`。每代把 `C` 分解为 `B·D²·Bᵀ`，沿主方向采样，
//! 再按优度重加权并更新步长与协方差——因此它能自动学习变量之间的相关性与各方向的尺度。
//! A distribution-based algorithm: the state is not a set of solutions but a multivariate normal distribution
//! `N(m, σ²C)`. Each generation decomposes `C` into `B·D²·Bᵀ`, samples along the principal axes, reweights by quality, and
//! updates the step size and covariance, so it learns both variable correlations and per-direction scale automatically.

use crate::solver::heuristic::contract::Runner;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel, to_f64};
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::clamped_value;
use crate::solver::heuristic::operator::compare_strictly;
use crate::solver::heuristic::operator::fitness::{
    DefaultOnMissFitnessEvaluator, FitnessEvaluator,
};
use crate::solver::heuristic::runner::CmaNumerics;
use crate::solver::heuristic::operator::stopping::NeverStop;
use crate::solver::heuristic::operator::{set_value_at, value_at};
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// CMA-ES 状态 / CMA-ES state
#[derive(Debug, Clone)]
pub struct CmaEsState<V>
where
    V: HeuristicValue,
{
    /// 分布均值 / Distribution mean
    pub mean: Vec<f64>,
    /// 全局步长 / Global step size
    pub sigma: f64,
    /// 协方差矩阵 / Covariance matrix
    pub covariance: Vec<Vec<f64>>,
    /// 步长累积路径 / Cumulative path for the step size
    pub path_sigma: Vec<f64>,
    /// 协方差累积路径 / Cumulative path for the covariance
    pub path_covariance: Vec<f64>,
    /// 已进行的代数 / Number of generations so far
    pub generation: u32,
    /// 迄今最优个体 / Best individual so far
    pub best: Option<Individual<V, V>>,
}

/// CMA-ES 骨架 / CMA-ES skeleton
pub struct CmaEsRunner<V>
where
    V: HeuristicValue,
{
    core: RunnerCore<V, V>,
    initial_sigma: f64,
    numerics: CmaNumerics,
    fitness_evaluator: Box<dyn FitnessEvaluator<V>>,
    entropy: Box<dyn Entropy>,
}

impl<V> CmaEsRunner<V>
where
    V: HeuristicValue,
{
    /// 创建 CMA-ES 骨架 / Create a CMA-ES skeleton
    pub fn new(entropy: Box<dyn Entropy>, initial_sigma: f64) -> Self {
        Self {
            core: RunnerCore::new(Box::new(BestArchive::new(1)), Box::new(NeverStop)),
            initial_sigma,
            numerics: CmaNumerics::Native,
            fitness_evaluator: Box::new(DefaultOnMissFitnessEvaluator),
            entropy,
        }
    }

    /// 显式选择数值协议 / Explicitly select the numerical contract
    pub fn with_numerics(mut self, numerics: CmaNumerics) -> Self {
        self.numerics = numerics;
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

impl Default for CmaEsRunner<f64> {
    fn default() -> Self {
        Self::new(
            Box::new(crate::solver::heuristic::entropy::SeededEntropy::default()),
            0.3,
        )
    }
}

impl<V> Runner<V, V> for CmaEsRunner<V>
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

impl<V> crate::solver::heuristic::runner::native::NativeRunner<V> for CmaEsRunner<V>
where
    V: HeuristicValue,
{
    type State = CmaEsState<V>;

    #[allow(unused_variables)]
    fn initialize_native<M>(&mut self, model: &M, entropy: &mut dyn Entropy) -> Result<Self::State>
    where
        M: ObjectiveModel<V, V>,
    {
        let Self {
            core: _,
            entropy: _,
            initial_sigma,
            numerics,
            fitness_evaluator,
        } = self;
        let initial_sigma = *initial_sigma;
        let numerics = *numerics;

        let state = {
            let order = model.variable_order();
            let solution = model.initial_solutions(1).into_iter().next();
            match solution {
                None => CmaEsState {
                    mean: Vec::new(),
                    sigma: initial_sigma,
                    covariance: Vec::new(),
                    path_sigma: Vec::new(),
                    path_covariance: Vec::new(),
                    generation: 0,
                    best: None,
                },
                Some(solution) => {
                    let mean: Vec<f64> = (0..order.len())
                        .map(|index| {
                            value_at(&solution, &order, index)
                                .map(|value| to_f64(&value))
                                .unwrap_or(0.0)
                        })
                        .collect();
                    let dimension = mean.len();
                    let covariance = (0..dimension)
                        .map(|row| {
                            (0..dimension)
                                .map(|column| if row == column { 1.0 } else { 0.0 })
                                .collect()
                        })
                        .collect();
                    CmaEsState {
                        mean,
                        sigma: initial_sigma,
                        covariance,
                        path_sigma: vec![0.0; dimension],
                        path_covariance: vec![0.0; dimension],
                        generation: 0,
                        best: fitness_evaluator.evaluate(model, &solution),
                    }
                }
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
            initial_sigma,
            numerics,
            fitness_evaluator,
        } = self;
        let initial_sigma = *initial_sigma;
        let numerics = *numerics;

        let dimension = state.mean.len();
        if dimension == 0 || !state.sigma.is_finite() || state.sigma <= 0.0 {
            return Ok(state);
        }

        // —— 策略参数（Hansen 的标准取值）—— / Strategy parameters (Hansen's standard values)
        let dimension_f = dimension as f64;
        let lambda = 4 + (3.0 * numerics.ln(dimension_f)).floor() as usize;
        let mu = lambda / 2;
        let raw_weights: Vec<f64> = (0..mu)
            .map(|index| numerics.ln(mu as f64 + 0.5) - numerics.ln(index as f64 + 1.0))
            .collect();
        let weight_total = numerics.sum(raw_weights.iter().copied());
        let weights: Vec<f64> = raw_weights
            .iter()
            .map(|weight| weight / weight_total)
            .collect();
        let squared_total = numerics.sum(weights.iter().map(|weight| weight * weight));
        let effective_mass = 1.0 / squared_total;

        let n = dimension_f;
        let cc = (4.0 + effective_mass / n) / (n + 4.0 + 2.0 * effective_mass / n);
        let cs = (effective_mass + 2.0) / (n + effective_mass + 5.0);
        let c1 = 2.0 / ((n + 1.3) * (n + 1.3) + effective_mass);
        let cmu = (1.0 - c1).min(
            2.0 * (effective_mass - 2.0 + 1.0 / effective_mass)
                / ((n + 2.0) * (n + 2.0) + effective_mass),
        );
        let damps = 1.0
            + 2.0
                * (0.0_f64)
                    .max(numerics.sqrt((effective_mass - 1.0) / (n + 1.0)) - 1.0)
            + cs;
        let chi_n =
            numerics.sqrt(n) * (1.0 - 1.0 / (4.0 * n) + 1.0 / (21.0 * n * n));

        let eigen = match numerics.eigen(&state.covariance) {
            Some(eigen) if eigen.values.len() == dimension => eigen,
            _ => return Ok(state),
        };
        let axes: Vec<f64> = eigen
            .values
            .iter()
            .map(|value| numerics.sqrt(value.max(1.0e-20)))
            .collect();

        // —— 采样—— / Sampling
        let order = model.variable_order();
        let mut samples: Vec<(Vec<f64>, Individual<V, V>)> = Vec::with_capacity(lambda);
        for _ in 0..lambda {
            let z: Vec<f64> = (0..dimension)
                .map(|_| numerics.gaussian(entropy))
                .collect();
            let mut y = vec![0.0; dimension];
            for row in 0..dimension {
                let mut total = 0.0;
                for column in 0..dimension {
                    total += eigen.vectors[column][row] * axes[column] * z[column];
                }
                y[row] = total;
            }

            let mut solution = crate::model::callback::Solution::<V>::new();
            for axis in 0..dimension {
                let (lower, upper) = model.bounds_at(axis);
                let value = clamped_value(
                    state.mean[axis] + state.sigma * y[axis],
                    lower,
                    upper,
                );
                set_value_at(&mut solution, &order, axis, value);
            }
            if let Some(individual) = fitness_evaluator.evaluate(model, &solution) {
                samples.push((y, individual));
            }
        }

        if samples.is_empty() {
            return Ok(state);
        }

        samples.sort_by(|lhs, rhs| {
            compare_strictly(model, &lhs.1.fitness, &rhs.1.fitness)
        });

        // —— 重加权并移动均值—— / Reweighting and mean shift
        let mut new_mean = vec![0.0; dimension];
        for index in 0..mu.min(samples.len()) {
            let weight = weights[index];
            for axis in 0..dimension {
                let value = value_at(&samples[index].1.solution, &order, axis)
                    .map(|value| to_f64(&value))
                    .unwrap_or(0.0);
                new_mean[axis] += weight * value;
            }
        }

        let mean_shift: Vec<f64> = (0..dimension)
            .map(|axis| (new_mean[axis] - state.mean[axis]) / state.sigma)
            .collect();

        // —— 步长路径与协方差路径—— / Step-size path and covariance path
        let sigma_coefficient =
            numerics.sqrt(cs * (2.0 - cs) * effective_mass);

        // B·y_w：把均值位移投到主方向后再变换回来。
        // B·y_w: project the mean shift onto the principal axes and transform back.
        let projected: Vec<f64> = (0..dimension)
            .map(|column| {
                numerics.sum((0..dimension)
                    .map(|row| eigen.vectors[column][row] * mean_shift[row])
                )
            })
            .collect();
        let normalized_shift: Vec<f64> = (0..dimension)
            .map(|row| {
                numerics.sum((0..dimension)
                    .map(|column| {
                        let axis = axes[column];
                        if axis > 1.0e-20 {
                            eigen.vectors[column][row] * projected[column] / axis
                        } else {
                            0.0
                        }
                    })
                )
            })
            .collect();

        let new_path_sigma: Vec<f64> = (0..dimension)
            .map(|axis| {
                (1.0 - cs) * state.path_sigma[axis]
                    + sigma_coefficient * normalized_shift[axis]
            })
            .collect();

        let path_norm: f64 =
            numerics.sqrt(numerics.sum(new_path_sigma.iter().map(|value| value * value)));

        let generation = state.generation + 1;
        let h_sigma = path_norm
            / numerics.sqrt(1.0 - numerics.pow(1.0 - cs, 2.0 * generation as f64))
            < (1.4 + 2.0 / (n + 1.0)) * chi_n;

        let cov_coefficient = numerics.sqrt(cc * (2.0 - cc) * effective_mass)
            * if h_sigma { 1.0 } else { 0.0 };
        let new_path_covariance: Vec<f64> = (0..dimension)
            .map(|axis| {
                (1.0 - cc) * state.path_covariance[axis]
                    + cov_coefficient * mean_shift[axis]
            })
            .collect();

        // —— 协方差更新：rank-one + rank-μ—— / Covariance update: rank-one + rank-μ
        let mut new_covariance = vec![vec![0.0; dimension]; dimension];
        for row in 0..dimension {
            for column in 0..dimension {
                let mut value = (1.0 - c1 - cmu) * state.covariance[row][column];

                let rank_one =
                    new_path_covariance[row] * new_path_covariance[column];
                let correction = (1.0 - if h_sigma { 1.0 } else { 0.0 })
                    * cc
                    * (2.0 - cc)
                    * state.covariance[row][column];
                value += c1 * (rank_one + correction);

                let mut rank_mu = 0.0;
                for index in 0..mu.min(samples.len()) {
                    rank_mu += weights[index]
                        * samples[index].0[row]
                        * samples[index].0[column];
                }
                value += cmu * rank_mu;

                new_covariance[row][column] = value;
            }
        }

        let new_sigma = (state.sigma
            * numerics.exp((cs / damps) * (path_norm / chi_n - 1.0)))
            .clamp(1.0e-12, 1.0e12);

        let best_candidate = samples.first().map(|(_, individual)| individual.clone());
        let best = match (best_candidate, &state.best) {
            (None, best) => best.clone(),
            (Some(candidate), None) => Some(candidate),
            (Some(candidate), Some(current)) => {
                if compare_strictly(model, &candidate.fitness, &current.fitness)
                    == std::cmp::Ordering::Less
                {
                    Some(candidate)
                } else {
                    Some(current.clone())
                }
            }
        };

        Ok(CmaEsState {
            mean: new_mean,
            sigma: new_sigma,
            covariance: new_covariance,
            path_sigma: new_path_sigma,
            path_covariance: new_path_covariance,
            generation,
            best,
        })
    }

    fn members_native(state: &Self::State) -> Vec<Individual<V, V>> {
        state.best.iter().cloned().collect()
    }

    fn core_native(&mut self) -> &mut RunnerCore<V, V> {
        &mut self.core
    }
}
