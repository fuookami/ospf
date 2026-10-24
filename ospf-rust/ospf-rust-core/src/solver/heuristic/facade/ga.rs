//! 遗传算法入口 / Genetic algorithm entry

use crate::solver::heuristic::composition::population::genetic as compose;
use crate::solver::heuristic::composition::PopulationSolver;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;

/// 遗传算法，即种群骨架 + 单点交叉 + 高斯变异 + 精英环境选择 / Genetic algorithm: population skeleton + single-point crossover + Gaussian mutation + elitist survivor
pub fn genetic<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    compose(size, stopping, entropy)
}
