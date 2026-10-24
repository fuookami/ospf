//! CMA-ES 入口 / CMA-ES entry

use crate::solver::heuristic::contract::RunnerSolver;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::runner::cmaes_runner::CmaEsRunner;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// CMA-ES 求解器类型别名 / CMA-ES solver type alias
pub type CmaEsSolver<V> = RunnerSolver<CmaEsRunner<V>>;

/// 协方差矩阵自适应进化策略 / Covariance matrix adaptation evolution strategy
pub fn cma_es<V>(
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> CmaEsSolver<V>
where
    V: HeuristicValue,
{
    let core = RunnerCore::new(Box::new(BestArchive::new(1)), stopping);
    RunnerSolver::new("cma-es", CmaEsRunner::new(entropy, 0.3).with_core(core))
}
