//! 邻域算子 / Neighbourhood operators
//!
//! 为单解搜索生成邻域候选，模拟退火、爬山与邻域搜索共用同一套算子。
//! Generates neighbourhood candidates for single-solution search; simulated annealing, hill climbing, and neighbourhood
//! search share the same operators.

use ospf_rust_math::distribution::{Distribution, Gaussian, Levy};

use crate::model::callback::Solution;

use super::super::entropy::Entropy;
use super::super::iteration::Iteration;
use super::super::model::{HeuristicValue, VariableModel, from_f64, to_f64};
use super::unit;

/// 邻域算子契约 / Neighbourhood contract
pub trait Neighborhood<V>
where
    V: HeuristicValue,
{
    /// 生成一个邻域解 / Generate one neighbour solution
    fn neighbor(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        solution: &Solution<V>,
        entropy: &mut dyn Entropy,
    ) -> Solution<V>;
}

/// 在随机维度上施加有界扰动 / Apply a bounded perturbation on random dimensions
fn perturb<V>(
    model: &dyn VariableModel<V>,
    solution: &Solution<V>,
    entropy: &mut dyn Entropy,
    amount: usize,
    mut step: impl FnMut(f64, &mut dyn Entropy) -> f64,
) -> Solution<V>
where
    V: HeuristicValue,
{
    let mut result = solution.clone();
    let order = model.variable_order();
    if order.is_empty() {
        return result;
    }

    let dimension = order.len();
    let target = amount.min(dimension);
    let mut touched = Vec::with_capacity(target);
    let mut attempts = 0;
    while touched.len() < target && attempts < dimension * 4 {
        let candidate = ((unit(entropy) * dimension as f64) as usize).min(dimension - 1);
        if !touched.contains(&candidate) {
            touched.push(candidate);
        }
        attempts += 1;
    }

    for index in touched {
        let (lower_bound, upper_bound) = model.bounds_at(index);
        let lower = lower_bound.unwrap_or(f64::NEG_INFINITY);
        let upper = upper_bound.unwrap_or(f64::INFINITY);
        // 无有限上下界时以 1.0 作为参考宽度，避免宽度为无穷导致步长失效。
        // Fall back to a reference width of 1.0 when the bounds are not finite.
        let width = if lower.is_finite() && upper.is_finite() {
            upper - lower
        } else {
            1.0
        };
        let current = result
            .get(&order[index])
            .map(|value| to_f64(value))
            .unwrap_or(0.0);
        let mut sampled = step(width, entropy);
        if sampled.is_nan() {
            sampled = 0.0;
        }
        let candidate = (current + sampled).clamp(lower, upper);
        result.insert(order[index], from_f64(candidate));
    }

    result
}

/// 均匀随机扰动 / Uniform random perturbation
#[derive(Debug, Clone, Copy)]
pub struct RandomPerturbation {
    /// 扰动维度数量 / Number of perturbed dimensions
    pub amount: usize,
}

impl RandomPerturbation {
    /// 创建均匀扰动 / Create a uniform perturbation
    pub fn new(amount: usize) -> Self {
        Self { amount }
    }
}

impl Default for RandomPerturbation {
    fn default() -> Self {
        Self::new(1)
    }
}

impl<V> Neighborhood<V> for RandomPerturbation
where
    V: HeuristicValue,
{
    fn neighbor(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        perturb(model, solution, entropy, self.amount, |width, entropy| {
            (unit(entropy) - 0.5) * width
        })
    }
}

/// 高斯扰动 / Gaussian perturbation
#[derive(Debug, Clone, Copy)]
pub struct GaussianPerturbation {
    /// 扰动维度数量 / Number of perturbed dimensions
    pub amount: usize,
    /// 标准差（相对变量宽度的比例）/ Standard deviation as a ratio of the variable width
    pub standard_deviation: f64,
}

impl GaussianPerturbation {
    /// 创建高斯扰动 / Create a Gaussian perturbation
    pub fn new(amount: usize, standard_deviation: f64) -> Self {
        Self {
            amount,
            standard_deviation,
        }
    }
}

impl Default for GaussianPerturbation {
    fn default() -> Self {
        Self::new(1, 0.1)
    }
}

impl<V> Neighborhood<V> for GaussianPerturbation
where
    V: HeuristicValue,
{
    fn neighbor(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        let distribution = Gaussian::new(0.0, self.standard_deviation);
        perturb(model, solution, entropy, self.amount, |width, entropy| {
            distribution.sample(entropy) * width
        })
    }
}

/// 莱维扰动 / Lévy perturbation
#[derive(Debug, Clone, Copy)]
pub struct LevyPerturbation {
    /// 扰动维度数量 / Number of perturbed dimensions
    pub amount: usize,
    /// 稳定性参数 / Stability parameter
    pub alpha: f64,
}

impl LevyPerturbation {
    /// 创建莱维扰动 / Create a Lévy perturbation
    pub fn new(amount: usize, alpha: f64) -> Self {
        Self { amount, alpha }
    }
}

impl Default for LevyPerturbation {
    fn default() -> Self {
        Self::new(1, 1.5)
    }
}

impl<V> Neighborhood<V> for LevyPerturbation
where
    V: HeuristicValue,
{
    fn neighbor(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        let distribution = Levy::new(self.alpha);
        perturb(model, solution, entropy, self.amount, |width, entropy| {
            distribution.sample(entropy) * width * 0.1
        })
    }
}
