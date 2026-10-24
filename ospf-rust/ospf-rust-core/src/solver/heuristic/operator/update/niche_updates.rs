//! 细分启发式位置更新 / Niche heuristic position updates
//!
//! 战争策略（WarSO）、能量谷（EVO）与生发中心（GCO）。三者都把「排名」当作优度：更靠前的个体更接近最优。
//! War strategy (WarSO), energy valley (EVO), and germinal centre (GCO). All three treat the rank as quality: earlier
//! individuals are closer to the optimum.

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel, to_f64};
use crate::solver::heuristic::operator::unit;

use super::{SolutionUpdate, clamp_at, read_values, width_of, write_values};

/// 战争策略优化（WarSO）。 / War strategy optimization (WarSO).
///
/// 每个个体被视为士兵：排名靠前者以 `attack_rate` 概率「进攻」（靠近最优），否则「防守」（向随机战友靠拢）。
/// Every individual is a soldier: better-ranked soldiers attack (close on the best) with probability `attack_rate`,
/// otherwise they defend (regroup towards a random comrade).
#[derive(Debug, Clone, Copy)]
pub struct WarStrategyUpdate {
    /// 进攻概率 / Attack probability
    pub attack_rate: f64,
    /// 策略步长系数 / Strategy step scale
    pub strategy_scale: f64,
}

impl WarStrategyUpdate {
    /// 创建战争策略更新 / Create a war strategy update
    pub fn new(attack_rate: f64, strategy_scale: f64) -> Self {
        Self {
            attack_rate,
            strategy_scale,
        }
    }
}

impl Default for WarStrategyUpdate {
    fn default() -> Self {
        Self::new(0.6, 2.0)
    }
}

