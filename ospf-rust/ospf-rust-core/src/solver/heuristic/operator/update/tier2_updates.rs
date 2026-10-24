//! Tier-2 位置更新（第一批）/ Tier-2 position updates (first batch)
//!
//! 教与学（TLBO）、樽海鞘（SSA）、引力搜索（GSA）与萤火虫（FA）。全部以「排名即优度」为前提。
//! Teaching-learning (TLBO), salp swarm (SSA), gravitational search (GSA), and firefly (FA), all relying on "rank is
//! quality".

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel, to_f64};
use crate::solver::heuristic::operator::evolution::{LinearSource, ParameterSource};
use crate::solver::heuristic::operator::unit;

use super::{
    ParameterSlot, SolutionUpdate, clamp_at, origin_of, read_values, width_of, write_values,
};

/// 教与学优化的「教师阶段」（TLBO）。 / The teacher phase of teaching-learning-based optimization (TLBO).
///
/// X' = X + r·(X_teacher − T_F·M)
///
/// `X_teacher` 是当前最优（父代已按优度升序），`M` 是种群均值，`T_F = 1 + round(r)` 是教学因子。
/// `X_teacher` is the current best (parents are ranked by ascending quality), `M` is the population mean, and
/// `T_F = 1 + round(r)` is the teaching factor.
#[derive(Debug, Clone, Copy)]
pub struct TeachingLearningUpdate {
    /// 教学因子的上界 / Upper bound of the teaching factor
    pub teaching_factor_range: f64,
}

impl TeachingLearningUpdate {
    /// 创建教师阶段更新 / Create a teacher-phase update
    pub fn new(teaching_factor_range: f64) -> Self {
        Self {
            teaching_factor_range,
        }
    }
}

impl Default for TeachingLearningUpdate {
    fn default() -> Self {
        Self::new(2.0)
    }
}

