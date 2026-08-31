//! 种群结构
//! Population Structure

use std::fmt::Debug;
use super::{FitnessComparator, Individual, SolutionWithFitness};

/// 种群 / Population
///
/// 管理一组个体，支持排序、选择等操作。
/// Manages a group of individuals, supporting sorting, selection, etc.
///
/// # 类型参数 / Type Parameters
/// - `I`: 个体类型 / Individual type
/// - `G`: 基因类型 / Gene type
#[derive(Debug, Clone)]
pub struct Population<I, G>
where
    I: Individual<G>,
{
    /// 个体列表 / Individual list
    individuals: Vec<I>,
    /// 适应度比较器 / Fitness comparator
    comparator: FitnessComparator,
    /// 基因类型标记 / Gene type marker
    _gene: std::marker::PhantomData<G>,
}

impl<I, G> Population<I, G>
where
    I: Individual<G>,
{
    /// 创建空种群 / Create empty population
    pub fn new() -> Self {
        Self {
            individuals: Vec::new(),
            comparator: FitnessComparator::Maximize,
            _gene: std::marker::PhantomData,
        }
    }

    /// 创建指定容量的种群 / Create population with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            individuals: Vec::with_capacity(capacity),
            comparator: FitnessComparator::Maximize,
            _gene: std::marker::PhantomData,
        }
    }

    /// 创建带比较器的种群 / Create population with comparator
    pub fn with_comparator(comparator: FitnessComparator) -> Self {
        Self {
            individuals: Vec::new(),
            comparator,
            _gene: std::marker::PhantomData,
        }
    }

    /// 从个体列表创建种群 / Create population from individual list
    pub fn from_individuals(individuals: Vec<I>) -> Self {
        Self {
            individuals,
            comparator: FitnessComparator::Maximize,
            _gene: std::marker::PhantomData,
        }
    }

    /// 添加个体 / Add individual
    pub fn add(&mut self, individual: I) {
        self.individuals.push(individual);
    }

    /// 批量添加个体 / Add individuals in batch
    pub fn extend(&mut self, individuals: Vec<I>) {
        self.individuals.extend(individuals);
    }

    /// 移除个体 / Remove individual
    pub fn remove(&mut self, index: usize) -> Option<I> {
        if index < self.individuals.len() {
            Some(self.individuals.remove(index))
        } else {
            None
        }
    }

    /// 获取个体引用 / Get individual reference
    pub fn get(&self, index: usize) -> Option<&I> {
        self.individuals.get(index)
    }

    /// 获取可变个体引用 / Get mutable individual reference
    pub fn get_mut(&mut self, index: usize) -> Option<&mut I> {
        self.individuals.get_mut(index)
    }

    /// 获取所有个体 / Get all individuals
    pub fn individuals(&self) -> &[I] {
        &self.individuals
    }

    /// 获取所有可变个体 / Get all mutable individuals
    pub fn individuals_mut(&mut self) -> &mut [I] {
        &mut self.individuals
    }

    /// 获取种群大小 / Get population size
    pub fn len(&self) -> usize {
        self.individuals.len()
    }

    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        self.individuals.is_empty()
    }

    /// 清空种群 / Clear population
    pub fn clear(&mut self) {
        self.individuals.clear();
    }

    /// 设置适应度比较器 / Set fitness comparator
    pub fn set_comparator(&mut self, comparator: FitnessComparator) {
        self.comparator = comparator;
    }

    /// 获取适应度比较器 / Get fitness comparator
    pub fn comparator(&self) -> FitnessComparator {
        self.comparator
    }

    /// 按适应度排序 / Sort by fitness
    pub fn sort_by_fitness(&mut self) {
        self.individuals.sort_by(|a, b| {
            let fitness_a = a.fitness().unwrap_or(f64::NEG_INFINITY);
            let fitness_b = b.fitness().unwrap_or(f64::NEG_INFINITY);
            self.comparator.compare(fitness_a, fitness_b)
        });
    }

    /// 获取最优个体 / Get best individual
    pub fn best(&self) -> Option<&I> {
        self.individuals.iter().max_by(|a, b| {
            let fitness_a = a.fitness().unwrap_or(f64::NEG_INFINITY);
            let fitness_b = b.fitness().unwrap_or(f64::NEG_INFINITY);
            self.comparator.compare(fitness_a, fitness_b)
        })
    }

    /// 获取最差个体 / Get worst individual
    pub fn worst(&self) -> Option<&I> {
        self.individuals.iter().min_by(|a, b| {
            let fitness_a = a.fitness().unwrap_or(f64::NEG_INFINITY);
            let fitness_b = b.fitness().unwrap_or(f64::NEG_INFINITY);
            self.comparator.compare(fitness_a, fitness_b)
        })
    }

    /// 计算平均适应度 / Calculate average fitness
    pub fn average_fitness(&self) -> Option<f64> {
        let count = self
            .individuals
            .iter()
            .filter(|i| i.fitness().is_some())
            .count();
        if count == 0 {
            return None;
        }
        let sum: f64 = self.individuals.iter().filter_map(|i| i.fitness()).sum();
        Some(sum / count as f64)
    }

    /// 计算适应度标准差 / Calculate fitness standard deviation
    pub fn fitness_std(&self) -> Option<f64> {
        let avg = self.average_fitness()?;
        let count = self
            .individuals
            .iter()
            .filter(|i| i.fitness().is_some())
            .count();
        if count <= 1 {
            return None;
        }
        let variance: f64 = self
            .individuals
            .iter()
            .filter_map(|i| i.fitness())
            .map(|f| (f - avg).powi(2))
            .sum::<f64>()
            / (count - 1) as f64;
        Some(variance.sqrt())
    }

    /// 刷新优秀个体记录 / Refresh good individuals record
    ///
    /// 更新种群中优秀个体的记录，用于精英保留策略。
    /// Updates records of good individuals in population for elitism.
    pub fn refresh_good_individuals(&mut self, elite_count: usize) -> Vec<I> {
        self.sort_by_fitness();
        self.individuals
            .iter()
            .take(elite_count.min(self.len()))
            .cloned()
            .collect()
    }

    /// 转换为带适应度的解列表 / Convert to solution with fitness list
    pub fn to_solutions(&self) -> Vec<SolutionWithFitness<I, G>> {
        self.individuals
            .iter()
            .filter_map(|i| SolutionWithFitness::from_individual(i.clone()))
            .collect()
    }
}

impl<I, G> Default for Population<I, G>
where
    I: Individual<G>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I, G> IntoIterator for Population<I, G>
where
    I: Individual<G>,
{
    type Item = I;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.individuals.into_iter()
    }
}

impl<'a, I, G> IntoIterator for &'a Population<I, G>
where
    I: Individual<G>,
{
    type Item = &'a I;
    type IntoIter = std::slice::Iter<'a, I>;

    fn into_iter(self) -> Self::IntoIter {
        self.individuals.iter()
    }
}

// 类型别名 / Type aliases
/// 默认种群 / Default population
pub type DefaultPopulation = Population<super::FloatIndividual, f64>;
