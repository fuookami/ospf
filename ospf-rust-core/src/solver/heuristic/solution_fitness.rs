//! 带适应度的解
//! Solution with Fitness

use std::fmt::Debug;
use super::Individual;

/// 带适应度的解 / Solution with Fitness
///
/// 包装个体并缓存其适应度值。
/// Wraps an individual and caches its fitness value.
///
/// # 类型参数 / Type Parameters
/// - `I`: 个体类型 / Individual type
/// - `G`: 基因类型 / Gene type
#[derive(Debug, Clone)]
pub struct SolutionWithFitness<I, G>
where
    I: Individual<G>,
{
    /// 个体 / Individual
    pub individual: I,
    /// 适应度 / Fitness
    pub fitness: f64,
    /// 基因类型标记 / Gene type marker
    _gene: std::marker::PhantomData<G>,
}

impl<I, G> SolutionWithFitness<I, G>
where
    I: Individual<G>,
{
    /// 创建新的带适应度解 / Create new solution with fitness
    pub fn new(individual: I, fitness: f64) -> Self {
        Self {
            individual,
            fitness,
            _gene: std::marker::PhantomData,
        }
    }

    /// 从个体创建（需要已计算适应度）/ Create from individual (requires computed fitness)
    pub fn from_individual(individual: I) -> Option<Self> {
        individual
            .fitness()
            .map(|fitness| Self::new(individual, fitness))
    }

    /// 获取个体引用 / Get individual reference
    pub fn individual(&self) -> &I {
        &self.individual
    }

    /// 获取适应度 / Get fitness
    pub fn fitness(&self) -> f64 {
        self.fitness
    }

    /// 设置适应度 / Set fitness
    pub fn set_fitness(&mut self, fitness: f64) {
        self.fitness = fitness;
    }

    /// 比较适应度（最大化问题）/ Compare fitness (maximization problem)
    pub fn is_better_than(&self, other: &Self) -> bool {
        self.fitness > other.fitness
    }

    /// 比较适应度（最小化问题）/ Compare fitness (minimization problem)
    pub fn is_better_than_min(&self, other: &Self) -> bool {
        self.fitness < other.fitness
    }
}

impl<I, G> PartialEq for SolutionWithFitness<I, G>
where
    I: Individual<G>,
{
    fn eq(&self, other: &Self) -> bool {
        self.fitness == other.fitness
    }
}

impl<I, G> PartialOrd for SolutionWithFitness<I, G>
where
    I: Individual<G>,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.fitness.partial_cmp(&other.fitness)
    }
}

/// 适应度比较器 / Fitness Comparator
///
/// 用于排序和选择操作。
/// Used for sorting and selection operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitnessComparator {
    /// 最大化问题 / Maximization problem
    Maximize,
    /// 最小化问题 / Minimization problem
    Minimize,
}

impl FitnessComparator {
    /// 比较两个适应度值 / Compare two fitness values
    pub fn compare(&self, a: f64, b: f64) -> std::cmp::Ordering {
        match self {
            FitnessComparator::Maximize => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
            FitnessComparator::Minimize => b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal),
        }
    }

    /// 检查 a 是否优于 b / Check if a is better than b
    pub fn is_better(&self, a: f64, b: f64) -> bool {
        matches!(self.compare(a, b), std::cmp::Ordering::Greater)
    }

    /// 选择最优个体 / Select best individual
    pub fn select_best<'a, I, G>(
        &self,
        solutions: &'a [SolutionWithFitness<I, G>],
    ) -> Option<&'a SolutionWithFitness<I, G>>
    where
        I: Individual<G>,
    {
        solutions
            .iter()
            .max_by(|a, b| self.compare(a.fitness, b.fitness))
    }

    /// 选择最差个体 / Select worst individual
    pub fn select_worst<'a, I, G>(
        &self,
        solutions: &'a [SolutionWithFitness<I, G>],
    ) -> Option<&'a SolutionWithFitness<I, G>>
    where
        I: Individual<G>,
    {
        solutions
            .iter()
            .min_by(|a, b| self.compare(a.fitness, b.fitness))
    }
}

// 类型别名 / Type aliases
/// 默认带适应度解 / Default solution with fitness
pub type DefaultSolutionWithFitness = SolutionWithFitness<super::FloatIndividual, f64>;
