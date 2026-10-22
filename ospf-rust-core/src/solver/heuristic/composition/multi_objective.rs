//! 多目标算法组合 / Multi-objective algorithm compositions
//!
//! 多目标没有单一目标值，因此这里不返回 [`crate::solver::heuristic::contract::RunnerSolver`]，而是直接构造多目标骨架，
//! 由 [`crate::solver::heuristic::result::MultiObjectiveSolver`] 输出整条前沿。
//! Multi-objective problems have no single objective value, so these factories build the multi-objective skeletons directly
//! instead of a `RunnerSolver`; `MultiObjectiveSolver` then returns the whole front.

use crate::model::callback::Solution;

use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::multi_objective::dominance::{Dominance, ParetoDominance};
use crate::solver::heuristic::operator::mutation::PolynomialMutation;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::operator::update::SimulatedBinaryCrossover;
use crate::solver::heuristic::runner::nsga2_runner::{Nsga2Runner, Nsga3Runner};

/// 组装 NSGA-II / Assemble NSGA-II
///
/// 交叉与变异默认取多目标场景的常用组合：SBX 交叉 + 多项式变异。
/// Crossover and mutation default to the usual multi-objective pair: SBX plus polynomial mutation.
pub fn nsga2<V>(
    size: usize,
    objectives_of: Box<dyn Fn(&Solution<V>) -> Vec<f64>>,
    dominance: Box<dyn Dominance>,
    entropy: Box<dyn Entropy>,
    stopping: Box<dyn StoppingCriterion>,
) -> Nsga2Runner<V>
where
    V: HeuristicValue,
{
    Nsga2Runner::new(
        size,
        objectives_of,
        dominance,
        Box::new(SimulatedBinaryCrossover::default()),
        Box::new(PolynomialMutation::default()),
        entropy,
        stopping,
    )
}

/// 以全最小化支配组装 NSGA-II / Assemble NSGA-II with all-minimisation dominance
pub fn nsga2_minimization<V>(
    size: usize,
    objective_amount: usize,
    objectives_of: Box<dyn Fn(&Solution<V>) -> Vec<f64>>,
    entropy: Box<dyn Entropy>,
    stopping: Box<dyn StoppingCriterion>,
) -> Nsga2Runner<V>
where
    V: HeuristicValue,
{
    nsga2(
        size,
        objectives_of,
        Box::new(ParetoDominance::minimization(objective_amount)),
        entropy,
        stopping,
    )
}

/// 组装 NSGA-III / Assemble NSGA-III
///
/// 参考点在 `divisions` 上做 Das–Dennis 构造：目标数 m、分割数 p 时参考点数量为 `C(p + m − 1, m − 1)`。
/// Reference points use the Das–Dennis construction over `divisions`: with `m` objectives and `p` divisions the count is
/// `C(p + m − 1, m − 1)`.
pub fn nsga3<V>(
    size: usize,
    objectives_of: Box<dyn Fn(&Solution<V>) -> Vec<f64>>,
    dominance: Box<dyn Dominance>,
    entropy: Box<dyn Entropy>,
    stopping: Box<dyn StoppingCriterion>,
    divisions: usize,
) -> Nsga3Runner<V>
where
    V: HeuristicValue,
{
    Nsga3Runner::new(
        size,
        objectives_of,
        dominance,
        Box::new(SimulatedBinaryCrossover::default()),
        Box::new(PolynomialMutation::default()),
        entropy,
        stopping,
        divisions,
    )
}

/// 以全最小化支配组装 NSGA-III / Assemble NSGA-III with all-minimisation dominance
pub fn nsga3_minimization<V>(
    size: usize,
    objective_amount: usize,
    objectives_of: Box<dyn Fn(&Solution<V>) -> Vec<f64>>,
    entropy: Box<dyn Entropy>,
    stopping: Box<dyn StoppingCriterion>,
    divisions: usize,
) -> Nsga3Runner<V>
where
    V: HeuristicValue,
{
    nsga3(
        size,
        objectives_of,
        Box::new(ParetoDominance::minimization(objective_amount)),
        entropy,
        stopping,
        divisions,
    )
}
