//! 柯西分布 / Cauchy distribution

use super::{Distribution, DistributionError, DistributionResult, Entropy, uniform_unit};

/// 柯西分布 / Cauchy distribution
///
/// 重尾分布，常用于柯西变异以增强跳出局部最优的能力。
/// A heavy-tailed distribution commonly used by Cauchy mutation to escape local optima.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cauchy {
    /// 位置参数 / Location parameter
    pub location: f64,
    /// 尺度参数 / Scale parameter
    pub scale: f64,
}

impl Cauchy {
    /// 创建柯西分布 / Create a Cauchy distribution
    pub fn new(location: f64, scale: f64) -> Self {
        Self { location, scale }
    }
}

impl Default for Cauchy {
    fn default() -> Self {
        Self::new(0.0, 1.0)
    }
}

impl Distribution for Cauchy {
    fn validate(&self) -> DistributionResult<()> {
        if self.scale > 0.0 {
            Ok(())
        } else {
            Err(DistributionError::new(
                "scale must be positive / 尺度参数必须为正",
            ))
        }
    }

    fn sample(&self, entropy: &mut dyn Entropy) -> f64 {
        let u = uniform_unit(entropy);
        self.location + self.scale * (std::f64::consts::PI * (u - 0.5)).tan()
    }
}
