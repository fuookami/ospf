//! 环境选择算子 / Survivor selection operators

use super::compare_strictly;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel};

/// 环境选择算子契约 / Survivor selection contract
///
/// 决定父代与子代如何合并为下一代。实现一律用「逐个挑最优」而不是排序：模型比较器可能带容差、不具传递性，
/// 排序算法会因此得到不一致结果。
/// Decides how parents and offspring merge into the next generation. Implementations always pick the best repeatedly
/// instead of sorting: model comparators may be approximate and non-transitive, making sorting inconsistent.
pub trait SurvivorSelection<O, V>
where
    V: HeuristicValue,
{
    /// 选出进入下一代的个体 / Select the individuals entering the next generation
    fn select(
        &self,
        model: &dyn ObjectiveModel<O, V>,
        parents: &[Individual<O, V>],
        offspring: &[Individual<O, V>],
        amount: usize,
    ) -> Vec<Individual<O, V>>;
}

/// 逐个挑最优的 top-k 选择 / Top-k selection by repeatedly picking the best
pub fn best_of<O, V>(
    model: &dyn ObjectiveModel<O, V>,
    individuals: &[Individual<O, V>],
    amount: usize,
) -> Vec<Individual<O, V>>
where
    V: HeuristicValue,
    O: Clone,
{
    let mut remaining: Vec<Individual<O, V>> = individuals.to_vec();
    let mut kept = Vec::with_capacity(amount.min(remaining.len()));
    while kept.len() < amount && !remaining.is_empty() {
        let mut best_index = 0;
        for index in 1..remaining.len() {
            if compare_strictly(model, &remaining[index].fitness, &remaining[best_index].fitness)
                == std::cmp::Ordering::Less
            {
                best_index = index;
            }
        }
        kept.push(remaining.remove(best_index));
    }
    kept
}

/// (μ + λ) 环境选择：父代与子代一起竞争 / (μ + λ) survivor selection where parents and offspring compete together
#[derive(Debug, Clone, Copy, Default)]
pub struct ElitistSurvivor;

impl<O, V> SurvivorSelection<O, V> for ElitistSurvivor
where
    V: HeuristicValue,
    O: Clone,
{
    fn select(
        &self,
        model: &dyn ObjectiveModel<O, V>,
        parents: &[Individual<O, V>],
        offspring: &[Individual<O, V>],
        amount: usize,
    ) -> Vec<Individual<O, V>> {
        let mut pool = parents.to_vec();
        pool.extend_from_slice(offspring);
        best_of(model, &pool, amount)
    }
}

/// (μ, λ) 环境选择：只保留子代 / (μ, λ) survivor selection keeping only offspring
#[derive(Debug, Clone, Copy, Default)]
pub struct GenerationalSurvivor;

impl<O, V> SurvivorSelection<O, V> for GenerationalSurvivor
where
    V: HeuristicValue,
    O: Clone,
{
    fn select(
        &self,
        model: &dyn ObjectiveModel<O, V>,
        _parents: &[Individual<O, V>],
        offspring: &[Individual<O, V>],
        amount: usize,
    ) -> Vec<Individual<O, V>> {
        best_of(model, offspring, amount)
    }
}

/// 截断环境选择：合并后按优度截断 / Truncation survivor selection cutting the merged pool by quality
#[derive(Debug, Clone, Copy)]
pub struct TruncationSurvivor {
    /// 保留数量上限 / Upper bound on the number of survivors
    pub amount: usize,
}

impl TruncationSurvivor {
    /// 创建截断选择 / Create a truncation selection
    pub fn new(amount: usize) -> Self {
        Self { amount }
    }
}

impl<O, V> SurvivorSelection<O, V> for TruncationSurvivor
where
    V: HeuristicValue,
    O: Clone,
{
    fn select(
        &self,
        model: &dyn ObjectiveModel<O, V>,
        parents: &[Individual<O, V>],
        offspring: &[Individual<O, V>],
        amount: usize,
    ) -> Vec<Individual<O, V>> {
        let mut pool = parents.to_vec();
        pool.extend_from_slice(offspring);
        best_of(model, &pool, self.amount.min(amount))
    }
}

/// 精英保留环境选择：先保留最优父代，再用子代补足 / Elitist preserving selection keeping the best parents, then filling with offspring
#[derive(Debug, Clone, Copy)]
pub struct ElitistPreservingSurvivor {
    /// 精英父代数量 / Number of elite parents
    pub elite_amount: usize,
}

impl ElitistPreservingSurvivor {
    /// 创建精英保留选择 / Create an elitist preserving selection
    pub fn new(elite_amount: usize) -> Self {
        Self { elite_amount }
    }
}

impl<O, V> SurvivorSelection<O, V> for ElitistPreservingSurvivor
where
    V: HeuristicValue,
    O: Clone,
{
    fn select(
        &self,
        model: &dyn ObjectiveModel<O, V>,
        parents: &[Individual<O, V>],
        offspring: &[Individual<O, V>],
        amount: usize,
    ) -> Vec<Individual<O, V>> {
        let mut elites = best_of(model, parents, self.elite_amount.min(amount));
        let rest = best_of(model, offspring, amount.saturating_sub(elites.len()));
        elites.extend(rest);
        elites
    }
}
