//! Tier-1 群智能位置更新 / Tier-1 swarm position updates
//!
//! 水循环（WCA）与海鸥（SOA）都以「排名即优度」为前提：父代组首个个体是最优解。
//! Water cycle (WCA) and seagull (SOA) both rely on "rank is quality": the first parent is the best solution.

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

/// 水循环算法（WCA）。 / Water cycle algorithm (WCA).
///
/// 按优度把个体分成三层：海（最优）、河流（次优若干）、溪流（其余）。溪流汇入河流、河流汇入海：
/// Individuals are split into three layers by quality: the sea (best), rivers (the next few) and streams (the rest).
/// Streams flow into rivers and rivers into the sea:
///
/// X' = X + rand · (X_target − X)
///
/// 以 `raining_probability` 的概率降下新雨（在取值域内随机重生），为种群引入全新位置。
/// With probability `raining_probability` fresh rain is generated at a uniformly random position.
pub struct WaterCycleUpdate {
    /// 河流数量 / River amount
    pub river_amount: usize,
    /// 流动步长系数 / Flow step scale
    pub flow_scale: f64,
    /// 降雨概率 / Raining probability
    pub raining_probability: f64,
}

impl WaterCycleUpdate {
    /// 创建水循环更新 / Create a water cycle update
    pub fn new(river_amount: usize, flow_scale: f64, raining_probability: f64) -> Self {
        Self {
            river_amount,
            flow_scale,
            raining_probability,
        }
    }
}

impl Default for WaterCycleUpdate {
    fn default() -> Self {
        Self::new(4, 1.0, 0.1)
    }
}

impl<V> SolutionUpdate<V> for WaterCycleUpdate
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
        let sea = &ordered[0];
        let river_count = self.river_amount.min(ordered.len().saturating_sub(1));

        let mut children = Vec::with_capacity(ordered.len());
        for (index, current) in ordered.iter().enumerate() {
            if unit(entropy) < self.raining_probability {
                // 降雨：在取值域内随机重生。/ Raining: rebirth at a uniformly random position.
                let mut rain = Vec::with_capacity(current.len());
                for dimension in 0..current.len() {
                    let (lower, upper) = model.bounds_at(dimension);
                    let width = width_of(lower, upper);
                    let origin = origin_of(lower);
                    rain.push(clamp_at(
                        model,
                        dimension,
                        origin + unit(entropy) * width,
                    ));
                }
                children.push(write_values(model, rain));
                continue;
            }

            // 海自己不流动；河流流向海；溪流流向某条河流（按下标轮转分配）。
            // The sea does not flow; rivers flow into the sea; streams flow into one of the rivers, assigned round-robin.
            let target = if index == 0 || index <= river_count || river_count == 0 {
                sea
            } else {
                &ordered[((index - 1) % river_count) + 1]
            };

            let mut moved = Vec::with_capacity(current.len());
            for dimension in 0..current.len() {
                let (lower, upper) = model.bounds_at(dimension);
                let width = width_of(lower, upper);
                let step = if index == 0 {
                    // 海做一次小幅随机游走，避免失去多样性。
                    // The sea performs a small random walk to keep some diversity.
                    (unit(entropy) * 2.0 - 1.0) * 0.1 * width * self.flow_scale
                } else {
                    unit(entropy)
                        * (to_f64(&target[dimension]) - to_f64(&current[dimension]))
                        * self.flow_scale
                };
                moved.push(clamp_at(
                    model,
                    dimension,
                    to_f64(&current[dimension]) + step,
                ));
            }
            children.push(write_values(model, moved));
        }
        children
    }
}

/// 海鸥优化算法（SOA）。 / Seagull optimization algorithm (SOA).
///
/// 两个阶段由控制参数 A 切换：A ≥ 1 时个体被最优解牵引并叠加随机放大；A < 1 时沿螺旋轨迹围绕最优解收缩。
/// Two phases switched by the control parameter A: A ≥ 1 pulls the individual towards the best with random
/// amplification, while A < 1 contracts around the best along a spiral trajectory.
pub struct SeagullUpdate {
    /// 控制参数 A 的演化源 / Evolution source of the control parameter A
    control: ParameterSlot,
    /// 螺旋增长率 / Spiral growth rate
    pub spiral_rate: f64,
}

impl SeagullUpdate {
    /// 创建海鸥更新 / Create a seagull update
    pub fn new(control: Box<dyn ParameterSource>, spiral_rate: f64) -> Self {
        Self {
            control: ParameterSlot::new(control),
            spiral_rate,
        }
    }
}

impl Default for SeagullUpdate {
    fn default() -> Self {
        Self::new(Box::new(LinearSource::new(2.0, 0.0, 100)), 0.1)
    }
}

impl<V> SolutionUpdate<V> for SeagullUpdate
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
        let Some(best_parent) = parents.first() else {
            return Vec::new();
        };
        let best = read_values(model, best_parent);
        let coefficient = self.control.value(iteration, entropy);
        let migrated = coefficient >= 1.0;

        parents
            .iter()
            .map(|parent| {
                let current = read_values(model, parent);
                let dimension = current.len();
                let mut child = Vec::with_capacity(dimension);
                for index in 0..dimension {
                    let position = to_f64(&current[index]);
                    let target = to_f64(&best[index]);
                    let moved = if migrated {
                        let amplification = 2.0 * coefficient * coefficient * unit(entropy);
                        position + amplification * (target - position)
                    } else {
                        let theta = 2.0 * PI * unit(entropy);
                        let radius = (theta * self.spiral_rate).exp();
                        target + (position - target) * radius * theta.cos()
                    };
                    child.push(clamp_at(model, index, moved));
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
    use super::{SeagullUpdate, WaterCycleUpdate};

    fn setup() -> (
        super::super::test_support::TestModel,
        Vec<crate::model::callback::Solution<f64>>,
    ) {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![
            solution(&test, 5.0, 5.0),
            solution(&test, 2.0, 8.0),
            solution(&test, 9.0, 1.0),
            solution(&test, 4.0, 6.0),
        ];
        (test, parents)
    }

    #[test]
    fn water_cycle_raining_branch_stays_within_bounds() {
        let (test, parents) = setup();
        // rainingProbability = 1.0 时全部个体走降雨分支 / With rainingProbability = 1.0 every individual rains.
        let operator = WaterCycleUpdate::new(2, 1.0, 1.0);
        let children = operator.update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut SeededEntropy::new(71),
        );

        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);
    }

    #[test]
    fn water_cycle_flow_branch_keeps_dimension() {
        let (test, parents) = setup();
        let operator = WaterCycleUpdate::new(2, 1.0, 0.0);
        let children = operator.update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut SeededEntropy::new(72),
        );

        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);
        // 海保持原位，因为流动分支下海只做 ±0.1 宽度的游走 / The sea only random-walks within 10% of the width.
        let sea = &children[0];
        assert!(sea.get(&test.x).copied().unwrap_or_default() >= 4.0);
        assert!(sea.get(&test.x).copied().unwrap_or_default() <= 6.0);
    }

    #[test]
    fn seagull_update_is_reproducible_for_the_same_seed() {
        let (test, parents) = setup();
        let iteration = Iteration::new();
        let operator = SeagullUpdate::new(Box::new(ConstantSource::new(0.5)), 0.1);

        let first = operator.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(81),
        );
        let second = operator.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(81),
        );
        assert_eq!(first, second);
        assert_dimension_and_bounds(&first, 0.0, 10.0);
    }
}
