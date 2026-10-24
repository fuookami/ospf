//! 状态离散化 / State discretization

use crate::model::callback::Solution;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, to_f64};

/// 状态离散化契约：把连续搜索态势压成一个离散状态下标 / State discretization compressing the continuous search situation into a discrete state index
pub trait StateDiscretizer<V>
where
    V: HeuristicValue,
{
    /// 离散状态总数 / Total number of discrete states
    fn state_amount(&self) -> usize;

    /// 计算离散状态 / Compute the discrete state
    fn discretize(&self, iteration: &Iteration, members: &[Solution<V>]) -> usize;
}

/// 以「多样性 × 停滞度」为两轴的离散化 / Discretization over the two axes "diversity × stagnation"
#[derive(Debug, Clone, Copy)]
pub struct DiversityStagnationDiscretizer {
    /// 每个轴的层级数 / Number of levels per axis
    pub levels: usize,
}

impl DiversityStagnationDiscretizer {
    /// 创建离散化算子 / Create a discretizer
    pub fn new(levels: usize) -> Self {
        Self { levels }
    }
}

impl Default for DiversityStagnationDiscretizer {
    fn default() -> Self {
        Self::new(3)
    }
}

impl DiversityStagnationDiscretizer {
    fn level_of(&self, value: f64) -> usize {
        let width = 1.0 / self.levels as f64;
        ((value / width) as usize).min(self.levels - 1)
    }
}

impl<V> StateDiscretizer<V> for DiversityStagnationDiscretizer
where
    V: HeuristicValue,
{
    fn state_amount(&self) -> usize {
        if self.levels < 1 {
            0
        } else {
            self.levels * self.levels
        }
    }

    fn discretize(&self, iteration: &Iteration, members: &[Solution<V>]) -> usize {
        if self.levels < 2 || members.len() < 2 {
            return 0;
        }

        let mut dimension = 0;
        for member in members {
            if member.len() > dimension {
                dimension = member.len();
            }
        }
        if dimension == 0 {
            return 0;
        }

        let values: Vec<Vec<f64>> = members
            .iter()
            .map(|member| {
                // 按变量标识排序，保证不同解映射之间分量一一对应；`HashMap` 的迭代顺序不可依赖。
                // Sort by variable id so components correspond across distinct maps; `HashMap` iteration order is unreliable.
                let mut entries: Vec<(crate::variable::VariableId, f64)> = member
                    .iter()
                    .map(|(id, value)| (*id, to_f64(value)))
                    .collect();
                entries.sort_by_key(|(id, _)| *id);
                entries.into_iter().map(|(_, value)| value).collect()
            })
            .collect();

        let mut accumulated = 0.0;
        let mut maximum = 0.0;
        let mut pairs = 0_u64;
        for i in 0..values.len() {
            for j in (i + 1)..values.len() {
                let mut squared = 0.0;
                for k in 0..dimension {
                    let difference = values[i].get(k).copied().unwrap_or(0.0)
                        - values[j].get(k).copied().unwrap_or(0.0);
                    squared += difference * difference;
                }
                let distance = squared.sqrt();
                accumulated += distance;
                if distance > maximum {
                    maximum = distance;
                }
                pairs += 1;
            }
        }

        let diversity = if pairs == 0 || maximum <= 0.0 {
            0.0
        } else {
            (accumulated / pairs as f64) / maximum
        };

        let total = iteration.iteration() as f64;
        let stagnation = if total <= 0.0 {
            0.0
        } else {
            iteration.not_better_iteration() as f64 / total
        };

        self.level_of(diversity) * self.levels + self.level_of(stagnation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solution(values: &[f64]) -> Solution<f64> {
        let mut map = Solution::<f64>::new();
        for value in values {
            map.insert(crate::variable::new_standalone_id(), *value);
        }
        map
    }

    #[test]
    fn state_amount_is_levels_squared() {
        let small = DiversityStagnationDiscretizer::new(3);
        let none = DiversityStagnationDiscretizer::new(0);
        assert_eq!(StateDiscretizer::<f64>::state_amount(&small), 9);
        assert_eq!(StateDiscretizer::<f64>::state_amount(&none), 0);
    }

    #[test]
    fn collapse_and_early_iteration_map_to_low_state() {
        let discretizer = DiversityStagnationDiscretizer::new(3);
        let member = solution(&[1.0, 2.0]);
        let members = vec![member.clone(), member];
        assert_eq!(discretizer.discretize(&Iteration::new(), &members), 0);
    }
}
