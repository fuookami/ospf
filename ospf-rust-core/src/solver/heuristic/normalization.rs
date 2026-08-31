//! 归一化
//! Normalization

use super::Individual;

/// 归一化方法 / Normalization Method
///
/// 将基因值归一化到指定范围。
/// Normalizes gene values to specified range.
#[derive(Debug, Clone, Copy)]
pub struct Normalization {
    /// 目标下界 / Target lower bound
    pub lower: f64,
    /// 目标上界 / Target upper bound
    pub upper: f64,
}

impl Normalization {
    /// 创建新归一化器 / Create new normalizer
    pub fn new(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }

    /// 创建 [0, 1] 归一化器 / Create [0, 1] normalizer
    pub fn unit() -> Self {
        Self::new(0.0, 1.0)
    }

    /// 创建 [-1, 1] 归一化器 / Create [-1, 1] normalizer
    pub fn symmetric() -> Self {
        Self::new(-1.0, 1.0)
    }

    /// 归一化个体 / Normalize individual
    ///
    /// # 参数 / Parameters
    /// - `individual`: 待归一化个体 / Individual to normalize
    /// - `min_vals`: 每个基因的最小值 / Minimum value for each gene
    /// - `max_vals`: 每个基因的最大值 / Maximum value for each gene
    pub fn normalize<I>(&self, individual: &mut I, min_vals: &[f64], max_vals: &[f64])
    where
        I: Individual<f64>,
    {
        let genes = individual.genes_mut();
        let n = genes.len().min(min_vals.len()).min(max_vals.len());

        for i in 0..n {
            let min = min_vals[i];
            let max = max_vals[i];

            if max > min {
                // 归一化到 [0, 1]，然后缩放到目标范围
                // Normalize to [0, 1], then scale to target range
                let normalized = (genes[i] - min) / (max - min);
                genes[i] = self.lower + normalized * (self.upper - self.lower);
            }
        }
    }

    /// 反归一化 / Denormalize
    ///
    /// 将归一化的值还原到原始范围。
    /// Restores normalized values to original range.
    pub fn denormalize<I>(&self, individual: &mut I, min_vals: &[f64], max_vals: &[f64])
    where
        I: Individual<f64>,
    {
        let genes = individual.genes_mut();
        let n = genes.len().min(min_vals.len()).min(max_vals.len());

        for i in 0..n {
            let min = min_vals[i];
            let max = max_vals[i];

            if self.upper > self.lower {
                // 从目标范围还原到原始范围
                // Restore from target range to original range
                let normalized = (genes[i] - self.lower) / (self.upper - self.lower);
                genes[i] = min + normalized * (max - min);
            }
        }
    }
}

impl Default for Normalization {
    fn default() -> Self {
        Self::unit()
    }
}

/// Z-score 标准化 / Z-score Standardization
///
/// 将基因值标准化为均值 0、标准差 1。
/// Standardizes gene values to mean 0, std 1.
#[derive(Debug, Clone, Copy, Default)]
pub struct ZScoreNormalization;

impl ZScoreNormalization {
    pub fn new() -> Self {
        Self
    }

    /// 标准化个体 / Standardize individual
    ///
    /// # 参数 / Parameters
    /// - `individual`: 待标准化个体 / Individual to standardize
    /// - `means`: 每个基因的均值 / Mean for each gene
    /// - `stds`: 每个基因的标准差 / Standard deviation for each gene
    pub fn standardize<I>(&self, individual: &mut I, means: &[f64], stds: &[f64])
    where
        I: Individual<f64>,
    {
        let genes = individual.genes_mut();
        let n = genes.len().min(means.len()).min(stds.len());

        for i in 0..n {
            if stds[i] > 0.0 {
                genes[i] = (genes[i] - means[i]) / stds[i];
            }
        }
    }

    /// 反标准化 / Destandardize
    ///
    /// 将标准化的值还原到原始范围。
    /// Restores standardized values to original range.
    pub fn destandardize<I>(&self, individual: &mut I, means: &[f64], stds: &[f64])
    where
        I: Individual<f64>,
    {
        let genes = individual.genes_mut();
        let n = genes.len().min(means.len()).min(stds.len());

        for i in 0..n {
            genes[i] = genes[i] * stds[i] + means[i];
        }
    }
}

