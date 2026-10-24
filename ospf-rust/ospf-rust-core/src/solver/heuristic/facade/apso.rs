//! 自适应粒子群入口 / Adaptive particle swarm entry

use crate::solver::heuristic::contract::RunnerSolver;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::evolution::LinearSource;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::runner::particle_swarm_runner::ParticleSwarmRunner;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// 自适应粒子群求解器类型别名 / Adaptive particle swarm solver type alias
pub type AdaptiveParticleSwarmSolver<V> = RunnerSolver<ParticleSwarmRunner<V>>;

/// 自适应粒子群：惯性权重与学习因子随迭代线性调度 / Adaptive particle swarm: inertia and learning factors scheduled linearly
///
/// `w` 由 0.9 降到 0.4，`c₁` 由 2.5 降到 0.5，`c₂` 由 0.5 升到 2.5：前期多信自己、多探索，后期多信群体、多开发。
/// `w` falls from 0.9 to 0.4, `c₁` from 2.5 to 0.5, and `c₂` rises from 0.5 to 2.5: trust yourself and explore early, trust the
/// swarm and exploit late.
pub fn adaptive_particle_swarm<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> AdaptiveParticleSwarmSolver<V>
where
    V: HeuristicValue,
{
    let core = RunnerCore::new(Box::new(BestArchive::new(1)), stopping);
    let runner = ParticleSwarmRunner::new(size, entropy)
        .with_inertia(Box::new(LinearSource::new(0.9, 0.4, 100)))
        .with_personal_weight(Box::new(LinearSource::new(2.5, 0.5, 100)))
        .with_social_weight(Box::new(LinearSource::new(0.5, 2.5, 100)))
        .with_core(core);
    RunnerSolver::new("apso", runner)
}
