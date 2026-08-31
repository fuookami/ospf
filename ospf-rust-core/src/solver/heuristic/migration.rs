//! 迁移算子 / Migration Operators
//!
//! 定义岛屿模型中迁移操作的标准接口和多种实现，
//! 包括简单迁移、环形迁移和随机迁移。
//! Defines the standard interface and multiple implementations for migration operations
//! in the island model, including simple, ring, and random migration.

use super::{Individual, Population};

/// 迁移算子 trait / Migration Operator Trait
///
/// 定义迁移操作的标准接口，用于在岛屿模型的不同种群之间交换个体。
/// Defines the standard interface for migration operations, used to exchange individuals between different populations in the island model.
pub trait MigrationOperator<I, G>: Send + Sync
where
    I: Individual<G>,
{
    /// 执行迁移操作 / Perform migration
    ///
    /// 将个体从源种群迁移到目标种群。
    /// Migrates individuals from the source population to the target population.
    ///
    /// # 参数 / Parameters
    /// - `source`: 源种群 / Source population
    /// - `target`: 目标种群 / Target population
    /// - `count`: 迁移数量 / Number to migrate
    fn migrate(&self, source: &Population<I, G>, target: &mut Population<I, G>, count: usize);
}

/// 简单迁移 / Simple Migration
///
/// 将最优个体从源种群迁移到目标种群，可选择是否替换目标种群中最差的个体。
/// Migrates best individuals from source to target population, with an option to replace the worst individuals in the target population.
#[derive(Debug, Clone, Copy)]
pub struct SimpleMigration {
    /// 是否替换最差个体 / Whether to replace worst individuals
    pub replace_worst: bool,
}

impl SimpleMigration {
    /// 创建新迁移算子 / Create new migration operator
    ///
    /// # 参数 / Parameters
    /// - `replace_worst`: 是否替换目标种群中最差个体 / Whether to replace worst individuals in target population
    pub fn new(replace_worst: bool) -> Self {
        Self { replace_worst }
    }
}

impl Default for SimpleMigration {
    fn default() -> Self {
        Self::new(true)
    }
}

impl<I, G> MigrationOperator<I, G> for SimpleMigration
where
    I: Individual<G>,
{
    fn migrate(&self, source: &Population<I, G>, target: &mut Population<I, G>, count: usize) {
        // 获取源种群中最优的个体
        // Get best individuals from source population
        let mut migrants: Vec<I> = Vec::new();

        // 简化：复制源种群中前 count 个个体
        // Simplified: copy first count individuals from source
        for i in 0..count.min(source.len()) {
            if let Some(ind) = source.get(i).cloned() {
                migrants.push(ind);
            }
        }

        if self.replace_worst {
            // 替换目标种群中最差的个体
            // Replace worst individuals in target population
            // 简化：替换末尾的个体
            // Simplified: replace individuals at the end
            let start = target.len().saturating_sub(migrants.len());
            for (i, migrant) in migrants.into_iter().enumerate() {
                if start + i < target.len() {
                    if let Some(slot) = target.get_mut(start + i) {
                        *slot = migrant;
                    }
                } else {
                    target.add(migrant);
                }
            }
        } else {
            // 直接添加到目标种群
            // Directly add to target population
            for migrant in migrants {
                target.add(migrant);
            }
        }
    }
}

/// 环形迁移 / Ring Migration
///
/// 多个种群之间环形迁移，每个种群将个体迁移到下一个种群。
/// Ring migration among multiple populations, where each population migrates individuals to the next population.
#[derive(Debug, Clone, Copy)]
pub struct RingMigration {
    /// 迁移数量 / Number to migrate
    pub migration_count: usize,
}

impl RingMigration {
    /// 创建新环形迁移算子 / Create new ring migration operator
    ///
    /// # 参数 / Parameters
    /// - `migration_count`: 每次迁移的个体数量 / Number of individuals to migrate each time
    pub fn new(migration_count: usize) -> Self {
        Self { migration_count }
    }
}

impl Default for RingMigration {
    fn default() -> Self {
        Self::new(2)
    }
}

impl RingMigration {
    /// 执行环形迁移 / Perform ring migration
    ///
    /// 在多个种群之间执行环形迁移，每个种群将个体发送到下一个种群。
    /// Performs ring migration among multiple populations, where each population sends individuals to the next.
    ///
    /// # 参数 / Parameters
    /// - `populations`: 种群列表 / Population list
    pub fn migrate_ring<I, G>(&self, populations: &mut [Population<I, G>])
    where
        I: Individual<G>,
    {
        if populations.len() < 2 {
            return;
        }

        let n = populations.len();
        let mut migrants: Vec<Vec<I>> = Vec::with_capacity(n);

        // 收集每个种群中要迁移的个体
        // Collect individuals to migrate from each population
        for pop in populations.iter() {
            let mut pop_migrants = Vec::new();
            for i in 0..self.migration_count.min(pop.len()) {
                if let Some(ind) = pop.get(i).cloned() {
                    pop_migrants.push(ind);
                }
            }
            migrants.push(pop_migrants);
        }

        // 执行环形迁移：每个种群将个体迁移到下一个种群
        // Perform ring migration: each population migrates to the next
        for (i, migrant_list) in migrants.iter().enumerate() {
            let next_i = (i + 1) % n;

            for migrant in migrant_list {
                populations[next_i].add(migrant.clone());
            }
        }
    }
}

/// 随机迁移 / Random Migration
///
/// 随机选择个体进行迁移，按迁移概率间隔选取源种群中的个体。
/// Randomly selects individuals for migration, picking individuals from the source population at intervals based on the migration rate.
#[derive(Debug, Clone, Copy)]
pub struct RandomMigration {
    /// 迁移概率 / Migration probability
    pub migration_rate: f64,
}

impl RandomMigration {
    /// 创建新随机迁移算子 / Create new random migration operator
    ///
    /// # 参数 / Parameters
    /// - `migration_rate`: 迁移概率 / Migration probability
    pub fn new(migration_rate: f64) -> Self {
        Self { migration_rate }
    }
}

impl Default for RandomMigration {
    fn default() -> Self {
        Self::new(0.1)
    }
}

impl<I, G> MigrationOperator<I, G> for RandomMigration
where
    I: Individual<G>,
{
    fn migrate(&self, source: &Population<I, G>, target: &mut Population<I, G>, count: usize) {
        // 简化：每隔一定间隔选择个体
        // Simplified: select individuals at intervals
        let interval = (1.0 / self.migration_rate).max(1.0) as usize;

        for i in (0..source.len()).step_by(interval) {
            if target.len() >= count + source.len() / 2 {
                break;
            }
            if let Some(ind) = source.get(i).cloned() {
                target.add(ind);
            }
        }
    }
}
