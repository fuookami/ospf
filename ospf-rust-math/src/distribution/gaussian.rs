//! 高斯（正态）分布 / Gaussian (normal) distribution

use super::{
    Distribution, DistributionError, DistributionResult, Entropy, standard_gaussian,
};

/// 高斯分布 / Gaussian distribution
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian {
    /// 均值 / Mean
    pub mean: f64,
    /// 标准差 / Standard deviation
    pub standard_deviation: f64,
}

impl Gaussian {
    /// 创建高斯分布 / Create a Gaussian distribution
    pub fn new(mean: f64, standard_deviation: f64) -> Self {
        Self {
            mean,
            standard_deviation,
        }
    }

    /// 标准正态分布 / Standard normal distribution
    pub fn standard() -> Self {
        Self::new(0.0, 1.0)
    }
}

impl Default for Gaussian {
    fn default() -> Self {
        Self::standard()
    }
}

impl Distribution for Gaussian {
    fn validate(&self) -> DistributionResult<()> {
        if self.standard_deviation >= 0.0 {
            Ok(())
        } else {
            Err(DistributionError::new(
                "standardDeviation must be non-negative / 标准差不得为负",
            ))
        }
    }

    fn sample(&self, entropy: &mut dyn Entropy) -> f64 {
        self.mean + self.standard_deviation * standard_gaussian(entropy)
    }
}
