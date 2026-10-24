//! 均匀分布 / Uniform distribution

use super::{Distribution, DistributionError, DistributionResult, Entropy, uniform_unit};

/// 连续均匀分布 / Continuous uniform distribution
///
/// 在 `[lower_bound, upper_bound]` 区间内均匀采样。
/// Samples uniformly within `[lower_bound, upper_bound]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Uniform {
    /// 下界 / Lower bound
    pub lower_bound: f64,
    /// 上界 / Upper bound
    pub upper_bound: f64,
}

impl Uniform {
    /// 创建均匀分布 / Create a uniform distribution
    pub fn new(lower_bound: f64, upper_bound: f64) -> Self {
        Self {
            lower_bound,
            upper_bound,
        }
    }
}

impl Default for Uniform {
    fn default() -> Self {
        Self::new(0.0, 1.0)
    }
}

impl Distribution for Uniform {
    fn validate(&self) -> DistributionResult<()> {
        if self.lower_bound <= self.upper_bound {
            Ok(())
        } else {
            Err(DistributionError::new(
                "lowerBound must not exceed upperBound / 下界不得大于上界",
            ))
        }
    }

    fn sample(&self, entropy: &mut dyn Entropy) -> f64 {
        let width = self.upper_bound - self.lower_bound;
        self.lower_bound + uniform_unit(entropy) * width
    }
}
