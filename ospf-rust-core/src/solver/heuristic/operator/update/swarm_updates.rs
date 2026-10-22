//! 正弦余弦与多元宇宙位置更新 / Sine cosine and multi-verse position updates
//!
//! 两个算子都以父代组首个个体为当前最优：种群骨架保证父代按优度升序，因此下标即排名。
//! Both operators treat the first parent as the current best: the population skeleton guarantees ascending quality, so
//! the index is the rank.

use std::f64::consts::PI;

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel, to_f64};
use crate::solver::heuristic::operator::evolution::{LinearSource, ParameterSource};
use crate::solver::heuristic::operator::unit;

use super::{ParameterSlot, SolutionUpdate, clamp_at, origin_of, read_values, width_of, write_values};

/// 正弦余弦位置更新（SCA）。 / Sine cosine position update (SCA).
///
/// 逐维用正弦或余弦把个体拉向（或推离）当前最优解： / Every dimension moves towards (or away from) the best solution:
///
/// X' = X + r₁ · sin(r₂) · |r₃ · X_best − X|   (r₄ < 0.5)
/// X' = X + r₁ · cos(r₂) · |r₃ · X_best − X|   (otherwise)
///
/// `r₁` 从 2 衰减到 0 控制探索/开发切换，`r₂ ∈ [0, 2π]` 决定方向，`r₃ ∈ [0, 2]` 决定对最优解的拉伸。
/// `r₁` decaying from 2 to 0 drives the exploration/exploitation shift, `r₂ ∈ [0, 2π]` sets the direction, and `r₃`
/// sets the stretch relative to the best solution.
pub struct SineCosineUpdate {
    amplitude: ParameterSlot,
}

impl SineCosineUpdate {
    /// 创建 SCA 位置更新 / Create a sine cosine update
    pub fn new(amplitude: Box<dyn ParameterSource>) -> Self {
        Self {
            amplitude: ParameterSlot::new(amplitude),
        }
    }
}

impl Default for SineCosineUpdate {
    fn default() -> Self {
        Self::new(Box::new(LinearSource::new(2.0, 0.0, 100)))
    }
}

impl<V> SolutionUpdate<V> for SineCosineUpdate
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
        let r1 = self.amplitude.value(iteration, entropy);

        parents
            .iter()
            .map(|parent| {
                let current = read_values(model, parent);
                let dimension = current.len();
                let mut child = Vec::with_capacity(dimension);
                for index in 0..dimension {
                    let r2 = unit(entropy) * 2.0 * PI;
                    let r3 = unit(entropy) * 2.0;
                    let r4 = unit(entropy);
                    let position = to_f64(&current[index]);
                    let target = to_f64(&best[index]);
                    let wave = if r4 < 0.5 { r2.sin() } else { r2.cos() };
                    let moved = position + r1 * wave * (r3 * target - position).abs();
                    child.push(clamp_at(model, index, moved));
                }
                write_values(model, child)
            })
            .collect()
    }
}

/// 多元宇宙位置更新（MVO）。 / Multi-verse position update (MVO).
///
/// 每个宇宙按虫洞存在概率 WEP 决定「穿越到最优宇宙附近」还是「通过白洞获取更优宇宙的物质」： /
/// Each universe either traverses towards the best universe according to the wormhole existence probability (WEP), or
/// acquires matter from a better universe through a white hole:
///
/// X' = X_best ± TDR · ((UB − LB) · r₄ + LB)              (r₂ < WEP)
/// X' = X_donor                                          (r₂ < 归一化膨胀率)
/// X' = X                                                (otherwise)
///
/// 由于解更新契约看不到目标值，白洞捐赠者按**排名加权**抽取：排名越靠前越可能被选中。
/// The update contract cannot see objective values, so the white-hole donor is drawn with **rank weights**: earlier
/// ranks are more likely to be chosen.
pub struct MultiVerseUpdate {
    wormhole_rate: ParameterSlot,
    travelling_rate: ParameterSlot,
}

