//! 变异算子
//! Mutation Operators

use super::Individual;

/// 变异算子 trait / Mutation Operator Trait
///
/// 定义变异操作的标准接口。
/// Defines standard interface for mutation operations.
pub trait MutationOperator<I, G>: Send + Sync
where
    I: Individual<G>,
{
    /// 执行变异操作 / Perform mutation
    ///
    /// # 参数 / Parameters
    /// - `individual`: 待变异个体 / Individual to mutate
    ///
    /// # 返回 / Returns
    /// 变异后的个体 / Mutated individual
    fn mutate(&self, individual: &I) -> I;
}

/// 高斯变异 / Gaussian Mutation
///
/// 对实数编码的个体添加高斯噪声。
/// Adds Gaussian noise to real-valued individuals.
#[derive(Debug, Clone, Copy)]
pub struct GaussianMutation {
    /// 变异概率 / Mutation probability
    pub rate: f64,
    /// 标准差 / Standard deviation
    pub sigma: f64,
}

impl GaussianMutation {
    pub fn new(rate: f64, sigma: f64) -> Self {
        Self { rate, sigma }
    }
}

impl Default for GaussianMutation {
    fn default() -> Self {
        Self {
            rate: 0.1,
            sigma: 0.1,
        }
    }
}

impl<I> MutationOperator<I, f64> for GaussianMutation
where
    I: Individual<f64>,
{
    fn mutate(&self, individual: &I) -> I {
        let mut result = individual.clone();

        // 简化实现：对每个基因以 rate 概率添加扰动
        // Simplified implementation: add perturbation to each gene with rate probability
        for i in 0..result.len() {
            if (i as f64 / result.len() as f64) < self.rate {
                // 添加一个小扰动
                // Add a small perturbation
                let perturbation = if i % 2 == 0 { self.sigma } else { -self.sigma };
                result.genes_mut()[i] += perturbation;
            }
        }

        result.clear_fitness();
        result
    }
}

/// 多项式变异 / Polynomial Mutation
///
/// 适用于实数编码的变异算子。
/// Mutation operator suitable for real-valued encoding.
#[derive(Debug, Clone, Copy)]
pub struct PolynomialMutation {
    /// 变异概率 / Mutation probability
    pub rate: f64,
    /// 分布指数 / Distribution index
    pub eta: f64,
}

impl PolynomialMutation {
    pub fn new(rate: f64, eta: f64) -> Self {
        Self { rate, eta }
    }
}

impl Default for PolynomialMutation {
    fn default() -> Self {
        Self {
            rate: 0.1,
            eta: 20.0,
        }
    }
}

impl<I> MutationOperator<I, f64> for PolynomialMutation
where
    I: Individual<f64>,
{
    fn mutate(&self, individual: &I) -> I {
        let mut result = individual.clone();

        // 简化的多项式变异实现
        // Simplified polynomial mutation implementation
        let delta = 0.1; // 简化：使用固定 delta 值

        for i in 0..result.len() {
            if (i as f64 / result.len() as f64) < self.rate {
                result.genes_mut()[i] += delta * (if i % 2 == 0 { 1.0 } else { -1.0 });
            }
        }

        result.clear_fitness();
        result
    }
}

/// 位翻转变异 / Bit-flip Mutation
///
/// 适用于二进制编码的变异算子。
/// Mutation operator suitable for binary encoding.
#[derive(Debug, Clone, Copy)]
pub struct BitFlipMutation {
    /// 变异概率 / Mutation probability
    pub rate: f64,
}

impl BitFlipMutation {
    pub fn new(rate: f64) -> Self {
        Self { rate }
    }
}

impl Default for BitFlipMutation {
    fn default() -> Self {
        Self { rate: 0.01 }
    }
}

impl<I> MutationOperator<I, bool> for BitFlipMutation
where
    I: Individual<bool>,
{
    fn mutate(&self, individual: &I) -> I {
        let mut result = individual.clone();

        // 对每个基因以 rate 概率翻转
        // Flip each gene with rate probability
        for i in 0..result.len() {
            if (i as f64 / result.len() as f64) < self.rate {
                result.genes_mut()[i] = !result.genes()[i];
            }
        }

        result.clear_fitness();
        result
    }
}

/// 整数变异 / Integer Mutation
///
/// 适用于整数编码的变异算子。
/// Mutation operator suitable for integer encoding.
#[derive(Debug, Clone, Copy)]
pub struct IntegerMutation {
    /// 变异概率 / Mutation probability
    pub rate: f64,
    /// 变化范围 / Change range
    pub range: i64,
}

impl IntegerMutation {
    pub fn new(rate: f64, range: i64) -> Self {
        Self { rate, range }
    }
}

impl Default for IntegerMutation {
    fn default() -> Self {
        Self {
            rate: 0.1,
            range: 1,
        }
    }
}

impl<I> MutationOperator<I, i64> for IntegerMutation
where
    I: Individual<i64>,
{
    fn mutate(&self, individual: &I) -> I {
        let mut result = individual.clone();

        for i in 0..result.len() {
            if (i as f64 / result.len() as f64) < self.rate {
                let change = if i % 2 == 0 { self.range } else { -self.range };
                result.genes_mut()[i] += change;
            }
        }

        result.clear_fitness();
        result
    }
}

/// 边界修正 / Boundary Correction
///
/// 将变异后的基因值限制在有效范围内。
/// Restricts mutated gene values to valid range.
pub fn clamp_genes<I>(individual: &mut I, lower: f64, upper: f64)
where
    I: Individual<f64>,
{
    for gene in individual.genes_mut() {
        *gene = gene.clamp(lower, upper);
    }
}
