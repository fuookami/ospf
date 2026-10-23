//! 和声搜索位置更新算子 / Harmony search position update operator
//!
//! 逐维决定新和声的分量来源：以 HMCR 概率从记忆库借用一个分量（再以 PAR 概率做音高微调），否则在整个取值域内
//! 随机生成。
//! The source of each harmony component is decided per dimension: with probability HMCR it is borrowed from the
//! harmony memory (and pitch-adjusted with probability PAR), otherwise generated uniformly over the domain.

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel, to_f64};
use crate::solver::heuristic::operator::unit;

use super::{SolutionUpdate, clamp_at, origin_of, read_values, width_of, write_values};

/// 和声搜索（HS）。 / Harmony search (HS).
///
/// HMCR 控制「利用」，PAR 控制「局部扰动强度」，二者共同决定探索与开发的比例。
/// HMCR controls exploitation and PAR controls the strength of local perturbation; together they set the
/// exploration/exploitation balance.
#[derive(Debug, Clone, Copy)]
pub struct HarmonySearchUpdate {
    /// 和声记忆库取用率 HMCR / Harmony memory consideration rate
    pub memory_consideration_rate: f64,
    /// 音高调整率 PAR / Pitch adjustment rate
    pub pitch_adjustment_rate: f64,
    /// 音高调整带宽，相对于变量取值范围 / Pitch adjustment bandwidth relative to the variable range
    pub bandwidth: f64,
}

impl HarmonySearchUpdate {
    /// 创建和声搜索更新 / Create a harmony search update
    pub fn new(
        memory_consideration_rate: f64,
        pitch_adjustment_rate: f64,
        bandwidth: f64,
    ) -> Self {
        Self {
            memory_consideration_rate,
            pitch_adjustment_rate,
            bandwidth,
        }
    }
}

impl Default for HarmonySearchUpdate {
    fn default() -> Self {
        Self::new(0.9, 0.3, 0.01)
    }
}

impl<V> SolutionUpdate<V> for HarmonySearchUpdate
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

        ordered
            .iter()
            .map(|current| {
                let dimension = current.len();
                let mut child = Vec::with_capacity(dimension);

                for (index, _) in current.iter().enumerate() {
                    let (lower, upper) = model.bounds_at(index);
                    let width = width_of(lower, upper);

                    let value = if unit(entropy) < self.memory_consideration_rate {
                        // 记忆库取用：从现有和声中随机取一个分量。
                        // Memory consideration: borrow a component from a random harmony.
                        let donor = ((unit(entropy) * size as f64) as usize).min(size - 1);
                        let mut borrowed = to_f64(&ordered[donor][index]);
                        if unit(entropy) < self.pitch_adjustment_rate {
                            let direction = if unit(entropy) < 0.5 { -1.0 } else { 1.0 };
                            borrowed += direction * self.bandwidth * width;
                        }
                        borrowed
                    } else {
                        // 随机生成：在整个取值域内均匀采样。
                        // Random generation: sample uniformly over the whole domain.
                        origin_of(lower) + unit(entropy) * width
                    };
                    child.push(clamp_at(model, index, value));
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
    use super::HarmonySearchUpdate;

    #[test]
    fn harmony_search_update_stays_within_bounds() {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![
            solution(&test, 5.0, 5.0),
            solution(&test, 1.0, 9.0),
            solution(&test, 9.0, 1.0),
        ];

        let children = HarmonySearchUpdate::default().update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut SeededEntropy::new(131),
        );

        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);
    }

    #[test]
    fn harmony_search_with_full_memory_rate_borrows_from_the_population() {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![solution(&test, 5.0, 5.0), solution(&test, 1.0, 9.0)];

        // HMCR = 1、PAR = 0 时分量必须来自父代 / With HMCR = 1 and PAR = 0 every component is borrowed from a parent.
        let operator = HarmonySearchUpdate::new(1.0, 0.0, 0.01);
        let children = operator.update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut SeededEntropy::new(132),
        );

        for child in &children {
            for value in child.values() {
                assert!(
                    *value == 1.0 || *value == 5.0 || *value == 9.0,
                    "borrowed value {value} must come from the harmony memory"
                );
            }
        }
    }
}
