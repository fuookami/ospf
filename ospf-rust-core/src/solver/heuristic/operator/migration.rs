//! 岛屿迁移算子 / Island migration operators

use super::compare_strictly;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel};

/// 岛屿迁移契约 / Island migration contract
pub trait IslandMigration<O, V>
where
    V: HeuristicValue,
{
    /// 执行迁移并返回新的岛屿集合 / Perform migration and return the new islands
    fn migrate(
        &self,
        iteration: &Iteration,
        islands: Vec<Vec<Individual<O, V>>>,
        model: &dyn ObjectiveModel<O, V>,
        entropy: &mut dyn Entropy,
    ) -> Vec<Vec<Individual<O, V>>>;
}

/// 逐个挑最优的升序排列，避免依赖比较器的传递性 / Ascending ranking by repeatedly picking the best, avoiding comparator transitivity
fn rank_ascending<O, V>(
    island: &[Individual<O, V>],
    model: &dyn ObjectiveModel<O, V>,
) -> Vec<Individual<O, V>>
where
    V: HeuristicValue,
    O: Clone,
{
    let mut remaining = island.to_vec();
    let mut ranked = Vec::with_capacity(remaining.len());
    while !remaining.is_empty() {
        let mut best_index = 0;
        for index in 1..remaining.len() {
            if compare_strictly(model, &remaining[index].fitness, &remaining[best_index].fitness)
                == std::cmp::Ordering::Less
            {
                best_index = index;
            }
        }
        ranked.push(remaining.remove(best_index));
    }
    ranked
}

/// 不迁移 / No migration
#[derive(Debug, Clone, Copy, Default)]
pub struct NoMigration;

impl<O, V> IslandMigration<O, V> for NoMigration
where
    V: HeuristicValue,
{
    fn migrate(
        &self,
        _iteration: &Iteration,
        islands: Vec<Vec<Individual<O, V>>>,
        _model: &dyn ObjectiveModel<O, V>,
        _entropy: &mut dyn Entropy,
    ) -> Vec<Vec<Individual<O, V>>> {
        islands
    }
}

/// 环形迁移：每个岛屿把若干个最优个体送给下一个岛屿，替换对方最差的个体 / Ring migration sending each island's best individuals to the next island, replacing its worst
#[derive(Debug, Clone, Copy)]
pub struct RingMigration {
    /// 每个岛屿外迁的个体比例 / Fraction of each island's individuals that migrate
    pub rate: f64,
}

impl RingMigration {
    /// 创建环形迁移 / Create a ring migration
    pub fn new(rate: f64) -> Self {
        Self { rate }
    }
}

impl Default for RingMigration {
    fn default() -> Self {
        Self::new(0.1)
    }
}

impl<O, V> IslandMigration<O, V> for RingMigration
where
    V: HeuristicValue,
    O: Clone,
{
    fn migrate(
        &self,
        _iteration: &Iteration,
        islands: Vec<Vec<Individual<O, V>>>,
        model: &dyn ObjectiveModel<O, V>,
        _entropy: &mut dyn Entropy,
    ) -> Vec<Vec<Individual<O, V>>> {
        if islands.len() < 2 {
            return islands;
        }

        let ranked: Vec<Vec<Individual<O, V>>> = islands
            .iter()
            .map(|island| rank_ascending(island, model))
            .collect();
        let island_amount = islands.len();

        islands
            .iter()
            .enumerate()
            .map(|(index, original)| {
                let incoming = &ranked[(index + island_amount - 1) % island_amount];
                let local = &ranked[index];
                let amount = ((local.len() as f64 * self.rate) as usize)
                    .max(1)
                    .min(local.len().min(incoming.len()));
                if amount == 0 {
                    return original.clone();
                }
                let mut merged: Vec<Individual<O, V>> =
                    local[..local.len() - amount].to_vec();
                merged.extend(incoming[..amount].iter().cloned());
                merged
            })
            .collect()
    }
}

/// 优岛到劣岛迁移：表现最好的岛屿把最优个体直接注入表现最差的岛屿 / Best-to-worst migration injecting the best island's leaders into the worst island
#[derive(Debug, Clone, Copy)]
pub struct BestToWorstMigration {
    /// 每次迁移的个体数量 / Number of individuals per migration
    pub amount: usize,
}

impl BestToWorstMigration {
    /// 创建优岛到劣岛迁移 / Create a best-to-worst migration
    pub fn new(amount: usize) -> Self {
        Self { amount }
    }
}

impl Default for BestToWorstMigration {
    fn default() -> Self {
        Self::new(1)
    }
}

impl<O, V> IslandMigration<O, V> for BestToWorstMigration
where
    V: HeuristicValue,
    O: Clone,
{
    fn migrate(
        &self,
        _iteration: &Iteration,
        islands: Vec<Vec<Individual<O, V>>>,
        model: &dyn ObjectiveModel<O, V>,
        _entropy: &mut dyn Entropy,
    ) -> Vec<Vec<Individual<O, V>>> {
        if islands.len() < 2 || islands.iter().any(|island| island.is_empty()) {
            return islands;
        }

        let ranked: Vec<Vec<Individual<O, V>>> = islands
            .iter()
            .map(|island| rank_ascending(island, model))
            .collect();
        let leaders: Vec<&Individual<O, V>> =
            ranked.iter().map(|island| &island[0]).collect();

        // 逐个挑选最优/最差岛屿，避免依赖比较器的传递性。
        // Pick the best and worst islands one by one instead of relying on comparator transitivity.
        let mut source = 0;
        let mut target = 0;
        for index in 0..leaders.len() {
            if compare_strictly(model, &leaders[index].fitness, &leaders[source].fitness)
                == std::cmp::Ordering::Less
            {
                source = index;
            }
            if compare_strictly(model, &leaders[index].fitness, &leaders[target].fitness)
                == std::cmp::Ordering::Greater
            {
                target = index;
            }
        }
        if source == target {
            return islands;
        }

        let migrant_amount = self.amount.min(ranked[source].len());
        if migrant_amount == 0 {
            return islands;
        }
        let migrants = ranked[source][..migrant_amount].to_vec();

        islands
            .into_iter()
            .enumerate()
            .map(|(index, original)| {
                if index != target {
                    original
                } else {
                    let local = &ranked[target];
                    let keep = local
                        .len()
                        .saturating_sub(migrant_amount.min(local.len()));
                    let mut merged: Vec<Individual<O, V>> = local[..keep].to_vec();
                    merged.extend(migrants.iter().cloned());
                    merged
                }
            })
            .collect()
    }
}
