//! 随机过程算子 / Stochastic operators

use ospf_rust_math::distribution::{
    DirectionSampler, Distribution, DistributionError, Entropy, Gaussian, Levy,
};

/// 高维莱维飞行，把重尾幅值与单位方向组合为完整步长 / High-dimensional Lévy flight combining a heavy-tailed magnitude with a unit direction
#[derive(Debug, Clone, Copy)]
pub struct LevyFlight {
    /// 维度 / Dimensionality
    pub dimension: usize,
    /// 稳定性参数 / Stability parameter
    pub alpha: f64,
    /// 幅值缩放 / Magnitude scaling
    pub scale: f64,
}

impl LevyFlight {
    /// 创建莱维飞行 / Create a Lévy flight
    pub fn new(dimension: usize, alpha: f64, scale: f64) -> Self {
        Self {
            dimension,
            alpha,
            scale,
        }
    }

    /// 采样一步莱维飞行 / Sample one Lévy flight step
    pub fn sample(&self, entropy: &mut dyn Entropy) -> Result<Vec<f64>, DistributionError> {
        let magnitude = Levy::new(self.alpha).sample_checked(entropy)? * self.scale;
        let direction = DirectionSampler::new(self.dimension).sample_checked(entropy)?;
        Ok(direction
            .into_iter()
            .map(|component| component * magnitude)
            .collect())
    }
}

impl Default for LevyFlight {
    fn default() -> Self {
        Self::new(3, 1.5, 1.0)
    }
}

/// 逐维噪声步长，用于变异与位置更新 / Per-dimension noise step used by mutation and position updates
pub struct NoiseStep {
    /// 维度 / Dimensionality
    pub dimension: usize,
    /// 噪声分布 / Noise distribution
    pub distribution: Box<dyn Distribution>,
}

impl NoiseStep {
    /// 创建噪声步长 / Create a noise step
    pub fn new(dimension: usize, distribution: Box<dyn Distribution>) -> Self {
        Self {
            dimension,
            distribution,
        }
    }

    /// 采样一步噪声 / Sample one noise step
    pub fn sample(&self, entropy: &mut dyn Entropy) -> Result<Vec<f64>, DistributionError> {
        self.distribution.validate()?;
        Ok((0..self.dimension)
            .map(|_| self.distribution.sample(entropy))
            .collect())
    }
}

impl Default for NoiseStep {
    fn default() -> Self {
        Self::new(3, Box::new(Gaussian::standard()))
    }
}
