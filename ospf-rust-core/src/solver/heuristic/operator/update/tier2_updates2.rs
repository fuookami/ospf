//! Tier-2 位置更新（第二批）/ Tier-2 position updates (second batch)
//!
//! 布谷鸟（CS）、蝙蝠（BA）、飞蛾扑火（MFO）与蝗虫（GOA）。
//! Cuckoo search (CS), bat algorithm (BA), moth-flame (MFO), and grasshopper (GOA).

use std::f64::consts::PI;

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel, to_f64};
use crate::solver::heuristic::operator::evolution::{LinearSource, ParameterSource};
use crate::solver::heuristic::operator::unit;

use super::{
    ParameterSlot, SolutionUpdate, clamp_at, origin_of, read_values, width_of, write_values,
};

/// Mantegna 公式中的 σ / σ of Mantegna's formula
fn levy_sigma(exponent: f64) -> f64 {
    let numerator = gamma(1.0 + exponent) * (PI * exponent / 2.0).sin();
    let denominator =
        gamma((1.0 + exponent) / 2.0) * exponent * 2.0_f64.powf((exponent - 1.0) / 2.0);
    (numerator / denominator).powf(1.0 / exponent)
}

/// Lanczos 近似的 Γ 函数 / Γ function through the Lanczos approximation
fn gamma(value: f64) -> f64 {
    let coefficients = [
        676.5203681218851,
        -1259.1392167224028,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507343278686905,
        -0.13857109526572012,
        9.984_369_578_019_572e-6,
        1.5056327351493116e-7,
    ];
    if value < 0.5 {
        return PI / ((PI * value).sin() * gamma(1.0 - value));
    }
    let shifted = value - 1.0;
    let mut series = 0.999_999_999_999_809_9;
    for (index, coefficient) in coefficients.iter().enumerate() {
        series += coefficient / (shifted + index as f64 + 1.0);
    }
    let t = shifted + coefficients.len() as f64 - 0.5;
    (2.0 * PI).sqrt() * t.powf(shifted + 0.5) * (-t).exp() * series
}

/// 由均匀熵源近似标准正态（4 次均匀求和）/ Approximate a standard normal from four uniforms
fn normal(entropy: &mut dyn Entropy) -> f64 {
    unit(entropy) + unit(entropy) + unit(entropy) + unit(entropy) - 2.0
}

/// 布谷鸟搜索（CS）。 / Cuckoo search (CS).
///
/// 莱维飞行：X' = X + α·Levy(λ)；宿主发现：以概率 `p_a` 放弃当前巢，直接在变量域内重建。
/// Lévy flight: X' = X + α·Levy(λ); host discovery: with probability `p_a` the nest is rebuilt uniformly inside the
/// domain.
#[derive(Debug, Clone, Copy)]
pub struct CuckooSearchUpdate {
    /// 莱维指数 λ / Lévy exponent λ
    pub levy_exponent: f64,
    /// 步长系数 α / Step scale α
    pub step_scale: f64,
    /// 宿主发现概率 p_a / Host discovery probability p_a
    pub abandonment_probability: f64,
}

impl CuckooSearchUpdate {
    /// 创建布谷鸟更新 / Create a cuckoo search update
    pub fn new(levy_exponent: f64, step_scale: f64, abandonment_probability: f64) -> Self {
        Self {
            levy_exponent,
            step_scale,
            abandonment_probability,
        }
    }
}

impl Default for CuckooSearchUpdate {
    fn default() -> Self {
        Self::new(1.5, 0.01, 0.25)
    }
}

