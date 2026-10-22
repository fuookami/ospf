//! 岛模型搜索骨架 / Island-model search skeleton
//!
//! 岛模型是对种群骨架的组合而非重写：每个岛屿就是一个 [`PopulationRunner`]，本骨架只负责「遍历岛屿」与「按周期
//! 触发迁移」两件事。
//! The island model composes the population skeleton instead of reimplementing it: each island is one
//! [`PopulationRunner`], and this skeleton owns only "iterate the islands" and "trigger migration on schedule".

use crate::solver::heuristic::contract::Runner;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel};
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::migration::{IslandMigration, NoMigration};
use crate::solver::heuristic::operator::stopping::NeverStop;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::drive::drive;
use crate::solver::heuristic::runner::population_runner::PopulationRunner;
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// 岛模型状态 / Island-model state
pub type Islands<V> = Vec<Vec<Individual<V, V>>>;

/// 岛模型搜索骨架 / Island-model search skeleton
pub struct IslandRunner<V>
where
    V: HeuristicValue,
{
    core: RunnerCore<V, V>,
    islands: Vec<PopulationRunner<V>>,
    migration_period: u64,
    migration: Box<dyn IslandMigration<V, V>>,
    entropy: Box<dyn Entropy>,
}

impl<V> IslandRunner<V>
where
    V: HeuristicValue,
{
    /// 创建岛模型骨架 / Create an island-model skeleton
    pub fn new(islands: Vec<PopulationRunner<V>>, entropy: Box<dyn Entropy>) -> Self {
        Self {
            core: RunnerCore::new(Box::new(BestArchive::new(1)), Box::new(NeverStop)),
            islands,
            migration_period: 5,
            migration: Box::new(NoMigration),
            entropy,
        }
    }

    /// 设置迁移周期 / Set the migration period
    pub fn with_migration_period(mut self, migration_period: u64) -> Self {
        self.migration_period = migration_period;
        self
    }

    /// 设置迁移算子 / Set the migration operator
    pub fn with_migration(mut self, migration: Box<dyn IslandMigration<V, V>>) -> Self {
        self.migration = migration;
        self
    }

    /// 设置搜索核心 / Set the search core
    pub fn with_core(mut self, core: RunnerCore<V, V>) -> Self {
        self.core = core;
        self
    }

    /// 搜索核心引用 / Search core reference
    pub fn core(&self) -> &RunnerCore<V, V> {
        &self.core
    }
}

impl<V> Runner<V, V> for IslandRunner<V>
where
    V: HeuristicValue,
{
    fn run<M>(&mut self, model: &mut M, iteration: &mut Iteration) -> Result<Vec<Individual<V, V>>>
    where
        M: ObjectiveModel<V, V>,
    {
        let Self {
            core,
            islands,
            migration_period,
            migration,
            entropy,
        } = self;

        let state: Islands<V> = islands
            .iter()
            .map(|island| island.initialize_island(model, entropy.as_mut()))
            .collect();

        drive(
            core,
            model,
            iteration,
            state,
            |model, iteration, state, entropy| {
                if state.is_empty() {
                    return Ok(state);
                }

                // 各岛屿独立推进一代；岛屿之间没有共享可变状态。
                // Each island advances one generation independently; islands share no mutable state.
                let advanced: Islands<V> = state
                    .iter()
                    .enumerate()
                    .map(|(index, island)| match islands.get(index) {
                        Some(runner) => runner.step_island(model, iteration, island, entropy),
                        None => island.clone(),
                    })
                    .collect();

                if *migration_period == 0
                    || iteration.iteration() % *migration_period != 0
                {
                    return Ok(advanced);
                }
                Ok(migration.migrate(iteration, advanced, model, entropy))
            },
            |state| state.iter().flatten().cloned().collect(),
            entropy.as_mut(),
        )
    }
}
