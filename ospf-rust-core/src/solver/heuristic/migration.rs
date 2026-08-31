//! 迁移算子
//! Migration Operators

use super::{Individual, Population};

/// 迁移算子 trait / Migration Operator Trait
///
/// 定义迁移操作的标准接口。
/// Defines standard interface for migration operations.
pub trait MigrationOperator<I, G>: Send + Sync
where
    I: Individual<G>,
{
    /// 执行迁移操作 / Perform migration
    ///
    /// # 参数 / Parameters
    /// - `source`: 源种群 / Source population
    /// - `target`: 目标种群 / Target population
    /// - `count`: 迁移数量 / Number to migrate
    fn migrate(&self, source: &Population<I, G>, target: &mut Population<I, G>, count: usize);
}

/// 简单迁移 / Simple Migration
///
/// 将最优个体从源种群迁移到目标种群。
/// Migrates best individuals from source to target population.
#[derive(Debug, Clone, Copy)]
pub struct SimpleMigration {
    /// 是否替换最差个体 / Whether to replace worst individuals
    pub replace_worst: bool,
}

impl SimpleMigration {
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
/// 多个种群之间环形迁移。
/// Ring migration among multiple populations.
#[derive(Debug, Clone, Copy)]
pub struct RingMigration {
    /// 迁移数量 / Number to migrate
    pub migration_count: usize,
}

impl RingMigration {
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
        for i in 0..n {
            let next_i = (i + 1) % n;
            let migrant_list = &migrants[i];

            for migrant in migrant_list {
                populations[next_i].add(migrant.clone());
            }
        }
    }
}

/// 随机迁移 / Random Migration
///
/// 随机选择个体进行迁移。
/// Randomly selects individuals for migration.
#[derive(Debug, Clone, Copy)]
pub struct RandomMigration {
    /// 迁移概率 / Migration probability
    pub migration_rate: f64,
}

impl RandomMigration {
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