impl<V> SolutionUpdate<V> for TeachingLearningUpdate
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
        let size = ordered.len();
        let teacher = &ordered[0];
        let dimension = teacher.len();

        let mut mean = vec![0.0_f64; dimension];
        for parent in &ordered {
            for dim in 0..dimension {
                mean[dim] += to_f64(&parent[dim]);
            }
        }
        for value in &mut mean {
            *value /= size as f64;
        }

        ordered
            .iter()
            .map(|current| {
                let factor = 1.0 + (unit(entropy) * self.teaching_factor_range).round();
                let mut child = Vec::with_capacity(dimension);
                for dim in 0..dimension {
                    let step = unit(entropy) * (to_f64(&teacher[dim]) - factor * mean[dim]);
                    child.push(clamp_at(model, dim, to_f64(&current[dim]) + step));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 教与学优化的「学生阶段」（TLBO）。 / The learner phase of teaching-learning-based optimization (TLBO).
///
/// 学生随机找一个同学互学；优劣判断直接用**下标**：骨架保证父代按优度升序，因此下标更小即更优。
/// A learner picks a classmate at random; quality is compared by **index** because the skeleton guarantees ascending
/// quality.
#[derive(Debug, Clone, Copy, Default)]
pub struct LearnerPhaseUpdate;

impl LearnerPhaseUpdate {
    /// 创建学生阶段更新 / Create a learner-phase update
    pub fn new() -> Self {
        Self
    }
}

impl<V> SolutionUpdate<V> for LearnerPhaseUpdate
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
        if parents.len() < 2 {
            return parents.to_vec();
        }

        let ordered: Vec<Vec<V>> = parents
            .iter()
            .map(|parent| read_values(model, parent))
            .collect();
        let size = ordered.len();

        ordered
            .iter()
            .enumerate()
            .map(|(index, current)| {
                let mut partner_index = ((unit(entropy) * size as f64) as usize).min(size - 1);
                if partner_index == index {
                    partner_index = (index + 1) % size;
                }
                let partner = &ordered[partner_index];
                let sign = if index < partner_index { 1.0 } else { -1.0 };

                let mut child = Vec::with_capacity(current.len());
                for dim in 0..current.len() {
                    let step =
                        unit(entropy) * sign * (to_f64(&partner[dim]) - to_f64(&current[dim]));
                    child.push(clamp_at(model, dim, to_f64(&current[dim]) + step));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 樽海鞘群算法（SSA）。 / Salp swarm algorithm (SSA).
///
/// 链首是领导者，其余是跟随者：领导者围绕最优解在变量域内摆动，跟随者向自己的前一个个体靠拢。
/// The chain has one leader and the rest are followers: the leader swings around the best within the variable domain,
/// and every follower moves towards the individual in front of it.
pub struct SalpSwarmUpdate {
    /// 控制参数 c₁ 的演化源 / Evolution source of the control parameter c₁
    decay: ParameterSlot,
}

impl SalpSwarmUpdate {
    /// 创建樽海鞘更新 / Create a salp swarm update
    pub fn new(decay: Box<dyn ParameterSource>) -> Self {
        Self {
            decay: ParameterSlot::new(decay),
        }
    }
}

impl Default for SalpSwarmUpdate {
    fn default() -> Self {
        Self::new(Box::new(LinearSource::new(2.0, 0.0, 100)))
    }
}

impl<V> SolutionUpdate<V> for SalpSwarmUpdate
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
        let leader = &ordered[0];
        let coefficient = self.decay.value(iteration, entropy);

        ordered
            .iter()
            .enumerate()
            .map(|(index, current)| {
                let mut child = Vec::with_capacity(current.len());
                for dim in 0..current.len() {
                    let (lower, upper) = model.bounds_at(dim);
                    let width = width_of(lower, upper);
                    let origin = origin_of(lower);

                    let value = if index == 0 {
                        let swing = unit(entropy);
                        let sign = if unit(entropy) < 0.5 { -1.0 } else { 1.0 };
                        to_f64(&leader[dim]) + sign * coefficient * (width * swing + origin)
                    } else {
                        (to_f64(&current[dim]) + to_f64(&ordered[index - 1][dim])) / 2.0
                    };
                    child.push(clamp_at(model, dim, value));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 引力搜索算法（GSA）。 / Gravitational search algorithm (GSA).
///
/// a_i = G·Σ_j m_j·(X_j − X_i) / (r_ij + ε)，X' = X + a_i；质量用**排名**近似（排名是优度的单调变换）。
/// a_i = G·Σ_j m_j·(X_j − X_i) / (r_ij + ε) and X' = X + a_i; mass is approximated by **rank**, a monotone transform of
/// quality.
pub struct GravitationalSearchUpdate {
    /// 引力常数 G 的演化源 / Evolution source of the gravitational constant G
    gravitational_constant: ParameterSlot,
    /// 距离软化的 ε / Softening ε of the distance
    pub softness: f64,
}

impl GravitationalSearchUpdate {
    /// 创建引力搜索更新 / Create a gravitational search update
    pub fn new(gravitational_constant: Box<dyn ParameterSource>, softness: f64) -> Self {
        Self {
            gravitational_constant: ParameterSlot::new(gravitational_constant),
            softness,
        }
    }
}

impl Default for GravitationalSearchUpdate {
    fn default() -> Self {
        Self::new(Box::new(LinearSource::new(1.0, 0.01, 100)), 1.0e-6)
    }
}

impl<V> SolutionUpdate<V> for GravitationalSearchUpdate
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
        let constant = self.gravitational_constant.value(iteration, entropy);

        // 排名越靠前质量越大；归一化到总和为 1。
        // Better ranks carry larger mass; masses are normalised to sum to 1.
        let mass_total = (size * (size + 1) / 2) as f64;
        let masses: Vec<f64> = (0..size)
            .map(|index| (size - index) as f64 / mass_total)
            .collect();

        ordered
            .iter()
            .enumerate()
            .map(|(index, current)| {
                let dimension = current.len();
                let mut child = Vec::with_capacity(dimension);
                for dim in 0..dimension {
                    let mut acceleration = 0.0;
                    for other in 0..size {
                        if other == index {
                            continue;
                        }
                        let mut distance_squared = 0.0;
                        for axis in 0..dimension {
                            let delta =
                                to_f64(&ordered[other][axis]) - to_f64(&current[axis]);
                            distance_squared += delta * delta;
                        }
                        let distance = distance_squared.sqrt() + self.softness;
                        acceleration += constant
                            * masses[other]
                            * (to_f64(&ordered[other][dim]) - to_f64(&current[dim]))
                            / distance;
                    }
                    child.push(clamp_at(
                        model,
                        dim,
                        to_f64(&current[dim]) + acceleration,
                    ));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 萤火虫算法（FA）。 / Firefly algorithm (FA).
///
/// β = β₀·e^{−γ·r²}，X' = X + β·(X_bright − X) + α·(rand − 0.5)；亮度即优度，用**排名**判定「更亮」。
/// β = β₀·e^{−γ·r²} and X' = X + β·(X_bright − X) + α·(rand − 0.5); brightness is quality, decided by **rank**.
#[derive(Debug, Clone, Copy)]
pub struct FireflyUpdate {
    /// 初始吸引力 β₀ / Initial attractiveness β₀
    pub attractiveness: f64,
    /// 光吸收系数 γ / Light absorption coefficient γ
    pub absorption: f64,
    /// 随机项系数 α / Randomness coefficient α
    pub randomness: f64,
}

impl FireflyUpdate {
    /// 创建萤火虫更新 / Create a firefly update
    pub fn new(attractiveness: f64, absorption: f64, randomness: f64) -> Self {
        Self {
            attractiveness,
            absorption,
            randomness,
        }
    }
}

impl Default for FireflyUpdate {
    fn default() -> Self {
        Self::new(1.0, 1.0, 0.2)
    }
}

impl<V> SolutionUpdate<V> for FireflyUpdate
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

        ordered
            .iter()
            .enumerate()
            .map(|(index, current)| {
                // 排名更靠前的候选同伴；自己已是最优时不移动，只做随机抖动。
                // Candidates with a better rank; the best firefly only jitters.
                let partner_index = if index > 0 {
                    ((unit(entropy) * index as f64) as usize).min(index - 1)
                } else {
                    index
                };
                let partner = &ordered[partner_index];

                let mut distance_squared = 0.0;
                for dim in 0..current.len() {
                    let delta = to_f64(&partner[dim]) - to_f64(&current[dim]);
                    distance_squared += delta * delta;
                }
                let attraction = self.attractiveness * (-self.absorption * distance_squared).exp();

                let mut child = Vec::with_capacity(current.len());
                for dim in 0..current.len() {
                    let (lower, upper) = model.bounds_at(dim);
                    let width = width_of(lower, upper);
                    let position = to_f64(&current[dim]);
                    let pull = attraction * (to_f64(&partner[dim]) - position);
                    let jitter = self.randomness * (unit(entropy) - 0.5) * width;
                    child.push(clamp_at(model, dim, position + pull + jitter));
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
        FireflyUpdate, GravitationalSearchUpdate, LearnerPhaseUpdate, SalpSwarmUpdate,
        TeachingLearningUpdate,
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
            solution(&test, 3.0, 7.0),
        ];
        (test, parents)
    }

    #[test]
    fn tier2_updates_stay_within_bounds() {
        let (test, parents) = setup();
        let iteration = Iteration::new();

        let teaching = TeachingLearningUpdate::default();
        let children = teaching.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(181),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let learner = LearnerPhaseUpdate::new();
        let children = learner.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(182),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let salp = SalpSwarmUpdate::default();
        let children = salp.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(183),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let gravitational = GravitationalSearchUpdate::default();
        let children = gravitational.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(184),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let firefly = FireflyUpdate::default();
        let children = firefly.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(185),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);
    }

    #[test]
    fn learner_phase_with_a_single_parent_returns_it_unchanged() {
        let (test, parents) = setup();
        let single = vec![parents[0].clone()];

        let children = LearnerPhaseUpdate::new().update(
            &test.model,
            &Iteration::new(),
            &single,
            &mut SeededEntropy::new(186),
        );
        assert_eq!(children, single);
    }
}
