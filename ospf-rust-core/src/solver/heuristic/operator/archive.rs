//! 档案算子 / Archive operators

use super::compare_strictly;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel};

/// 档案契约 / Archive contract
///
/// 维护「当前最优集合」，是单目标与多目标共享的收敛载体，也是「本轮是否改进」信号的唯一来源。
/// Maintains the current best set; it is the convergence carrier shared by single- and multi-objective search and the
/// only source of the "did this round improve" signal.
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 值类型 / Value type
pub trait Archive<O, V>
where
    V: HeuristicValue,
{
    /// 当前档案个体，按优度排列 / Individuals currently in the archive, ordered by quality
    fn individuals(&self) -> &[Individual<O, V>];

    /// 用新个体更新档案 / Update the archive with new individuals
    ///
    /// @return 档案最优是否被改善 / Whether the archive best improved
    fn update(&mut self, model: &dyn ObjectiveModel<O, V>, candidates: Vec<Individual<O, V>>) -> bool;

    /// 清空档案 / Clear the archive
    fn clear(&mut self);
}

/// 精英档案，保留固定数量的最优个体 / Elite archive keeping a fixed number of best individuals
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 值类型 / Value type
pub struct BestArchive<O, V>
where
    V: HeuristicValue,
{
    amount: usize,
    entries: Vec<Individual<O, V>>,
}

impl<O, V> BestArchive<O, V>
where
    V: HeuristicValue,
{
    /// 创建精英档案 / Create an elite archive
    pub fn new(amount: usize) -> Self {
        Self {
            amount,
            entries: Vec::new(),
        }
    }

    /// 档案容量 / Archive capacity
    pub fn amount(&self) -> usize {
        self.amount
    }
}

impl<O, V> Archive<O, V> for BestArchive<O, V>
where
    V: HeuristicValue,
    O: Clone,
{
    fn individuals(&self) -> &[Individual<O, V>] {
        &self.entries
    }

    fn update(
        &mut self,
        model: &dyn ObjectiveModel<O, V>,
        candidates: Vec<Individual<O, V>>,
    ) -> bool {
        let previous_best = self.entries.first().cloned();
        self.entries.extend(candidates);
        self.entries.sort_by(|lhs, rhs| {
            compare_strictly(model, &lhs.fitness, &rhs.fitness)
        });
        self.entries.truncate(self.amount);

        match (previous_best, self.entries.first()) {
            (None, Some(_)) => true,
            (Some(previous), Some(current)) => {
                compare_strictly(model, &current.fitness, &previous.fitness) == std::cmp::Ordering::Less
            }
            _ => false,
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

/// 历史最优档案，只保留迄今为止的最优个体 / Historical best archive keeping only the best individual so far
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 值类型 / Value type
pub struct HistoryBestArchive<O, V>
where
    V: HeuristicValue,
{
    entry: Option<Individual<O, V>>,
}

impl<O, V> HistoryBestArchive<O, V>
where
    V: HeuristicValue,
{
    /// 创建历史最优档案 / Create a historical best archive
    pub fn new() -> Self {
        Self { entry: None }
    }
}

impl<O, V> Default for HistoryBestArchive<O, V>
where
    V: HeuristicValue,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<O, V> Archive<O, V> for HistoryBestArchive<O, V>
where
    V: HeuristicValue,
    O: Clone,
{
    fn individuals(&self) -> &[Individual<O, V>] {
        match &self.entry {
            Some(entry) => std::slice::from_ref(entry),
            None => &[],
        }
    }

    fn update(
        &mut self,
        model: &dyn ObjectiveModel<O, V>,
        candidates: Vec<Individual<O, V>>,
    ) -> bool {
        let mut improved = false;
        for candidate in candidates {
            let better = match &self.entry {
                None => true,
                Some(current) => {
                    compare_strictly(model, &candidate.fitness, &current.fitness)
                        == std::cmp::Ordering::Less
                }
            };
            if better {
                self.entry = Some(candidate);
                improved = true;
            }
        }
        improved
    }

    fn clear(&mut self) {
        self.entry = None;
    }
}
