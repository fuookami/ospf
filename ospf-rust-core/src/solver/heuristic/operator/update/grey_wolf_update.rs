//! 灰狼位置更新算子 / Grey wolf position update operator
//!
//! 按优度取前若干名作为 α / β / δ，每个个体被全部头狼共同牵引；父代已按优度升序，因此直接取前缀即可。
//! The best few individuals act as α / β / δ and every wolf is pulled by all leaders; parents are already ordered by
//! ascending quality, so a prefix is enough.

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel, to_f64};
use crate::solver::heuristic::operator::evolution::{LinearSource, ParameterSource};
use crate::solver::heuristic::operator::unit;

use super::{ParameterSlot, SolutionUpdate, clamp_at, read_values, write_values};

/// 灰狼位置更新（GWO）。 / Grey wolf position update (GWO).
///
/// X' = (1/k) Σ (X_leader − A · |C · X_leader − X|)
/// A = 2a·r₁ − a,  C = 2r₂
///
/// 其中 `a` 从 2 线性衰减到 0，使搜索由全局探索平滑过渡到局部开发。
/// Here `a` decays linearly from 2 to 0, smoothly shifting the search from exploration to exploitation.
pub struct GreyWolfUpdate {
    /// 收敛系数 a 的演化源 / Evolution source of the convergence coefficient a
    coefficient: ParameterSlot,
    /// 头狼数量 / Number of leaders
    pub leader_amount: usize,
}

impl GreyWolfUpdate {
    /// 创建灰狼更新 / Create a grey wolf update
    pub fn new(coefficient: Box<dyn ParameterSource>, leader_amount: usize) -> Self {
        Self {
            coefficient: ParameterSlot::new(coefficient),
            leader_amount,
        }
    }
}

impl Default for GreyWolfUpdate {
    fn default() -> Self {
        Self::new(Box::new(LinearSource::new(2.0, 0.0, 100)), 3)
    }
}

impl<V> SolutionUpdate<V> for GreyWolfUpdate
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
        let leader_count = self.leader_amount.max(1).min(ordered.len());
        let convergence = self.coefficient.value(iteration, entropy);

        ordered
            .iter()
            .map(|current| {
                let dimension = current.len();
                let mut accumulated = vec![0.0_f64; dimension];

                for leader in &ordered[..leader_count] {
                    let first_random = unit(entropy);
                    let second_random = unit(entropy);
                    let a = 2.0 * convergence * first_random - convergence;
                    let c = 2.0 * second_random;
                    for index in 0..dimension {
                        let target = to_f64(&leader[index]);
                        accumulated[index] +=
                            target - a * (c * target - to_f64(&current[index])).abs();
                    }
                }

                let scale = leader_count as f64;
                let child = (0..dimension)
                    .map(|index| clamp_at(model, index, accumulated[index] / scale))
                    .collect();
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
    use super::GreyWolfUpdate;

    #[test]
    fn grey_wolf_update_stays_within_bounds() {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![
            solution(&test, 5.0, 5.0),
            solution(&test, 1.0, 9.0),
            solution(&test, 9.0, 1.0),
            solution(&test, 3.0, 7.0),
        ];
        let operator = GreyWolfUpdate::new(Box::new(ConstantSource::new(1.0)), 3);

        let children = operator.update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut SeededEntropy::new(111),
        );

        assert_eq!(children.len(), parents.len());
        assert_dimension_and_bounds(&children, 0.0, 10.0);
    }

    #[test]
    fn grey_wolf_update_is_reproducible_for_the_same_seed() {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![
            solution(&test, 5.0, 5.0),
            solution(&test, 1.0, 9.0),
            solution(&test, 9.0, 1.0),
        ];
        let iteration = Iteration::new();
        let operator = GreyWolfUpdate::default();

        let first = operator.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(121),
        );
        let second = operator.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(121),
        );
        assert_eq!(first, second);
    }
}
