//! 交叉算子 / Crossover Operators
//!
//! 定义遗传算法中交叉操作的标准接口和多种实现，
//! 包括单点交叉、双点交叉、均匀交叉和模拟二进制交叉。
//! Defines the standard interface and multiple implementations for crossover operations
//! in genetic algorithms, including single-point, two-point, uniform, and simulated binary crossover.

use super::Individual;

/// 交叉算子 trait / Crossover Operator Trait
///
/// 定义交叉操作的标准接口，用于将两个父代个体组合产生子代。
/// Defines the standard interface for crossover operations, used to combine two parent individuals to produce offspring.
pub trait CrossoverOperator<I, G>: Send + Sync
where
    I: Individual<G>,
{
    /// 执行交叉操作 / Perform crossover
    ///
    /// 将两个父代个体的基因进行重组，产生两个子代个体。
    /// Recombines the genes of two parent individuals to produce two offspring individuals.
    ///
    /// # 参数 / Parameters
    /// - `parent1`: 父代 1 / Parent 1
    /// - `parent2`: 父代 2 / Parent 2
    ///
    /// # 返回 / Returns
    /// 两个子代个体 / Two offspring individuals
    fn crossover(&self, parent1: &I, parent2: &I) -> (I, I);
}

/// 单点交叉 / Single-point Crossover
///
/// 在随机位置切割基因序列，交换后半部分。
/// Cuts gene sequence at random position and swaps the latter halves.
#[derive(Debug, Clone, Copy)]
pub struct SinglePointCrossover {
    /// 交叉概率 / Crossover probability
    pub rate: f64,
}

impl SinglePointCrossover {
    /// 创建新算子 / Create new operator
    ///
    /// # 参数 / Parameters
    /// - `rate`: 交叉概率 / Crossover probability
    pub fn new(rate: f64) -> Self {
        Self { rate }
    }

    /// 创建默认算子（交叉概率 0.8）/ Create default operator (crossover rate 0.8)
    pub fn default_rate() -> Self {
        Self { rate: 0.8 }
    }
}

impl Default for SinglePointCrossover {
    fn default() -> Self {
        Self::default_rate()
    }
}

impl<I> CrossoverOperator<I, f64> for SinglePointCrossover
where
    I: Individual<f64>,
{
    fn crossover(&self, parent1: &I, parent2: &I) -> (I, I) {
        let len = parent1.len().min(parent2.len());
        if len == 0 {
            return (parent1.clone(), parent2.clone());
        }

        let mut child1 = parent1.clone();
        let mut child2 = parent2.clone();

        // 简化实现：中点交叉
        // Simplified implementation: midpoint crossover
        let mid = len / 2;

        for i in mid..len {
            std::mem::swap(&mut child1.genes_mut()[i], &mut child2.genes_mut()[i]);
        }

        // 清除适应度缓存
        child1.clear_fitness();
        child2.clear_fitness();

        (child1, child2)
    }
}

/// 双点交叉 / Two-point Crossover
///
/// 在两个随机位置切割基因序列，交换中间部分。
/// Cuts gene sequence at two random positions and swaps the middle part.
#[derive(Debug, Clone, Copy)]
pub struct TwoPointCrossover {
    /// 交叉概率 / Crossover probability
    pub rate: f64,
}

impl TwoPointCrossover {
    /// 创建新算子 / Create new operator
    ///
    /// # 参数 / Parameters
    /// - `rate`: 交叉概率 / Crossover probability
    pub fn new(rate: f64) -> Self {
        Self { rate }
    }
}

impl Default for TwoPointCrossover {
    fn default() -> Self {
        Self { rate: 0.8 }
    }
}

impl<I> CrossoverOperator<I, f64> for TwoPointCrossover
where
    I: Individual<f64>,
{
    fn crossover(&self, parent1: &I, parent2: &I) -> (I, I) {
        let len = parent1.len().min(parent2.len());
        if len < 2 {
            return (parent1.clone(), parent2.clone());
        }

        let mut child1 = parent1.clone();
        let mut child2 = parent2.clone();

        // 简化实现：使用 1/3 和 2/3 位置
        // Simplified implementation: use 1/3 and 2/3 positions
        let p1 = len / 3;
        let p2 = 2 * len / 3;

        for i in p1..p2 {
            std::mem::swap(&mut child1.genes_mut()[i], &mut child2.genes_mut()[i]);
        }

        child1.clear_fitness();
        child2.clear_fitness();

        (child1, child2)
    }
}

