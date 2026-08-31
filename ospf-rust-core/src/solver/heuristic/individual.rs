//! 个体 trait 定义
//! Individual Trait Definition

use std::fmt::Debug;

/// 个体 trait / Individual Trait
///
/// 表示进化算法中的一个个体。
/// Represents an individual in evolutionary algorithms.
///
/// # 类型参数 / Type Parameters
/// - `G`: 基因类型 / Gene type
///
/// # 示例 / Examples
///
/// ```rust
/// use ospf_rust_core::solver::heuristic::Individual;
///
/// #[derive(Clone, Debug)]
/// struct MyIndividual {
///     genes: Vec<f64>,
///     fitness: Option<f64>,
/// }
///
/// impl Individual<f64> for MyIndividual {
///     fn genes(&self) -> &[f64] { &self.genes }
///     fn genes_mut(&mut self) -> &mut [f64] { &mut self.genes }
///     fn fitness(&self) -> Option<f64> { self.fitness }
///     fn set_fitness(&mut self, fitness: f64) { self.fitness = Some(fitness); }
/// }
/// ```
pub trait Individual<G>: Clone + Debug + Send + Sync {
    /// 获取基因引用 / Get genes reference
    fn genes(&self) -> &[G];

    /// 获取可变基因引用 / Get mutable genes reference
    fn genes_mut(&mut self) -> &mut [G];

    /// 获取适应度 / Get fitness
    fn fitness(&self) -> Option<f64>;

    /// 设置适应度 / Set fitness
    fn set_fitness(&mut self, fitness: f64);

    /// 清除适应度 / Clear fitness
    fn clear_fitness(&mut self) {
        // 默认实现：设置适应度为 None
        // Default implementation: set fitness to None
    }

    /// 获取基因数量 / Get gene count
    fn len(&self) -> usize {
        self.genes().len()
    }

    /// 检查是否为空 / Check if empty
    fn is_empty(&self) -> bool {
        self.genes().is_empty()
    }

    /// 检查适应度是否已计算 / Check if fitness is computed
    fn has_fitness(&self) -> bool {
        self.fitness().is_some()
    }
}

/// 浮点数个体 / Floating-point Individual
///
/// 使用 `Vec<f64>` 作为基因的个体实现。
/// Individual implementation using `Vec<f64>` as genes.
#[derive(Debug, Clone)]
pub struct FloatIndividual {
    /// 基因向量 / Gene vector
    genes: Vec<f64>,
    /// 适应度 / Fitness
    fitness: Option<f64>,
}

impl FloatIndividual {
    /// 创建新个体 / Create new individual
    pub fn new(genes: Vec<f64>) -> Self {
        Self {
            genes,
            fitness: None,
        }
    }

    /// 创建随机个体 / Create random individual
    ///
    /// # 参数 / Parameters
    /// - `len`: 基因数量 / Gene count
    /// - `lower_bound`: 基因下界 / Lower bound for genes
    /// - `upper_bound`: 基因上界 / Upper bound for genes
    #[cfg(feature = "rand")]
    pub fn random(len: usize, lower_bound: f64, upper_bound: f64) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let genes = (0..len)
            .map(|_| rng.gen_range(lower_bound..=upper_bound))
            .collect();
        Self::new(genes)
    }

    /// 创建零个体 / Create zero individual
    pub fn zeros(len: usize) -> Self {
        Self::new(vec![0.0; len])
    }
}

impl Individual<f64> for FloatIndividual {
    fn genes(&self) -> &[f64] {
        &self.genes
    }

    fn genes_mut(&mut self) -> &mut [f64] {
        &mut self.genes
    }

    fn fitness(&self) -> Option<f64> {
        self.fitness
    }

    fn set_fitness(&mut self, fitness: f64) {
        self.fitness = Some(fitness);
    }

    fn clear_fitness(&mut self) {
        self.fitness = None;
    }
}

/// 二进制个体 / Binary Individual
///
/// 使用 `Vec<bool>` 作为基因的个体实现。
/// Individual implementation using `Vec<bool>` as genes.
#[derive(Debug, Clone)]
pub struct BinaryIndividual {
    /// 基因向量 / Gene vector
    genes: Vec<bool>,
    /// 适应度 / Fitness
    fitness: Option<f64>,
}

impl BinaryIndividual {
    /// 创建新个体 / Create new individual
    pub fn new(genes: Vec<bool>) -> Self {
        Self {
            genes,
            fitness: None,
        }
    }

    /// 创建随机个体 / Create random individual
    #[cfg(feature = "rand")]
    pub fn random(len: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let genes = (0..len).map(|_| rng.gen_bool(0.5)).collect();
        Self::new(genes)
    }

    /// 创建全 false 个体 / Create all-false individual
    pub fn all_false(len: usize) -> Self {
        Self::new(vec![false; len])
    }

    /// 创建全 true 个体 / Create all-true individual
    pub fn all_true(len: usize) -> Self {
        Self::new(vec![true; len])
    }
}

impl Individual<bool> for BinaryIndividual {
    fn genes(&self) -> &[bool] {
        &self.genes
    }

    fn genes_mut(&mut self) -> &mut [bool] {
        &mut self.genes
    }

    fn fitness(&self) -> Option<f64> {
        self.fitness
    }

    fn set_fitness(&mut self, fitness: f64) {
        self.fitness = Some(fitness);
    }

    fn clear_fitness(&mut self) {
        self.fitness = None;
    }
}

/// 整数个体 / Integer Individual
///
/// 使用 `Vec<i64>` 作为基因的个体实现。
/// Individual implementation using `Vec<i64>` as genes.
#[derive(Debug, Clone)]
pub struct IntegerIndividual {
    /// 基因向量 / Gene vector
    genes: Vec<i64>,
    /// 适应度 / Fitness
    fitness: Option<f64>,
}

impl IntegerIndividual {
    /// 创建新个体 / Create new individual
    pub fn new(genes: Vec<i64>) -> Self {
        Self {
            genes,
            fitness: None,
        }
    }

    /// 创建随机个体 / Create random individual
    #[cfg(feature = "rand")]
    pub fn random(len: usize, lower_bound: i64, upper_bound: i64) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let genes = (0..len)
            .map(|_| rng.gen_range(lower_bound..=upper_bound))
            .collect();
        Self::new(genes)
    }

    /// 创建零个体 / Create zero individual
    pub fn zeros(len: usize) -> Self {
        Self::new(vec![0; len])
    }
}

impl Individual<i64> for IntegerIndividual {
    fn genes(&self) -> &[i64] {
        &self.genes
    }

    fn genes_mut(&mut self) -> &mut [i64] {
        &mut self.genes
    }

    fn fitness(&self) -> Option<f64> {
        self.fitness
    }

    fn set_fitness(&mut self, fitness: f64) {
        self.fitness = Some(fitness);
    }

    fn clear_fitness(&mut self) {
        self.fitness = None;
    }
}

// 类型别名 / Type aliases
/// 默认浮点个体 / Default float individual
pub type DefaultIndividual = FloatIndividual;
