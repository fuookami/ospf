//! 差分进化入口 / Differential evolution entry

use crate::solver::heuristic::composition::population::position_update;
use crate::solver::heuristic::composition::PopulationSolver;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::operator::update::DifferentialCrossover;

/// 差分进化：`V = X_r1 + F·(X_r2 − X_r3)` 与二项式交叉合成一次更新 / Differential evolution: `V = X_r1 + F·(X_r2 − X_r3)` and binomial crossover fused into one update
///
/// 差分进化没有独立的变异步骤，因此变异槽位留空。
/// Differential evolution has no separate mutation step, so the mutation slot stays empty.
pub fn differential_evolution<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "de",
        size,
        stopping,
        entropy,
        Box::new(DifferentialCrossover::default()),
    )
}
