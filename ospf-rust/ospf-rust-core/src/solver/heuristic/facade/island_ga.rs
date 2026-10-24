//! 岛模型遗传算法入口 / Island-model genetic algorithm entry

use crate::solver::heuristic::composition::population::{island, IslandConfig, IslandSolver};
use crate::solver::heuristic::entropy::{Entropy, SeededEntropy};
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::migration::RingMigration;
use crate::solver::heuristic::operator::mutation::GaussianMutation;
use crate::solver::heuristic::operator::stopping::{NeverStop, StoppingCriterion};
use crate::solver::heuristic::operator::survivor::ElitistSurvivor;
use crate::solver::heuristic::operator::update::ArithmeticCrossover;
use crate::solver::heuristic::runner::population_runner::PopulationRunner;

/// 岛屿规格：每个岛可以有自己的规模与变异率 / Island specification: each island may have its own size and mutation rate
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IslandSpec {
    /// 岛屿规模 / Island size
    pub size: usize,
    /// 岛屿变异率 / Island mutation rate
    pub mutation_rate: f64,
}

impl IslandSpec {
    /// 创建岛屿规格 / Create an island specification
    pub fn new(size: usize, mutation_rate: f64) -> Self {
        Self {
            size,
            mutation_rate,
        }
    }
}

impl Default for IslandSpec {
    fn default() -> Self {
        Self::new(20, 0.1)
    }
}

/// 岛模型遗传算法：若干个独立子种群各自演化，按周期做环形迁移 / Island-model genetic algorithm: independent sub-populations evolving separately with periodic ring migration
pub fn island_genetic<V>(
    islands: Vec<IslandSpec>,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
    migration_period: u64,
) -> IslandSolver<V>
where
    V: HeuristicValue,
{
    let runners: Vec<PopulationRunner<V>> = islands
        .into_iter()
        .map(|spec| {
            PopulationRunner::new(
                spec.size,
                Box::new(ArithmeticCrossover),
                Box::new(GaussianMutation::default()),
                Box::new(ElitistSurvivor),
                Box::new(SeededEntropy::default()),
            )
            .with_mutation_rate(spec.mutation_rate)
            // 岛屿的唯一终止判据是总迭代上限：单岛自身不设上限，否则先停的岛会冻结。
            // The only stopping criterion is the global iteration limit: an island itself has no limit, otherwise the first
            // island to stop would freeze.
            .with_core(crate::solver::heuristic::runner::runner_core::RunnerCore::new(
                Box::new(BestArchive::new(1)),
                Box::new(NeverStop),
            ))
        })
        .collect();

    island(
        "island-ga",
        IslandConfig {
            islands: runners,
            migration_period,
            migration: Box::new(RingMigration::default()),
            stopping,
            entropy,
        },
    )
}
