//! 生物启发与算术位置更新 / Bio-inspired and arithmetic position updates
//!
//! 鲸鱼（WOA）、哈里斯鹰（HHO）与算术优化（AOA）。三者都以父代组首个个体为最优，并且都按变量边界夹紧。
//! Whale (WOA), Harris hawks (HHO), and arithmetic optimization (AOA). All three treat the first parent as the best
//! solution and clamp every moved position against the variable bounds.

use std::f64::consts::PI;

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel, to_f64};
use crate::solver::heuristic::operator::evolution::{LinearSource, ParameterSource};
use crate::solver::heuristic::operator::unit;

use super::{ParameterSlot, SolutionUpdate, clamp_at, origin_of, read_values, width_of, write_values};

/// 鲸鱼优化算法（WOA）。 / Whale optimization algorithm (WOA).
///
/// 三条互斥分支由 `p` 与 `|A|` 共同选择： / Three mutually exclusive branches are chosen by `p` and `|A|`:
///
/// - 包围猎物（`p < 0.5` 且 `|A| < 1`）：X' = X_best − A·|C·X_best − X|
///   encircling prey: X' = X_best − A·|C·X_best − X|
/// - 随机搜索（`p < 0.5` 且 `|A| ≥ 1`）：X' = X_rand − A·|C·X_rand − X|
///   random search: X' = X_rand − A·|C·X_rand − X|
/// - 螺旋气泡网（`p ≥ 0.5`）：X' = |X_best − X|·e^{b·l}·cos(2πl) + X_best
///   bubble-net spiral: X' = |X_best − X|·e^{b·l}·cos(2πl) + X_best
pub struct WhaleOptimizationUpdate {
    /// 控制参数 a 的演化源 / Evolution source of the control parameter a
    decay: ParameterSlot,
    /// 螺旋形状参数 b / Spiral shape parameter b
    pub spiral_rate: f64,
}

impl WhaleOptimizationUpdate {
    /// 创建鲸鱼更新 / Create a whale optimization update
    pub fn new(decay: Box<dyn ParameterSource>, spiral_rate: f64) -> Self {
        Self {
            decay: ParameterSlot::new(decay),
            spiral_rate,
        }
    }
}

impl Default for WhaleOptimizationUpdate {
    fn default() -> Self {
        Self::new(Box::new(LinearSource::new(2.0, 0.0, 100)), 1.0)
    }
}

