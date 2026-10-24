//! 迭代观测钩子 / Iteration observer hook

use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::HeuristicValue;

/// 迭代观测钩子 / Iteration observer hook
///
/// 骨架持有循环与档案，因此只有它能回答「这一轮有没有变好」。需要奖励信号的自适应算子
/// （Q 学习、自适应步长等）通过本钩子拿到该信号，而不必自行重复计算收敛情况。
/// The skeleton owns the loop and the archive, so only it can answer "did this round improve". Adaptive operators that
/// need a reward signal (Q-learning, adaptive step sizes, …) obtain it here instead of recomputing convergence.
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 值类型 / Value type
pub trait IterationObserver<O, V>
where
    V: HeuristicValue,
{
    /// 观察一轮结果 / Observe the outcome of one round
    ///
    /// @param iteration 当前迭代（本轮推进前的计数）/ Current iteration (count before this round advanced)
    /// @param members 本轮成员 / Members of this round
    /// @param improved 档案最优是否被改善 / Whether the archive best improved
    fn observe(&mut self, iteration: &Iteration, members: &[Individual<O, V>], improved: bool);
}