/// 适应度缩放 / Fitness Scaling
///
/// 对适应度值进行缩放以改善选择压力。
/// Scales fitness values to improve selection pressure.
#[derive(Debug, Clone, Copy)]
pub enum FitnessScaling {
    /// 线性缩放 / Linear scaling
    Linear { a: f64, b: f64 },
    /// 幂律缩放 / Power law scaling
    Power { alpha: f64 },
    /// 对数缩放 / Logarithmic scaling
    Logarithmic { base: f64 },
    /// 排名缩放 / Rank scaling
    Rank { pressure: f64 },
}

impl FitnessScaling {
    /// 创建线性缩放 / Create linear scaling
    pub fn linear(a: f64, b: f64) -> Self {
        Self::Linear { a, b }
    }

    /// 创建幂律缩放 / Create power law scaling
    pub fn power(alpha: f64) -> Self {
        Self::Power { alpha }
    }

    /// 创建对数缩放 / Create logarithmic scaling
    pub fn logarithmic(base: f64) -> Self {
        Self::Logarithmic { base }
    }

    /// 创建排名缩放 / Create rank scaling
    pub fn rank(pressure: f64) -> Self {
        Self::Rank { pressure }
    }

    /// 应用缩放 / Apply scaling
    pub fn scale(&self, fitness: f64) -> f64 {
        match self {
            Self::Linear { a, b } => a * fitness + b,
            Self::Power { alpha } => fitness.powf(*alpha),
            Self::Logarithmic { base } => {
                if fitness > 0.0 && *base > 1.0 {
                    fitness.log(*base)
                } else {
                    0.0
                }
            }
            Self::Rank { pressure: _f64 } => {
                // 排名缩放需要知道排名，这里返回原值
                // Rank scaling needs rank, return original value here
                fitness
            }
        }
    }

    /// 批量缩放 / Batch scaling
    pub fn scale_all(&self, fitnesses: &[f64]) -> Vec<f64> {
        match self {
            Self::Rank { pressure } => {
                // 排名缩放：根据排名分配缩放后的适应度
                // Rank scaling: assign scaled fitness based on rank
                let n = fitnesses.len();
                if n == 0 {
                    return Vec::new();
                }

                // 创建索引-适应度对并排序
                // Create index-fitness pairs and sort
                let mut indexed: Vec<(usize, f64)> =
                    fitnesses.iter().enumerate().map(|(i, &f)| (i, f)).collect();
                indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                // 计算缩放后的适应度
                // Calculate scaled fitness
                let mut result = vec![0.0; n];
                for (rank, (i, _)) in indexed.iter().enumerate() {
                    result[*i] =
                        *pressure - (*pressure - 1.0) * rank as f64 / (n - 1).max(1) as f64;
                }
                result
            }
            _ => fitnesses.iter().map(|&f| self.scale(f)).collect(),
        }
    }
}

impl Default for FitnessScaling {
    fn default() -> Self {
        Self::linear(1.0, 0.0)
    }
}

/// Min-Max 归一化 / Min-Max Normalization
///
/// 将适应度值归一化到 [0, 1] 范围。
/// Normalizes fitness values to [0, 1] range.
pub fn min_max_normalize(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }

    let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    if max_val == min_val {
        return vec![0.5; values.len()];
    }

    values
        .iter()
        .map(|&v| (v - min_val) / (max_val - min_val))
        .collect()
}

/// 累积概率计算（用于轮盘赌选择）/ Cumulative probability calculation (for roulette wheel selection)
pub fn cumulative_probabilities(fitnesses: &[f64]) -> Vec<f64> {
    if fitnesses.is_empty() {
        return Vec::new();
    }

    let total: f64 = fitnesses.iter().sum();
    if total == 0.0 {
        return vec![1.0; fitnesses.len()];
    }

    let mut cumsum = 0.0;
    fitnesses
        .iter()
        .map(|&f| {
            cumsum += f / total;
            cumsum
        })
        .collect()
}
