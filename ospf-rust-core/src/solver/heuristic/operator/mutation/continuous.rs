//! 连续变异算子 / Continuous mutation operators
//!
//! 逐基因按变异率施加扰动。无有限上下界时所有实现都退化为「当前值 ± 1 区间内的小步」，与基线一致。
//! Perturbs genes one by one according to the mutation rate. Without finite bounds every implementation degrades to a
//! small step within ±1 of the current value, matching the baseline.

use ospf_rust_math::distribution::{Cauchy, Distribution, Gaussian, Levy};

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel, to_f64};
use crate::solver::heuristic::operator::update::{read_values, width_of, write_values};
use crate::solver::heuristic::operator::{clamped_value, should_fire, unit};

use super::Mutation;

/// 逐基因施加变异 / Apply a mutation gene by gene
fn mutate_genes<V, F>(
    model: &dyn VariableModel<V>,
    solution: &Solution<V>,
    entropy: &mut dyn Entropy,
    mutation_rate: f64,
    mut step: F,
) -> Solution<V>
where
    V: HeuristicValue,
    F: FnMut(usize, f64, Option<f64>, Option<f64>, &mut dyn Entropy) -> f64,
{
    let mut values = read_values(model, solution);
    for index in 0..values.len() {
        if !should_fire(entropy, mutation_rate) {
            continue;
        }
        let current = to_f64(&values[index]);
        let (lower, upper) = model.bounds_at(index);
        let mutated = step(index, current, lower, upper, entropy);
        values[index] = clamped_value(mutated, lower, upper);
    }
    write_values(model, values)
}

/// 高斯变异 / Gaussian mutation
#[derive(Debug, Clone, Copy)]
pub struct GaussianMutation {
    /// 标准差（占变量宽度的比例）/ Standard deviation as a ratio of the variable width
    pub sigma: f64,
}

impl GaussianMutation {
    /// 创建高斯变异 / Create a Gaussian mutation
    pub fn new(sigma: f64) -> Self {
        Self { sigma }
    }
}

impl Default for GaussianMutation {
    fn default() -> Self {
        Self::new(0.1)
    }
}

impl<V> Mutation<V> for GaussianMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        let scale = self.sigma;
        let distribution = Gaussian::standard();
        mutate_genes(model, solution, entropy, mutation_rate, |_, current, lower, upper, entropy| {
            current + distribution.sample(entropy) * scale * width_of(lower, upper)
        })
    }
}

/// 柯西变异，重尾步长有利于跳出局部最优 / Cauchy mutation whose heavy-tailed step helps escape local optima
#[derive(Debug, Clone, Copy)]
pub struct CauchyMutation {
    /// 尺度参数（占变量宽度的比例）/ Scale parameter as a ratio of the variable width
    pub gamma: f64,
}

impl CauchyMutation {
    /// 创建柯西变异 / Create a Cauchy mutation
    pub fn new(gamma: f64) -> Self {
        Self { gamma }
    }
}

impl Default for CauchyMutation {
    fn default() -> Self {
        Self::new(0.1)
    }
}

impl<V> Mutation<V> for CauchyMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        let scale = self.gamma;
        let distribution = Cauchy::new(0.0, 1.0);
        mutate_genes(model, solution, entropy, mutation_rate, |_, current, lower, upper, entropy| {
            current + distribution.sample(entropy) * scale * width_of(lower, upper)
        })
    }
}

/// 莱维变异，用重尾稳定分布提供大跨度跳变 / Lévy mutation using a heavy-tailed stable distribution for long jumps
#[derive(Debug, Clone, Copy)]
pub struct LevyMutation {
    /// 稳定性参数 / Stability parameter
    pub alpha: f64,
    /// 幅值比例 / Magnitude ratio
    pub scale: f64,
}

impl LevyMutation {
    /// 创建莱维变异 / Create a Lévy mutation
    pub fn new(alpha: f64, scale: f64) -> Self {
        Self { alpha, scale }
    }
}

impl Default for LevyMutation {
    fn default() -> Self {
        Self::new(1.5, 0.1)
    }
}

impl<V> Mutation<V> for LevyMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        let magnitude = self.scale;
        let distribution = Levy::new(self.alpha);
        mutate_genes(model, solution, entropy, mutation_rate, |_, current, lower, upper, entropy| {
            current + distribution.sample(entropy) * magnitude * width_of(lower, upper)
        })
    }
}