/// 均匀交叉 / Uniform Crossover
///
/// 每个基因独立地以 50% 概率来自任一父代。
/// Each gene independently comes from either parent with 50% probability.
#[derive(Debug, Clone, Copy)]
pub struct UniformCrossover {
    /// 交叉概率 / Crossover probability
    pub rate: f64,
    /// 基因交换概率 / Gene swap probability
    pub swap_rate: f64,
}

impl UniformCrossover {
    /// 创建新算子 / Create new operator
    ///
    /// # 参数 / Parameters
    /// - `rate`: 交叉概率 / Crossover probability
    /// - `swap_rate`: 基因交换概率 / Gene swap probability
    pub fn new(rate: f64, swap_rate: f64) -> Self {
        Self { rate, swap_rate }
    }
}

impl Default for UniformCrossover {
    fn default() -> Self {
        Self {
            rate: 0.8,
            swap_rate: 0.5,
        }
    }
}

impl<I> CrossoverOperator<I, f64> for UniformCrossover
where
    I: Individual<f64>,
{
    fn crossover(&self, parent1: &I, parent2: &I) -> (I, I) {
        let len = parent1.len().min(parent2.len());
        if len == 0 {
            return (parent1.clone(), parent2.clone());
        }

        let mut child1 = parent1.clone();
        let mut child2 = parent2.clone();

        // 简化实现：每个基因 50% 概率交换
        // Simplified implementation: 50% chance to swap each gene
        for i in 0..len {
            if i % 2 == 0 {
                std::mem::swap(&mut child1.genes_mut()[i], &mut child2.genes_mut()[i]);
            }
        }

        child1.clear_fitness();
        child2.clear_fitness();

        (child1, child2)
    }
}

/// 模拟二进制交叉 (SBX) / Simulated Binary Crossover
///
/// 适用于实数编码的交叉算子，通过模拟二进制交叉的分布特性
/// 生成与父代接近的子代。
/// Crossover operator suitable for real-valued encoding, generating offspring
/// close to the parents by simulating the distribution characteristics of binary crossover.
#[derive(Debug, Clone, Copy)]
pub struct SimulatedBinaryCrossover {
    /// 交叉概率 / Crossover probability
    pub rate: f64,
    /// 分布指数 / Distribution index
    pub eta: f64,
}

impl SimulatedBinaryCrossover {
    /// 创建新算子 / Create new operator
    ///
    /// # 参数 / Parameters
    /// - `rate`: 交叉概率 / Crossover probability
    /// - `eta`: 分布指数，值越大子代越接近父代 / Distribution index, larger values produce offspring closer to parents
    pub fn new(rate: f64, eta: f64) -> Self {
        Self { rate, eta }
    }
}

impl Default for SimulatedBinaryCrossover {
    fn default() -> Self {
        Self {
            rate: 0.9,
            eta: 20.0,
        }
    }
}

impl<I> CrossoverOperator<I, f64> for SimulatedBinaryCrossover
where
    I: Individual<f64>,
{
    fn crossover(&self, parent1: &I, parent2: &I) -> (I, I) {
        let len = parent1.len().min(parent2.len());
        if len == 0 {
            return (parent1.clone(), parent2.clone());
        }

        let mut child1 = parent1.clone();
        let mut child2 = parent2.clone();

        // 简化的 SBX 实现
        // Simplified SBX implementation
        let beta = 1.0; // 简化：使用固定 beta 值

        for i in 0..len {
            let p1 = parent1.genes()[i];
            let p2 = parent2.genes()[i];

            let c1 = 0.5 * ((1.0 + beta) * p1 + (1.0 - beta) * p2);
            let c2 = 0.5 * ((1.0 - beta) * p1 + (1.0 + beta) * p2);

            child1.genes_mut()[i] = c1;
            child2.genes_mut()[i] = c2;
        }

        child1.clear_fitness();
        child2.clear_fitness();

        (child1, child2)
    }
}

/// 二进制编码的单点交叉实现 / Single-point crossover implementation for binary encoding
impl<I> CrossoverOperator<I, bool> for SinglePointCrossover
where
    I: Individual<bool>,
{
    fn crossover(&self, parent1: &I, parent2: &I) -> (I, I) {
        let len = parent1.len().min(parent2.len());
        if len == 0 {
            return (parent1.clone(), parent2.clone());
        }

        let mut child1 = parent1.clone();
        let mut child2 = parent2.clone();

        let mid = len / 2;

        for i in mid..len {
            std::mem::swap(&mut child1.genes_mut()[i], &mut child2.genes_mut()[i]);
        }

        child1.clear_fitness();
        child2.clear_fitness();

        (child1, child2)
    }
}
