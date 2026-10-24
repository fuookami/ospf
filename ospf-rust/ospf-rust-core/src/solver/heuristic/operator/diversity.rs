//! 多样性算子 / Diversity operators

use ospf_rust_math::fractal::correlation_dimension;

use crate::solver::heuristic::model::{HeuristicValue, to_f64};
use crate::variable::VariableId;

use super::value_at;

/// 多样性度量契约，用于监测种群是否塌缩 / Diversity metric contract used to detect population collapse
pub trait DiversityMetric<V>
where
    V: HeuristicValue,
{
    /// 计算种群多样性 / Compute population diversity
    fn measure(&self, solutions: &[crate::model::callback::Solution<V>], order: &[VariableId])
        -> f64;
}

/// 平均成对欧氏距离 / Mean pairwise Euclidean distance
#[derive(Debug, Clone, Copy, Default)]
pub struct MeanPairwiseDistance;

impl<V> DiversityMetric<V> for MeanPairwiseDistance
where
    V: HeuristicValue,
{
    fn measure(
        &self,
        solutions: &[crate::model::callback::Solution<V>],
        order: &[VariableId],
    ) -> f64 {
        if solutions.len() < 2 {
            return 0.0;
        }

        let dimension = solutions[0].len();
        let mut accumulated = 0.0;
        let mut pairs = 0_u64;
        for i in 0..solutions.len() {
            for j in (i + 1)..solutions.len() {
                let mut squared = 0.0;
                for k in 0..dimension {
                    let left = value_at(&solutions[i], order, k)
                        .map(|value| to_f64(&value))
                        .unwrap_or(f64::NAN);
                    let right = value_at(&solutions[j], order, k)
                        .map(|value| to_f64(&value))
                        .unwrap_or(f64::NAN);
                    let difference = left - right;
                    squared += difference * difference;
                }
                accumulated += squared.sqrt();
                pairs += 1;
            }
        }

        if pairs == 0 {
            0.0
        } else {
            accumulated / pairs as f64
        }
    }
}

/// 用相关维数（分形维数）度量种群多样性 / Measure diversity by the correlation dimension
#[derive(Debug, Clone, Copy, Default)]
pub struct FractalDimensionDiversity;

impl<V> DiversityMetric<V> for FractalDimensionDiversity
where
    V: HeuristicValue,
{
    fn measure(
        &self,
        solutions: &[crate::model::callback::Solution<V>],
        order: &[VariableId],
    ) -> f64 {
        let converted: Vec<Vec<f64>> = solutions
            .iter()
            .map(|solution| {
                (0..order.len())
                    .map(|index| {
                        value_at(solution, order, index)
                            .map(|value| to_f64(&value))
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();
        correlation_dimension(&converted).unwrap_or(0.0)
    }
}
