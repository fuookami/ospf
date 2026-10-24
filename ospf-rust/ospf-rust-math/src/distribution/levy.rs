//! 莱维稳定分布 / Lévy stable distribution

use super::{
    Distribution, DistributionError, DistributionResult, Entropy, UNIT_EPSILON,
    standard_gaussian, uniform_unit,
};

/// 对称莱维稳定分布 / Symmetric Lévy stable distribution
///
/// 使用 Chambers–Mallows–Stuck 方法采样。`alpha = 1.5` 即莱维飞行的经典步长分布；`alpha` 趋于 2
/// 时退化为高斯分布。
/// Sampled with the Chambers–Mallows–Stuck method. `alpha = 1.5` yields the classic Lévy flight step distribution;
/// `alpha` approaching 2 degenerates to a Gaussian.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Levy {
    /// 稳定性参数 α ∈ (0, 2] / Stability parameter α ∈ (0, 2]
    pub alpha: f64,
}

impl Levy {
    /// 创建莱维稳定分布 / Create a Lévy stable distribution
    pub fn new(alpha: f64) -> Self {
        Self { alpha }
    }
}

impl Default for Levy {
    fn default() -> Self {
        Self::new(1.5)
    }
}

impl Distribution for Levy {
    fn validate(&self) -> DistributionResult<()> {
        if self.alpha > 0.0 && self.alpha <= 2.0 {
            Ok(())
        } else {
            Err(DistributionError::new(
                "alpha must be within (0, 2] / α 必须位于 (0, 2]",
            ))
        }
    }

    fn sample(&self, entropy: &mut dyn Entropy) -> f64 {
        let alpha = self.alpha;
        if alpha >= 2.0 {
            return standard_gaussian(entropy) * 2.0_f64.sqrt();
        }

        let u = uniform_unit(entropy).clamp(UNIT_EPSILON, 1.0 - UNIT_EPSILON);
        let v = (u - 0.5) * std::f64::consts::PI;
        if (alpha - 1.0).abs() <= f64::EPSILON {
            return v.tan();
        }

        let w = -uniform_unit(entropy).max(UNIT_EPSILON).ln();
        let first = (alpha * v).sin() / v.cos().powf(1.0 / alpha);
        let second = ((v - alpha * v).cos() / w).powf((1.0 - alpha) / alpha);
        first * second
    }
}
