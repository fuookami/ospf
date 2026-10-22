//! 指数分布 / Exponential distribution

use super::{Distribution, DistributionError, DistributionResult, Entropy, UNIT_EPSILON, uniform_unit};

/// 指数分布 / Exponential distribution
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exponential {
    /// 速率参数 λ / Rate parameter λ
    pub rate: f64,
}

impl Exponential {
    /// 创建指数分布 / Create an exponential distribution
    pub fn new(rate: f64) -> Self {
        Self { rate }
    }
}

impl Default for Exponential {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl Distribution for Exponential {
    fn validate(&self) -> DistributionResult<()> {
        if self.rate > 0.0 {
            Ok(())
        } else {
            Err(DistributionError::new(
                "rate must be positive / 速率参数必须为正",
            ))
        }
    }

    fn sample(&self, entropy: &mut dyn Entropy) -> f64 {
        let u = uniform_unit(entropy).max(UNIT_EPSILON);
        -(1.0 - u).ln() / self.rate
    }
}
