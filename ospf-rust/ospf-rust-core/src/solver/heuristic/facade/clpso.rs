//! 综合学习粒子群入口 / Comprehensive learning particle swarm entry

use crate::solver::heuristic::contract::RunnerSolver;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::runner::clpso_runner::{ClpsoConfig, ClpsoRunner};
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// CLPSO 求解器类型别名 / CLPSO solver type alias
pub type ClpsoSolver<V> = RunnerSolver<ClpsoRunner<V>>;

/// 综合学习粒子群：每个维度各自选择学习对象，避免整群塌缩到同一点 / Comprehensive learning particle swarm where every dimension picks its own exemplar
pub fn comprehensive_learning_particle_swarm<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> ClpsoSolver<V>
where
    V: HeuristicValue,
{
    let core = RunnerCore::new(Box::new(BestArchive::new(1)), stopping);
    RunnerSolver::new(
        "clpso",
        ClpsoRunner::new(ClpsoConfig::new(size), entropy).with_core(core),
    )
}
