//! 伽马分布 / Gamma distribution

use super::{
    Distribution, DistributionError, DistributionResult, Entropy, UNIT_EPSILON,
    standard_gaussian, uniform_unit,
};

/// 伽马分布 / Gamma distribution
///
/// 使用 Marsaglia–Tsang 方法采样；形状参数小于 1 时通过 `Gamma(k + 1) * U^(1 / k)` 变换。
/// Sampled with the Marsaglia–Tsang method; when the shape is below 1 it is handled by the
/// `Gamma(k + 1) * U^(1 / k)` transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gamma {
    /// 形状参数 k / Shape parameter k
    pub shape: f64,
    /// 尺度参数 θ / Scale parameter θ
    pub scale: f64,
}

impl Gamma {
    /// 创建伽马分布 / Create a gamma distribution
    pub fn new(shape: f64, scale: f64) -> Self {
        Self { shape, scale }
    }

    /// 每次采样的最大拒绝次数，避免固定熵源导致死循环 / Maximum rejection attempts per sample, preventing an infinite loop with constant entropy
    const MAX_ATTEMPTS: usize = 4096;
}

impl Default for Gamma {
    fn default() -> Self {
        Self::new(1.0, 1.0)
    }
}

impl Distribution for Gamma {
    fn validate(&self) -> DistributionResult<()> {
        if self.shape <= 0.0 {
            return Err(DistributionError::new(
                "shape must be positive / 形状参数必须为正",
            ));
        }
        if self.scale <= 0.0 {
            return Err(DistributionError::new(
                "scale must be positive / 尺度参数必须为正",
            ));
        }
        Ok(())
    }

    fn sample(&self, entropy: &mut dyn Entropy) -> f64 {
        let k = self.shape;
        if k < 1.0 {
            let u = uniform_unit(entropy).max(UNIT_EPSILON);
            let base = Gamma::new(k + 1.0, self.scale).sample(entropy);
            return base * u.powf(1.0 / k);
        }

        let d = k - 1.0 / 3.0;
        let c = 1.0 / (9.0 * d).sqrt();
        for _ in 0..Self::MAX_ATTEMPTS {
            let x = standard_gaussian(entropy);
            let v = 1.0 + c * x;
            if v <= 0.0 {
                continue;
            }
            let v3 = v * v * v;
            let u = uniform_unit(entropy);
            if u <= 0.0 {
                continue;
            }
            if u.ln() < 0.5 * x * x + d - d * v3 + d * v3.ln() {
                return d * v3 * self.scale;
            }
        }
        // 拒绝采样失败时退化为均值，保证采样一定返回有限值。
        // Fall back to the mean when rejection sampling fails, guaranteeing a finite sample.
        d * self.scale
    }
}
