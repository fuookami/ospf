//! 贝塔分布 / Beta distribution

use super::{Distribution, DistributionError, DistributionResult, Entropy, Gamma};

/// 贝塔分布 / Beta distribution
///
/// 由两个伽马分布之比得到，取值位于 `(0, 1)` 区间。
/// Obtained from the ratio of two gamma distributions, taking values within `(0, 1)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beta {
    /// 形状参数 α / Shape parameter α
    pub alpha: f64,
    /// 形状参数 β / Shape parameter β
    pub beta: f64,
}

impl Beta {
    /// 创建贝塔分布 / Create a beta distribution
    pub fn new(alpha: f64, beta: f64) -> Self {
        Self { alpha, beta }
    }
}

impl Default for Beta {
    fn default() -> Self {
        Self::new(1.0, 1.0)
    }
}

impl Distribution for Beta {
    fn validate(&self) -> DistributionResult<()> {
        if self.alpha <= 0.0 {
            return Err(DistributionError::new(
                "alpha must be positive / α 必须为正",
            ));
        }
        if self.beta <= 0.0 {
            return Err(DistributionError::new(
                "beta must be positive / β 必须为正",
            ));
        }
        Ok(())
    }

    fn sample(&self, entropy: &mut dyn Entropy) -> f64 {
        let x = Gamma::new(self.alpha, 1.0).sample(entropy);
        let y = Gamma::new(self.beta, 1.0).sample(entropy);
        let sum = x + y;
        if sum <= 0.0 { 0.5 } else { x / sum }
    }
}