/// 均匀变异，在整个区间内重新采样 / Uniform mutation resampling across the whole interval
#[derive(Debug, Clone, Copy, Default)]
pub struct UniformMutation;

impl UniformMutation {
    /// 创建均匀变异 / Create a uniform mutation
    pub fn new() -> Self {
        Self
    }
}

impl<V> Mutation<V> for UniformMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        mutate_genes(model, solution, entropy, mutation_rate, |_, current, lower, upper, entropy| {
            match (lower, upper) {
                (Some(lower), Some(upper)) if lower.is_finite() && upper.is_finite() => {
                    lower + unit(entropy) * (upper - lower)
                }
                _ => current + (unit(entropy) - 0.5) * 2.0,
            }
        })
    }
}

/// 边界变异，把基因推到变量的下界或上界 / Boundary mutation pushing a gene onto the lower or upper bound
#[derive(Debug, Clone, Copy, Default)]
pub struct BoundaryMutation;

impl BoundaryMutation {
    /// 创建边界变异 / Create a boundary mutation
    pub fn new() -> Self {
        Self
    }
}

impl<V> Mutation<V> for BoundaryMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        mutate_genes(model, solution, entropy, mutation_rate, |_, current, lower, upper, entropy| {
            match (lower, upper) {
                (Some(lower), Some(upper)) if lower.is_finite() && upper.is_finite() => {
                    if unit(entropy) < 0.5 { lower } else { upper }
                }
                _ => current,
            }
        })
    }
}

/// 多项式变异（Deb），分布指数控制扰动强度 / Polynomial mutation (Deb) with a distribution index
#[derive(Debug, Clone, Copy)]
pub struct PolynomialMutation {
    /// 分布指数 η / Distribution index η
    pub distribution_index: f64,
}

impl PolynomialMutation {
    /// 创建多项式变异 / Create a polynomial mutation
    pub fn new(distribution_index: f64) -> Self {
        Self {
            distribution_index,
        }
    }
}

impl Default for PolynomialMutation {
    fn default() -> Self {
        Self::new(20.0)
    }
}

impl<V> Mutation<V> for PolynomialMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        let eta = self.distribution_index.max(0.0);
        mutate_genes(model, solution, entropy, mutation_rate, |_, current, lower, upper, entropy| {
            match (lower, upper) {
                (Some(lower), Some(upper))
                    if lower.is_finite() && upper.is_finite() && upper > lower =>
                {
                    let delta1 = (current - lower) / (upper - lower);
                    let delta2 = (upper - current) / (upper - lower);
                    let random = unit(entropy);
                    let delta_q = if random <= 0.5 {
                        (2.0 * random
                            + (1.0 - 2.0 * random) * (1.0 - delta1).powf(eta + 1.0))
                        .powf(1.0 / (eta + 1.0))
                            - 1.0
                    } else {
                        1.0 - (2.0 * (1.0 - random)
                            + 2.0 * (random - 0.5) * (1.0 - delta2).powf(eta + 1.0))
                        .powf(1.0 / (eta + 1.0))
                    };
                    current + delta_q * (upper - lower)
                }
                _ => current + (unit(entropy) - 0.5) * 2.0,
            }
        })
    }
}

/// 非一致变异（Michalewicz），扰动幅度随迭代推进而衰减 / Non-uniform mutation whose magnitude decays with the iterations
#[derive(Debug, Clone, Copy)]
pub struct NonUniformMutation {
    /// 衰减指数 b / Decay exponent b
    pub decay: f64,
    /// 迭代上限，用于计算进度 / Iteration limit used to compute progress
    pub max_iteration: u64,
}

impl NonUniformMutation {
    /// 创建非一致变异 / Create a non-uniform mutation
    pub fn new(decay: f64, max_iteration: u64) -> Self {
        Self {
            decay,
            max_iteration,
        }
    }
}

impl Default for NonUniformMutation {
    fn default() -> Self {
        Self::new(5.0, 1000)
    }
}

