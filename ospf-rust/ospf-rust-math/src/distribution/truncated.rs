//! 截断分布 / Truncated distribution

use super::{Distribution, DistributionError, DistributionResult, Entropy};

/// 截断分布包装 / Truncated distribution wrapper
///
/// 通过拒绝采样把任意分布限制在 `[lower_bound, upper_bound]` 区间；超过最大尝试次数后退化为区间裁剪。
/// Restricts an arbitrary distribution to `[lower_bound, upper_bound]` via rejection sampling, falling back to interval
/// clamping after the maximum number of attempts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Truncated<D>
where
    D: Distribution,
{
    /// 被截断的分布 / The distribution being truncated
    pub distribution: D,
    /// 下界 / Lower bound
    pub lower_bound: f64,
    /// 上界 / Upper bound
    pub upper_bound: f64,
    /// 最大拒绝采样次数 / Maximum number of rejection attempts
    pub max_attempts: usize,
}

impl<D> Truncated<D>
where
    D: Distribution,
{
    /// 创建截断分布 / Create a truncated distribution
    pub fn new(distribution: D, lower_bound: f64, upper_bound: f64, max_attempts: usize) -> Self {
        Self {
            distribution,
            lower_bound,
            upper_bound,
            max_attempts,
        }
    }
}

impl<D> Distribution for Truncated<D>
where
    D: Distribution,
{
    fn validate(&self) -> DistributionResult<()> {
        if self.lower_bound > self.upper_bound {
            return Err(DistributionError::new(
                "lowerBound must not exceed upperBound / 下界不得大于上界",
            ));
        }
        if self.max_attempts == 0 {
            return Err(DistributionError::new(
                "maxAttempts must be positive / 最大尝试次数必须为正",
            ));
        }
        self.distribution.validate()
    }

    fn sample(&self, entropy: &mut dyn Entropy) -> f64 {
        for _ in 0..self.max_attempts {
            let value = self.distribution.sample(entropy);
            if value >= self.lower_bound && value <= self.upper_bound {
                return value;
            }
        }
        let value = self.distribution.sample(entropy);
        if value.is_nan() {
            return self.lower_bound;
        }
        value.clamp(self.lower_bound, self.upper_bound)
    }
}