impl MultiVerseUpdate {
    /// 创建 MVO 位置更新 / Create a multi-verse update
    pub fn new(wormhole_rate: Box<dyn ParameterSource>, travelling_rate: Box<dyn ParameterSource>) -> Self {
        Self {
            wormhole_rate: ParameterSlot::new(wormhole_rate),
            travelling_rate: ParameterSlot::new(travelling_rate),
        }
    }
}

impl Default for MultiVerseUpdate {
    fn default() -> Self {
        Self::new(
            Box::new(LinearSource::new(0.2, 1.0, 100)),
            Box::new(LinearSource::new(1.0, 0.0, 100)),
        )
    }
}

/// 按排名加权抽取白洞捐赠者 / Draw a white-hole donor weighted by rank
fn pick_donor(size: usize, entropy: &mut dyn Entropy) -> usize {
    if size <= 1 {
        return 0;
    }
    let total = size as f64 * (size + 1) as f64 / 2.0;
    let mut remaining = unit(entropy) * total;
    for rank in 0..size {
        let weight = (size - rank) as f64;
        if remaining < weight {
            return rank;
        }
        remaining -= weight;
    }
    0
}

impl<V> SolutionUpdate<V> for MultiVerseUpdate
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
        let size = parents.len();
        let wormhole = self.wormhole_rate.value(iteration, entropy);
        let travelling = self.travelling_rate.value(iteration, entropy);

        let mut children = Vec::with_capacity(size);
        for (index, current) in ordered.iter().enumerate() {
            let inflation = (size - index) as f64 / size as f64;
            let mut child = Vec::with_capacity(current.len());
            for dimension in 0..current.len() {
                let (lower, upper) = model.bounds_at(dimension);
                let second = unit(entropy);
                let third = unit(entropy);
                let fourth = unit(entropy);
                let width = width_of(lower, upper);
                let origin = origin_of(lower);

                let value = if second < wormhole {
                    let offset = travelling * (width * fourth + origin);
                    let best_position = to_f64(&ordered[0][dimension]);
                    if third < 0.5 {
                        best_position + offset
                    } else {
                        best_position - offset
                    }
                } else if second < inflation {
                    let donor = pick_donor(size, entropy);
                    to_f64(&ordered[donor][dimension])
                } else {
                    to_f64(&current[dimension])
                };
                child.push(clamp_at(model, dimension, value));
            }
            children.push(write_values(model, child));
        }
        children
    }
}

#[cfg(test)]
mod tests {
    use crate::solver::heuristic::entropy::SeededEntropy;
    use crate::solver::heuristic::iteration::Iteration;
    use crate::solver::heuristic::operator::evolution::ConstantSource;

    use super::super::test_support::{assert_dimension_and_bounds, model_with_bounds, solution};
    use super::super::SolutionUpdate;
    use super::{MultiVerseUpdate, SineCosineUpdate};

    fn setup() -> (
        super::super::test_support::TestModel,
        Vec<crate::model::callback::Solution<f64>>,
    ) {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![
            solution(&test, 4.0, 6.0),
            solution(&test, 1.0, 9.0),
            solution(&test, 8.0, 3.0),
        ];
        (test, parents)
    }

    #[test]
    fn sine_cosine_update_stays_within_bounds() {
        let (test, parents) = setup();
        let operator = SineCosineUpdate::new(Box::new(ConstantSource::new(2.0)));
        let children = operator.update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut SeededEntropy::new(51),
        );

        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);
    }

    #[test]
    fn swarm_updates_are_reproducible_for_the_same_seed() {
        let (test, parents) = setup();
        let iteration = Iteration::new();

        let sine_cosine = SineCosineUpdate::default();
        let first = sine_cosine.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(61),
        );
        let second = sine_cosine.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(61),
        );
        assert_eq!(first, second);

        let multi_verse = MultiVerseUpdate::default();
        let first = multi_verse.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(62),
        );
        let second = multi_verse.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(62),
        );
        assert_eq!(first, second);
        assert_dimension_and_bounds(&first, 0.0, 10.0);
    }
}
