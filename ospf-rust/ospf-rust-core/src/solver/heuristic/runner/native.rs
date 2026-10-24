//! 可步进的原生生命周期 / Stepwise native lifecycle

use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel};
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// 完整求解与分片会话共享的原生钩子 / Native hooks shared by complete solves and sliced sessions
pub trait NativeRunner<V: HeuristicValue> {
    /// 原生算法状态 / Native algorithm state
    type State: Clone;

    /// 初始化，不运行完整求解循环 / Initialize without running a complete solve loop
    fn initialize_native<M>(&mut self, model: &M, entropy: &mut dyn Entropy) -> Result<Self::State>
    where
        M: ObjectiveModel<V, V>;

    /// 从上一完整状态推进一步 / Advance from the last complete state
    fn advance_native<M>(
        &mut self,
        model: &M,
        iteration: &Iteration,
        state: Self::State,
        entropy: &mut dyn Entropy,
    ) -> Result<Self::State>
    where
        M: ObjectiveModel<V, V>;

    /// 提取原生状态中的个体 / Extract individuals from native state
    fn members_native(state: &Self::State) -> Vec<Individual<V, V>>;

    /// 完整求解使用的档案与终止配置 / Archive and stopping configuration for complete solves
    fn core_native(&mut self) -> &mut RunnerCore<V, V>;
}

/// 驱动原生完整求解，复用会话所用的钩子 / Drive a complete native solve using the same hooks as sessions
pub fn run_native<R, M, V>(
    runner: &mut R,
    model: &mut M,
    iteration: &mut Iteration,
    entropy: &mut dyn Entropy,
) -> Result<Vec<Individual<V, V>>>
where
    R: NativeRunner<V>,
    M: ObjectiveModel<V, V>,
    V: HeuristicValue,
{
    let mut state = runner.initialize_native(model, entropy)?;
    runner.core_native().archive.update(model, R::members_native(&state));
    iteration.next(true);
    while !runner.core_native().stopping.should_stop(iteration) {
        state = runner.advance_native(model, iteration, state, entropy)?;
        let members = R::members_native(&state);
        let core = runner.core_native();
        let improved = core.archive.update(model, members.clone());
        for observer in &mut core.observers {
            observer.observe(iteration, &members, improved);
        }
        model.flush();
        iteration.next(improved);
    }
    Ok(runner.core_native().archive.individuals().to_vec())
}