impl<V> SolutionUpdate<V> for WarStrategyUpdate
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
        let size = ordered.len();

        ordered
            .iter()
            .map(|current| {
                let attacking = unit(entropy) < self.attack_rate;
                let target = if attacking {
                    best
                } else {
                    let comrade = ((unit(entropy) * size as f64) as usize).min(size - 1);
                    &ordered[comrade]
                };
                // 进攻对「向最优收缩」做 2 倍放大是原文的设定；防守目标为随机战友，只取 1 倍步长以保持收缩。
                // Attack amplifies the step towards the best by 2x as in the original; defence uses a plain 1x step.
                let scale = if attacking { self.strategy_scale } else { 1.0 };

                let mut child = Vec::with_capacity(current.len());
                for dimension in 0..current.len() {
                    let step = scale
                        * unit(entropy)
                        * (to_f64(&target[dimension]) - to_f64(&current[dimension]));
                    child.push(clamp_at(
                        model,
                        dimension,
                        to_f64(&current[dimension]) + step,
                    ));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 能量谷优化（EVO）。 / Energy valley optimizer (EVO).
///
/// 排名靠前的个体处于「稳定态」，围绕最优解小幅度振荡；其余个体「不稳定」，通过衰变跳向其他位置。
/// Better-ranked individuals are "stable" and oscillate around the best, while the rest are "unstable" and decay
/// towards other positions.
#[derive(Debug, Clone, Copy)]
pub struct EnergyValleyUpdate {
    /// 稳定态比例 / Proportion of the population treated as stable
    pub stability_rate: f64,
    /// 衰变步长系数 / Decay step scale
    pub decay_scale: f64,
}

impl EnergyValleyUpdate {
    /// 创建能量谷更新 / Create an energy valley update
    pub fn new(stability_rate: f64, decay_scale: f64) -> Self {
        Self {
            stability_rate,
            decay_scale,
        }
    }
}

impl Default for EnergyValleyUpdate {
    fn default() -> Self {
        Self::new(0.5, 1.0)
    }
}

impl<V> SolutionUpdate<V> for EnergyValleyUpdate
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
        let size = ordered.len();
        let stable_amount = ((size as f64 * self.stability_rate) as usize).clamp(1, size);

        ordered
            .iter()
            .enumerate()
            .map(|(index, current)| {
                let stable = index < stable_amount;
                let target = if stable {
                    best
                } else {
                    let other = ((unit(entropy) * size as f64) as usize).min(size - 1);
                    &ordered[other]
                };
                let scale = if stable { 1.0 } else { self.decay_scale };

                let mut child = Vec::with_capacity(current.len());
                for dimension in 0..current.len() {
                    let step = scale
                        * unit(entropy)
                        * (to_f64(&target[dimension]) - to_f64(&current[dimension]));
                    child.push(clamp_at(
                        model,
                        dimension,
                        to_f64(&current[dimension]) + step,
                    ));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 生发中心优化（GCO）。 / Germinal center optimization (GCO).
///
/// 精英个体构成生发中心，其余个体向中心的**质心**靠拢并叠加一次随机突变。
/// Elite individuals form the germinal centre; the rest contract towards its **centroid** with one random mutation on
/// top.
///
/// X' = X + r·(centroid − X) + mutationScale·(2r − 1)·width
#[derive(Debug, Clone, Copy)]
pub struct GerminalCenterUpdate {
    /// 生发中心规模 / Germinal centre size
    pub elite_amount: usize,
    /// 突变幅度，相对于变量取值范围 / Mutation magnitude relative to the variable range
    pub mutation_scale: f64,
}

impl GerminalCenterUpdate {
    /// 创建生发中心更新 / Create a germinal centre update
    pub fn new(elite_amount: usize, mutation_scale: f64) -> Self {
        Self {
            elite_amount,
            mutation_scale,
        }
    }
}

impl Default for GerminalCenterUpdate {
    fn default() -> Self {
        Self::new(3, 0.1)
    }
}

impl<V> SolutionUpdate<V> for GerminalCenterUpdate
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
        let elite_count = self.elite_amount.clamp(1, ordered.len());
        let dimension = ordered[0].len();

        let mut centroid = vec![0.0_f64; dimension];
        for elite in &ordered[..elite_count] {
            for (dim, value) in elite.iter().enumerate() {
                centroid[dim] += to_f64(value);
            }
        }
        for value in &mut centroid {
            *value /= elite_count as f64;
        }

        ordered
            .iter()
            .map(|current| {
                let mut child = Vec::with_capacity(dimension);
                for dim in 0..dimension {
                    let (lower, upper) = model.bounds_at(dim);
                    let width = width_of(lower, upper);
                    let position = to_f64(&current[dim]);
                    let contraction = unit(entropy) * (centroid[dim] - position);
                    let mutation = self.mutation_scale * (unit(entropy) * 2.0 - 1.0) * width;
                    child.push(clamp_at(
                        model,
                        dim,
                        position + contraction + mutation,
                    ));
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
    use super::{EnergyValleyUpdate, GerminalCenterUpdate, WarStrategyUpdate};

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
    fn niche_updates_stay_within_bounds() {
        let (test, parents) = setup();
        let iteration = Iteration::new();

        let war = WarStrategyUpdate::default();
        let children = war.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(171),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let energy = EnergyValleyUpdate::default();
        let children = energy.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(172),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let germinal = GerminalCenterUpdate::default();
        let children = germinal.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(173),
        );
        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);
    }

    #[test]
    fn germinal_centre_contracts_towards_the_elite_centroid() {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![solution(&test, 4.0, 4.0), solution(&test, 6.0, 6.0)];
        // 突变幅度为 0 时子代必须落在两个精英之间 / With zero mutation children stay between the two elites.
        let operator = GerminalCenterUpdate::new(2, 0.0);

        let children = operator.update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut SeededEntropy::new(174),
        );

        for child in &children {
            for value in child.values() {
                assert!(*value >= 4.0 - 1.0e-9 && *value <= 6.0 + 1.0e-9);
            }
        }
    }
}