impl<V> SolutionUpdate<V> for CuckooSearchUpdate
where
    V: HeuristicValue,
{
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
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
        let sigma = levy_sigma(self.levy_exponent);

        ordered
            .iter()
            .map(|current| {
                let abandoned = unit(entropy) < self.abandonment_probability;
                let mut child = Vec::with_capacity(current.len());
                for dim in 0..current.len() {
                    let (lower, upper) = model.bounds_at(dim);
                    let width = width_of(lower, upper);
                    let origin = origin_of(lower);
                    let position = to_f64(&current[dim]);

                    let value = if abandoned {
                        // 放弃巢穴：在变量域内重建。/ Abandon the nest: rebuild uniformly inside the domain.
                        origin + unit(entropy) * width
                    } else {
                        let u = normal(entropy) * sigma;
                        let v = normal(entropy).abs().max(1.0e-12);
                        let levy = u / v.powf(1.0 / self.levy_exponent);
                        // 朝最优方向缩放，避免无方向的纯随机游走。
                        // Scale towards the best so the walk is not purely undirected.
                        position + self.step_scale * levy * (position - to_f64(&best[dim]) + width)
                    };
                    child.push(clamp_at(model, dim, value));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 蝙蝠算法（BA）。 / Bat algorithm (BA).
///
/// f = f_min + (f_max − f_min)·β，X' = X + (X − X_best)·f·r；速度是跨迭代状态，这里用频率直接作用于位置。
/// f = f_min + (f_max − f_min)·β and X' = X + (X − X_best)·f·r; velocity is cross-iteration state, so frequency acts
/// directly on the position here.
#[derive(Debug, Clone, Copy)]
pub struct BatAlgorithmUpdate {
    /// 最小频率 / Minimum frequency
    pub minimum_frequency: f64,
    /// 最大频率 / Maximum frequency
    pub maximum_frequency: f64,
}

impl BatAlgorithmUpdate {
    /// 创建蝙蝠更新 / Create a bat algorithm update
    pub fn new(minimum_frequency: f64, maximum_frequency: f64) -> Self {
        Self {
            minimum_frequency,
            maximum_frequency,
        }
    }
}

impl Default for BatAlgorithmUpdate {
    fn default() -> Self {
        Self::new(0.0, 2.0)
    }
}

impl<V> SolutionUpdate<V> for BatAlgorithmUpdate
where
    V: HeuristicValue,
{
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
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

        ordered
            .iter()
            .map(|current| {
                let frequency = self.minimum_frequency
                    + unit(entropy) * (self.maximum_frequency - self.minimum_frequency);
                let mut child = Vec::with_capacity(current.len());
                for dim in 0..current.len() {
                    let position = to_f64(&current[dim]);
                    let step = (position - to_f64(&best[dim])) * frequency * unit(entropy);
                    child.push(clamp_at(model, dim, position + step));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 飞蛾扑火优化（MFO）。 / Moth-flame optimization (MFO).
///
/// X' = |F − X|·e^{b·t}·cos(2πt) + F, t ∈ [−1, 1]；火焰数量随迭代从种群规模递减到 1。
/// X' = |F − X|·e^{b·t}·cos(2πt) + F with t ∈ [−1, 1]; the flame count decreases from the population size towards 1.
pub struct MothFlameUpdate {
    /// 螺旋形状参数 b / Spiral shape parameter b
    pub spiral_rate: f64,
    /// 火焰数量占种群规模的比例演化源 / Evolution source of the flame count as a ratio of the population size
    flame_ratio: ParameterSlot,
}

impl MothFlameUpdate {
    /// 创建飞蛾扑火更新 / Create a moth-flame update
    pub fn new(spiral_rate: f64, flame_ratio: Box<dyn ParameterSource>) -> Self {
        Self {
            spiral_rate,
            flame_ratio: ParameterSlot::new(flame_ratio),
        }
    }
}

impl Default for MothFlameUpdate {
    fn default() -> Self {
        Self::new(1.0, Box::new(LinearSource::new(1.0, 0.01, 100)))
    }
}

impl<V> SolutionUpdate<V> for MothFlameUpdate
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
        // 火焰数量随迭代递减到 1，使搜索从多点并行探索过渡到围绕最优精修。
        // The flame count decreases towards 1, shifting the search from parallel exploration to refinement.
        let flame_amount = ((size as f64 * self.flame_ratio.value(iteration, entropy)) as usize)
            .clamp(1, size);

        ordered
            .iter()
            .enumerate()
            .map(|(index, current)| {
                let flame = &ordered[(index % flame_amount).min(size - 1)];
                let spiral = unit(entropy) * 2.0 - 1.0;

                let mut child = Vec::with_capacity(current.len());
                for dim in 0..current.len() {
                    let distance = (to_f64(&flame[dim]) - to_f64(&current[dim])).abs();
                    let value = distance
                        * (self.spiral_rate * spiral).exp()
                        * (2.0 * PI * spiral).cos()
                        + to_f64(&flame[dim]);
                    child.push(clamp_at(model, dim, value));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 蝗虫优化算法（GOA）。 / Grasshopper optimization algorithm (GOA).
///
/// X' = c·Σ_j [c·(UB − LB)/2·s(|X_j − X|)·(X_j − X)/d] + X_best，s(r) = f·e^{−r/l} − e^{−r}。
/// X' = c·Σ_j [c·(UB − LB)/2·s(|X_j − X|)·(X_j − X)/d] + X_best with s(r) = f·e^{−r/l} − e^{−r}.
pub struct GrasshopperUpdate {
    /// 收缩系数 c 的演化源 / Evolution source of the shrinking coefficient c
    shrink: ParameterSlot,
    /// 吸引强度 f / Attraction strength f
    pub attraction_scale: f64,
    /// 吸引长度尺度 l / Attraction length scale l
    pub attraction_length: f64,
}

impl GrasshopperUpdate {
    /// 创建蝗虫更新 / Create a grasshopper update
    pub fn new(
        shrink: Box<dyn ParameterSource>,
        attraction_scale: f64,
        attraction_length: f64,
    ) -> Self {
        Self {
            shrink: ParameterSlot::new(shrink),
            attraction_scale,
            attraction_length,
        }
    }
}

impl Default for GrasshopperUpdate {
    fn default() -> Self {
        Self::new(
            Box::new(LinearSource::new(1.0, 0.00004, 100)),
            0.5,
            1.5,
        )
    }
}

impl<V> SolutionUpdate<V> for GrasshopperUpdate
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
        let coefficient = self.shrink.value(iteration, entropy);

        ordered
            .iter()
            .map(|current| {
                let mut child = Vec::with_capacity(current.len());
                for dim in 0..current.len() {
                    let (lower, upper) = model.bounds_at(dim);
                    let width = width_of(lower, upper);

                    let mut social = 0.0;
                    for other in &ordered {
                        let distance = (to_f64(&other[dim]) - to_f64(&current[dim])).abs();
                        if distance <= 0.0 {
                            continue;
                        }
                        let interaction = self.attraction_scale
                            * (-distance / self.attraction_length).exp()
                            - (-distance).exp();
                        social += interaction
                            * (to_f64(&other[dim]) - to_f64(&current[dim]))
                            / distance;
                    }

                    let value = coefficient * coefficient * (width / 2.0) * social
                        + to_f64(&best[dim]);
                    child.push(clamp_at(model, dim, value));
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

    use super::super::test_support::{assert_dimension_and_bounds, model_with_bounds, solution};
    use super::super::SolutionUpdate;
    use super::{
        BatAlgorithmUpdate, CuckooSearchUpdate, GrasshopperUpdate, MothFlameUpdate, gamma,
        levy_sigma,
    };

    fn setup() -> (
        super::super::test_support::TestModel,
        Vec<crate::model::callback::Solution<f64>>,
    ) {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![
            solution(&test, 5.0, 5.0),
            solution(&test, 1.0, 9.0),
            solution(&test, 9.0, 1.0),
        ];
        (test, parents)
    }

    #[test]
    fn tier2_second_batch_stays_within_bounds() {
        let (test, parents) = setup();
        let iteration = Iteration::new();

        let cuckoo = CuckooSearchUpdate::default();
        let children = cuckoo.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(191),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let bat = BatAlgorithmUpdate::default();
        let children = bat.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(192),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let moth = MothFlameUpdate::default();
        let children = moth.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(193),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let grasshopper = GrasshopperUpdate::default();
        let children = grasshopper.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(194),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);
    }

    #[test]
    fn mantegna_sigma_and_gamma_match_the_analytic_values() {
        // Γ(2.5) = 1.329340388...，Γ(1) = 1
        assert!((gamma(1.0) - 1.0).abs() < 1.0e-9);
        assert!((gamma(2.5) - 1.329_340_388_1).abs() < 1.0e-6);
        assert!(levy_sigma(1.5).is_finite() && levy_sigma(1.5) > 0.0);
    }

    #[test]
    fn cuckoo_search_is_reproducible_for_the_same_seed() {
        let (test, parents) = setup();
        let iteration = Iteration::new();
        let operator = CuckooSearchUpdate::default();

        let first = operator.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(201),
        );
        let second = operator.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(201),
        );
        assert_eq!(first, second);
    }
}
