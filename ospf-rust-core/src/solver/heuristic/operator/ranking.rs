//! 排序算子 / Ranking operators

use super::super::individual::Individual;
use super::super::model::{HeuristicValue, ObjectiveModel};
use super::compare_strictly;

/// 排序算子契约 / Ranking contract
pub trait Ranking<O, V>
where
    V: HeuristicValue,
{
    /// 对个体排序 / Rank individuals
    fn rank(
        &self,
        model: &dyn ObjectiveModel<O, V>,
        individuals: &[Individual<O, V>],
    ) -> Vec<Individual<O, V>>;
}

/// 目标值升序排序（越靠前越优）/ Ascending objective ranking (better individuals first)
#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectiveRanking;

/// 目标值降序排序（越靠前越差）/ Descending objective ranking (worse individuals first)
#[derive(Debug, Clone, Copy, Default)]
pub struct ReversedObjectiveRanking;

/// 保持输入顺序 / Keep the input order
#[derive(Debug, Clone, Copy, Default)]
pub struct IdentityRanking;

impl<O, V> Ranking<O, V> for ObjectiveRanking
where
    V: HeuristicValue,
    O: Clone,
{
    fn rank(
        &self,
        model: &dyn ObjectiveModel<O, V>,
        individuals: &[Individual<O, V>],
    ) -> Vec<Individual<O, V>> {
        let mut ranked = individuals.to_vec();
        ranked.sort_by(|lhs, rhs| compare_strictly(model, &lhs.fitness, &rhs.fitness));
        ranked
    }
}

impl<O, V> Ranking<O, V> for ReversedObjectiveRanking
where
    V: HeuristicValue,
    O: Clone,
{
    fn rank(
        &self,
        model: &dyn ObjectiveModel<O, V>,
        individuals: &[Individual<O, V>],
    ) -> Vec<Individual<O, V>> {
        let mut ranked = individuals.to_vec();
        ranked.sort_by(|lhs, rhs| compare_strictly(model, &rhs.fitness, &lhs.fitness));
        ranked
    }
}

impl<O, V> Ranking<O, V> for IdentityRanking
where
    V: HeuristicValue,
    O: Clone,
{
    fn rank(
        &self,
        _model: &dyn ObjectiveModel<O, V>,
        individuals: &[Individual<O, V>],
    ) -> Vec<Individual<O, V>> {
        individuals.to_vec()
    }
}