impl<V> Mutation<V> for NonUniformMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        let exponent = self.decay.max(1.0e-12);
        let progress = if self.max_iteration == 0 {
            0.0
        } else {
            (iteration.iteration() as f64 / self.max_iteration as f64).clamp(0.0, 1.0)
        };
        let shrink = (1.0 - progress).powf(exponent);

        mutate_genes(model, solution, entropy, mutation_rate, |_, current, lower, upper, entropy| {
            match (lower, upper) {
                (Some(lower), Some(upper)) if lower.is_finite() && upper.is_finite() => {
                    let random = unit(entropy);
                    let delta = 1.0 - random.powf(shrink);
                    if unit(entropy) < 0.5 {
                        current + delta * (upper - current)
                    } else {
                        current - delta * (current - lower)
                    }
                }
                _ => current + (unit(entropy) - 0.5) * 2.0,
            }
        })
    }
}

/// 混沌变异，先用 Logistic 映射打散均匀随机数再映射到变量区间 / Chaotic mutation scrambling the uniform sample through the logistic map
#[derive(Debug, Clone, Copy)]
pub struct ChaoticMutation {
    /// Logistic 映射迭代次数 / Number of logistic-map iterations
    pub iterations: usize,
}

impl ChaoticMutation {
    /// 创建混沌变异 / Create a chaotic mutation
    pub fn new(iterations: usize) -> Self {
        Self { iterations }
    }
}

impl Default for ChaoticMutation {
    fn default() -> Self {
        Self::new(3)
    }
}

impl<V> Mutation<V> for ChaoticMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        mutate_genes(model, solution, entropy, mutation_rate, |_, current, lower, upper, entropy| {
            let mut chaotic = unit(entropy).clamp(1.0e-6, 1.0 - 1.0e-6);
            for _ in 0..self.iterations.max(1) {
                chaotic = 4.0 * chaotic * (1.0 - chaotic);
            }
            match (lower, upper) {
                (Some(lower), Some(upper)) if lower.is_finite() && upper.is_finite() => {
                    lower + chaotic * (upper - lower)
                }
                _ => current + (chaotic - 0.5) * 2.0,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::solver::heuristic::entropy::SeededEntropy;
    use crate::solver::heuristic::iteration::Iteration;

    use super::super::test_support::{assert_dimension_and_bounds, model_with_bounds, solution};
    use super::super::Mutation;
    use super::{
        BoundaryMutation, CauchyMutation, ChaoticMutation, GaussianMutation, LevyMutation,
        NonUniformMutation, PolynomialMutation, UniformMutation,
    };

    #[test]
    fn gaussian_mutation_stays_within_bounds_and_is_reproducible() {
        let test = model_with_bounds(0.0, 10.0);
        let input = solution(&test, 5.0, 5.0);
        let iteration = Iteration::new();
        let operator = GaussianMutation::default();

        let first = operator.mutate(
            &test.model,
            &iteration,
            &input,
            1.0,
            &mut SeededEntropy::new(141),
        );
        let second = operator.mutate(
            &test.model,
            &iteration,
            &input,
            1.0,
            &mut SeededEntropy::new(141),
        );

        assert_eq!(first, second);
        assert_dimension_and_bounds(&first, 0.0, 10.0);
    }

    #[test]
    fn zero_mutation_rate_returns_the_input_for_every_continuous_operator() {
        let test = model_with_bounds(0.0, 10.0);
        let input = solution(&test, 2.0, 7.0);
        let iteration = Iteration::new();

        let operators: Vec<Box<dyn Mutation<f64>>> = vec![
            Box::new(GaussianMutation::default()),
            Box::new(CauchyMutation::default()),
            Box::new(LevyMutation::default()),
            Box::new(UniformMutation::new()),
            Box::new(BoundaryMutation::new()),
            Box::new(PolynomialMutation::default()),
            Box::new(NonUniformMutation::default()),
            Box::new(ChaoticMutation::default()),
        ];

        for operator in operators {
            let output = operator.mutate(
                &test.model,
                &iteration,
                &input,
                0.0,
                &mut SeededEntropy::new(151),
            );
            assert_eq!(output, input);
        }
    }

    #[test]
    fn boundary_mutation_pushes_onto_a_bound() {
        let test = model_with_bounds(0.0, 10.0);
        let input = solution(&test, 5.0, 5.0);

        let output = BoundaryMutation::new().mutate(
            &test.model,
            &Iteration::new(),
            &input,
            1.0,
            &mut SeededEntropy::new(161),
        );

        for value in output.values() {
            assert!(*value == 0.0 || *value == 10.0);
        }
    }
}
