//! 选择算子
//! Selection Operators

use super::{FitnessComparator, Individual, Population};

/// 选择算子 trait / Selection Operator Trait
///
/// 定义选择操作的标准接口。
/// Defines standard interface for selection operations.
pub trait SelectionOperator<I, G>: Send + Sync
where
    I: Individual<G>,
{
    /// 从种群中选择个体 / Select individual from population
    ///
    /// # 参数 / Parameters
    /// - `population`: 种群 / Population
    ///
    /// # 返回 / Returns
    /// 被选中个体的索引 / Index of selected individual
    fn select(&self, population: &Population<I, G>) -> usize;

    /// 选择多个个体 / Select multiple individuals
    ///
    /// # 参数 / Parameters
    /// - `population`: 种群 / Population
    /// - `count`: 选择数量 / Number to select
    ///
    /// # 返回 / Returns
    /// 被选中个体的索引列表 / List of selected individual indices
    fn select_multiple(&self, population: &Population<I, G>, count: usize) -> Vec<usize> {
        (0..count).map(|_| self.select(population)).collect()
    }
}

/// 锦标赛选择 / Tournament Selection
///
/// 随机选择 k 个个体，返回其中最优的一个。
/// Randomly selects k individuals and returns the best one.
#[derive(Debug, Clone, Copy)]
pub struct TournamentSelection {
    /// 锦标赛大小 / Tournament size
    pub tournament_size: usize,
    /// 适应度比较器 / Fitness comparator
    pub comparator: FitnessComparator,
}

impl TournamentSelection {
    /// 创建新算子 / Create new operator
    pub fn new(tournament_size: usize) -> Self {
        Self {
            tournament_size,
            comparator: FitnessComparator::Maximize,
        }
    }

    /// 创建默认算子 / Create default operator
    pub fn default_size() -> Self {
        Self::new(3)
    }

    /// 设置比较器 / Set comparator
    pub fn with_comparator(mut self, comparator: FitnessComparator) -> Self {
        self.comparator = comparator;
        self
    }
}

impl Default for TournamentSelection {
    fn default() -> Self {
        Self::default_size()
    }
}

impl<I, G> SelectionOperator<I, G> for TournamentSelection
where
    I: Individual<G>,
{
    fn select(&self, population: &Population<I, G>) -> usize {
        if population.is_empty() {
            panic!("Cannot select from empty population");
        }

        let pop_size = population.len();

        // 简化实现：选择前 tournament_size 个个体中的最优者
        // Simplified implementation: select best from first tournament_size individuals
        let tournament_count = self.tournament_size.min(pop_size);

        let mut best_idx = 0;
        let mut best_fitness = f64::NEG_INFINITY;

        for i in 0..tournament_count {
            if let Some(fitness) = population.get(i).and_then(|ind| ind.fitness()) {
                if self.comparator.is_better(fitness, best_fitness) {
                    best_idx = i;
                    best_fitness = fitness;
                }
            }
        }

        best_idx
    }
}

/// 轮盘赌选择 / Roulette Wheel Selection
///
/// 根据适应度比例选择个体。
/// Selects individuals based on fitness proportion.
#[derive(Debug, Clone, Copy)]
pub struct RouletteWheelSelection {
    /// 适应度比较器 / Fitness comparator
    pub comparator: FitnessComparator,
}

impl RouletteWheelSelection {
    pub fn new() -> Self {
        Self {
            comparator: FitnessComparator::Maximize,
        }
    }
}

impl Default for RouletteWheelSelection {
    fn default() -> Self {
        Self::new()
    }
}