impl<V> SolutionUpdate<V> for WhaleOptimizationUpdate
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
        if parents.is_empty() {
            return Vec::new();
        }

        let ordered: Vec<Vec<V>> = parents
            .iter()
            .map(|parent| read_values(model, parent))
            .collect();
        let best = &ordered[0];
        let size = ordered.len();
        let a = self.decay.value(iteration, entropy);

        ordered
            .iter()
            .map(|current| {
                let p = unit(entropy);
                let spiral = p >= 0.5;
                // A 与 C 对每个个体独立抽样，这就是原文中「每头鲸鱼各自更新」的含义。
                // A and C are sampled per individual, which is what "each whale updates independently" means.
                let coefficient_a = 2.0 * a * unit(entropy) - a;
                let coefficient_c = 2.0 * unit(entropy);
                let reference = if coefficient_a.abs() >= 1.0 {
                    let donor = ((unit(entropy) * size as f64) as usize).min(size - 1);
                    &ordered[donor]
                } else {
                    best
                };
                let spiral_length = (a * 2.0 - 1.0) * unit(entropy);

                let mut child = Vec::with_capacity(current.len());
                for dimension in 0..current.len() {
                    let position = to_f64(&current[dimension]);
                    let target = to_f64(&reference[dimension]);
                    let value = if spiral {
                        let best_position = to_f64(&best[dimension]);
                        let distance = (best_position - position).abs();
                        distance
                            * (self.spiral_rate * spiral_length).exp()
                            * (2.0 * PI * spiral_length).cos()
                            + best_position
                    } else {
                        target - coefficient_a * (coefficient_c * target - position).abs()
                    };
                    child.push(clamp_at(model, dimension, value));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 哈里斯鹰优化（HHO）。 / Harris hawks optimization (HHO).
///
/// 逃逸能量 `E = 2·(1 − t/T)·(2r − 1)` 从约 2 衰减到 0，`|E| ≥ 1` 时全局搜索，否则转入软/硬围攻。
/// The escaping energy `E` decays from about 2 to 0; `|E| ≥ 1` triggers global search, otherwise soft/hard siege.
pub struct HarrisHawkUpdate {
    /// 逃逸能量的衰减源 / Decay source of the escaping energy
    decay: ParameterSlot,
}

impl HarrisHawkUpdate {
    /// 创建哈里斯鹰更新 / Create a Harris hawks update
    pub fn new(decay: Box<dyn ParameterSource>) -> Self {
        Self {
            decay: ParameterSlot::new(decay),
        }
    }
}

impl Default for HarrisHawkUpdate {
    fn default() -> Self {
        Self::new(Box::new(LinearSource::new(2.0, 0.0, 100)))
    }
}

impl<V> SolutionUpdate<V> for HarrisHawkUpdate
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
        if parents.is_empty() {
            return Vec::new();
        }

        let ordered: Vec<Vec<V>> = parents
            .iter()
            .map(|parent| read_values(model, parent))
            .collect();
        let size = ordered.len();
        let best = &ordered[0];
        let decay_factor = self.decay.value(iteration, entropy);

        ordered
            .iter()
            .map(|current| {
                let energy = decay_factor * (2.0 * unit(entropy) - 1.0);
                let donor = ((unit(entropy) * size as f64) as usize).min(size - 1);
                let random = &ordered[donor];
                let jump = 2.0 * (1.0 - unit(entropy));

                let mut child = Vec::with_capacity(current.len());
                for dimension in 0..current.len() {
                    let position = to_f64(&current[dimension]);
                    let best_position = to_f64(&best[dimension]);
                    let value = if energy.abs() >= 1.0 {
                        let random_position = to_f64(&random[dimension]);
                        random_position
                            - unit(entropy)
                                * (random_position - 2.0 * unit(entropy) * position).abs()
                    } else if energy.abs() >= 0.5 {
                        position - energy * (jump * best_position - position).abs()
                            + unit(entropy) * (best_position - position)
                    } else {
                        position - energy * (best_position - position).abs()
                    };
                    child.push(clamp_at(model, dimension, value));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 算术优化算法（AOA）。 / Arithmetic optimization algorithm (AOA).
///
/// 用「数学优化加速器」MOA 在探索与开发之间切换：`r₁ > MOA` 时用乘法或除法产生大幅跳变，否则用加法或减法
/// 产生小幅修正。
/// A "math optimizer accelerated" (MOA) coefficient switches between exploration (multiplication/division) and
/// exploitation (addition/subtraction).
pub struct ArithmeticOptimizationUpdate {
    /// 数学优化加速器 MOA 的演化源 / Evolution source of the math optimizer accelerated coefficient
    accelerator: ParameterSlot,
    /// 数学优化概率 MOP 的演化源 / Evolution source of the math optimizer probability
    probability: ParameterSlot,
    /// 数学优化概率 MOP 的敏感系数 μ / Sensitivity μ of the math optimizer probability
    pub sensitivity: f64,
}

impl ArithmeticOptimizationUpdate {
    /// 创建算术优化更新 / Create an arithmetic optimization update
    pub fn new(
        accelerator: Box<dyn ParameterSource>,
        probability: Box<dyn ParameterSource>,
        sensitivity: f64,
    ) -> Self {
        Self {
            accelerator: ParameterSlot::new(accelerator),
            probability: ParameterSlot::new(probability),
            sensitivity,
        }
    }
}

impl Default for ArithmeticOptimizationUpdate {
    fn default() -> Self {
        Self::new(
            Box::new(LinearSource::new(0.2, 0.9, 100)),
            Box::new(LinearSource::new(1.0, 0.001, 100)),
            0.5,
        )
    }
}

impl<V> SolutionUpdate<V> for ArithmeticOptimizationUpdate
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
        if parents.is_empty() {
            return Vec::new();
        }

        let ordered: Vec<Vec<V>> = parents
            .iter()
            .map(|parent| read_values(model, parent))
            .collect();
        let best = &ordered[0];
        let math_optimizer_accelerated = self.accelerator.value(iteration, entropy);
        let math_optimizer_probability = self
            .probability
            .value(iteration, entropy)
            .clamp(1.0e-6, 1.0);

        ordered
            .iter()
            .map(|current| {
                let first = unit(entropy);
                let second = unit(entropy);
                let third = unit(entropy);

                let mut child = Vec::with_capacity(current.len());
                for dimension in 0..current.len() {
                    let (lower, upper) = model.bounds_at(dimension);
                    let width = width_of(lower, upper);
                    let origin = origin_of(lower);
                    let scale = width * self.sensitivity + origin;
                    let best_position = to_f64(&best[dimension]);

                    let value = if first > math_optimizer_accelerated {
                        if second > 0.5 {
                            best_position / math_optimizer_probability * scale
                        } else {
                            best_position * math_optimizer_probability * scale
                        }
                    } else if third > 0.5 {
                        best_position - math_optimizer_probability * scale
                    } else {
                        best_position + math_optimizer_probability * scale
                    };
                    child.push(clamp_at(model, dimension, value));
                }
                write_values(model, child)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::solver::heuristic::entropy::SeededEntropy;
    use crate::solver::heuristic::iteration::Iteration;
    use crate::solver::heuristic::operator::evolution::ConstantSource;

    use super::super::test_support::{assert_dimension_and_bounds, model_with_bounds, solution};
    use super::super::SolutionUpdate;
    use super::{ArithmeticOptimizationUpdate, HarrisHawkUpdate, WhaleOptimizationUpdate};

    fn setup() -> (
        super::super::test_support::TestModel,
        Vec<crate::model::callback::Solution<f64>>,
    ) {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![
            solution(&test, 5.0, 5.0),
            solution(&test, 2.0, 9.0),
            solution(&test, 8.0, 1.0),
        ];
        (test, parents)
    }

    #[test]
    fn bio_updates_stay_within_bounds() {
        let (test, parents) = setup();
        let iteration = Iteration::new();

        let whale = WhaleOptimizationUpdate::default();
        let children = whale.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(91),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let hawk = HarrisHawkUpdate::default();
        let children = hawk.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(92),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let arithmetic = ArithmeticOptimizationUpdate::default();
        let children = arithmetic.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(93),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);
    }

    #[test]
    fn arithmetic_optimization_is_reproducible_for_the_same_seed() {
        let (test, parents) = setup();
        let iteration = Iteration::new();
        let operator = ArithmeticOptimizationUpdate::new(
            Box::new(ConstantSource::new(0.1)),
            Box::new(ConstantSource::new(0.5)),
            0.5,
        );

        let first = operator.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(101),
        );
        let second = operator.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(101),
        );
        assert_eq!(first, second);
    }
}