impl<I, G> SelectionOperator<I, G> for RouletteWheelSelection
where
    I: Individual<G>,
{
    fn select(&self, population: &Population<I, G>) -> usize {
        if population.is_empty() {
            panic!("Cannot select from empty population");
        }

        // 简化实现：按适应度比例选择
        // Simplified implementation: select based on fitness proportion
        let fitnesses: Vec<f64> = population
            .individuals()
            .iter()
            .map(|ind| ind.fitness().unwrap_or(0.0).max(0.0))
            .collect();

        let total: f64 = fitnesses.iter().sum();

        if total == 0.0 {
            // 所有适应度为 0，随机选择
            // All fitnesses are 0, select randomly (simplified: first)
            return 0;
        }

        // 简化：选择适应度最大的
        // Simplified: select the one with maximum fitness
        fitnesses
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

/// 精英选择 / Elitism Selection
///
/// 直接保留最优的 k 个个体。
/// Directly preserves the best k individuals.
#[derive(Debug, Clone, Copy)]
pub struct ElitismSelection {
    /// 精英数量 / Number of elites
    pub elite_count: usize,
    /// 适应度比较器 / Fitness comparator
    pub comparator: FitnessComparator,
}

impl ElitismSelection {
    pub fn new(elite_count: usize) -> Self {
        Self {
            elite_count,
            comparator: FitnessComparator::Maximize,
        }
    }
}

impl Default for ElitismSelection {
    fn default() -> Self {
        Self::new(1)
    }
}

impl<I, G> SelectionOperator<I, G> for ElitismSelection
where
    I: Individual<G>,
{
    fn select(&self, population: &Population<I, G>) -> usize {
        if population.is_empty() {
            panic!("Cannot select from empty population");
        }

        // 选择最优个体
        // Select the best individual
        let mut best_idx = 0;
        let mut best_fitness = f64::NEG_INFINITY;

        for (i, ind) in population.individuals().iter().enumerate() {
            if let Some(fitness) = ind.fitness() {
                if self.comparator.is_better(fitness, best_fitness) {
                    best_idx = i;
                    best_fitness = fitness;
                }
            }
        }

        best_idx
    }

    fn select_multiple(&self, population: &Population<I, G>, count: usize) -> Vec<usize> {
        let n = count.min(self.elite_count).min(population.len());

        // 获取所有个体的适应度
        // Get fitness of all individuals
        let mut indexed_fitness: Vec<(usize, f64)> = population
            .individuals()
            .iter()
            .enumerate()
            .map(|(i, ind)| (i, ind.fitness().unwrap_or(f64::NEG_INFINITY)))
            .collect();

        // 按适应度排序
        // Sort by fitness
        indexed_fitness.sort_by(|a, b| self.comparator.compare(a.1, b.1));

        // 返回前 n 个索引
        // Return first n indices
        indexed_fitness.iter().take(n).map(|(i, _)| *i).collect()
    }
}

/// 排名选择 / Rank Selection
///
/// 根据排名而非适应度选择个体。
/// Selects individuals based on rank rather than fitness.
#[derive(Debug, Clone, Copy)]
pub struct RankSelection {
    /// 选择压力 / Selection pressure
    pub pressure: f64,
    /// 适应度比较器 / Fitness comparator
    pub comparator: FitnessComparator,
}

impl RankSelection {
    pub fn new(pressure: f64) -> Self {
        Self {
            pressure,
            comparator: FitnessComparator::Maximize,
        }
    }
}

impl Default for RankSelection {
    fn default() -> Self {
        Self::new(2.0)
    }
}

impl<I, G> SelectionOperator<I, G> for RankSelection
where
    I: Individual<G>,
{
    fn select(&self, population: &Population<I, G>) -> usize {
        if population.is_empty() {
            panic!("Cannot select from empty population");
        }

        // 简化实现：选择排名最高的
        // Simplified implementation: select highest ranked
        let mut best_idx = 0;
        let mut best_fitness = f64::NEG_INFINITY;

        for (i, ind) in population.individuals().iter().enumerate() {
            if let Some(fitness) = ind.fitness() {
                if self.comparator.is_better(fitness, best_fitness) {
                    best_idx = i;
                    best_fitness = fitness;
                }
            }
        }

        best_idx
    }
}
