//! 概率分布 / Probability distributions
//!
//! 所有分布以「熵源注入」的方式采样：调用方提供 [`Entropy`] 作为均匀随机源，从而让随机性可替换
//! （伪随机 / 混沌序列 / 固定种子可复现）。
//! Every distribution samples through an injected [`Entropy`] uniform source, making randomness replaceable
//! (pseudo-random / chaotic sequence / reproducible with a fixed seed).
//!
//! 参数校验通过 [`Distribution::validate`] 显式返回，采样通过 [`Distribution::sample`] 返回数值，
//! 不做隐式异常。
//! Parameter validation is returned explicitly by [`Distribution::validate`], sampling returns a number through
//! [`Distribution::sample`], and nothing panics implicitly.

use std::fmt::{Display, Formatter};

pub mod beta;
pub mod cauchy;
pub mod direction;
pub mod entropy;
pub mod exponential;
pub mod gamma;
pub mod gaussian;
pub mod levy;
pub mod student_t;
pub mod truncated;
pub mod uniform;

pub use beta::Beta;
pub use cauchy::Cauchy;
pub use direction::DirectionSampler;
pub use entropy::Entropy;
pub use exponential::Exponential;
pub use gamma::Gamma;
pub use gaussian::Gaussian;
pub use levy::Levy;
pub use student_t::StudentT;
pub use truncated::Truncated;
pub use uniform::Uniform;

/// 规避 0 与 1 端点的极小正数 / Tiny positive value used to avoid the 0 and 1 endpoints
pub const UNIT_EPSILON: f64 = 1.0e-12;

/// 分布错误 / Distribution error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionError {
    message: String,
}

impl DistributionError {
    /// 创建分布错误 / Create a distribution error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 错误信息 / Error message
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for DistributionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DistributionError {}

/// 分布结果 / Distribution result
pub type DistributionResult<T> = std::result::Result<T, DistributionError>;

/// 概率分布 / Probability distribution
pub trait Distribution {
    /// 校验分布参数 / Validate distribution parameters
    fn validate(&self) -> DistributionResult<()>;

    /// 执行数值采样，假定参数已通过校验 / Perform numeric sampling, assuming the parameters were validated
    fn sample(&self, entropy: &mut dyn Entropy) -> f64;

    /// 校验后采样 / Validate then sample
    fn sample_checked(&self, entropy: &mut dyn Entropy) -> DistributionResult<f64> {
        self.validate()?;
        Ok(self.sample(entropy))
    }
}

/// 读取 `[0, 1)` 区间的均匀随机数，越界时裁剪 / Read a uniform random number in `[0, 1)`, clamped when out of range
pub fn uniform_unit(entropy: &mut dyn Entropy) -> f64 {
    let value = entropy.next_unit();
    if value.is_nan() || value <= 0.0 {
        0.0
    } else if value >= 1.0 {
        1.0 - UNIT_EPSILON
    } else {
        value
    }
}

/// 标准正态采样，使用 Box–Muller 变换 / Standard normal sampling using the Box–Muller transform
pub fn standard_gaussian(entropy: &mut dyn Entropy) -> f64 {
    let u1 = uniform_unit(entropy).max(UNIT_EPSILON);
    let u2 = uniform_unit(entropy);
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ConstantEntropy(f64);

    impl Entropy for ConstantEntropy {
        fn next_unit(&mut self) -> f64 {
            self.0
        }
    }

    struct CounterEntropy {
        state: u64,
    }

    impl CounterEntropy {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }
    }

    impl Entropy for CounterEntropy {
        fn next_unit(&mut self) -> f64 {
            self.state = self
                .state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.state >> 11) as f64) / ((1_u64 << 53) as f64)
        }
    }

    #[test]
    fn uniform_and_gaussian_samples_stay_in_contract() {
        let mut entropy = CounterEntropy::new(7);
        let uniform = Uniform::new(2.0, 5.0);
        for _ in 0..64 {
            let value = uniform.sample(&mut entropy);
            assert!((2.0..=5.0).contains(&value));
        }

        let gaussian = Gaussian::new(3.0, 2.0);
        for _ in 0..64 {
            let value = gaussian.sample(&mut entropy);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn validation_rejects_invalid_parameters() {
        assert!(Uniform::new(2.0, 1.0).validate().is_err());
        assert!(Gaussian::new(0.0, -1.0).validate().is_err());
        assert!(Exponential::new(0.0).validate().is_err());
        assert!(Gamma::new(0.0, 1.0).validate().is_err());
        assert!(Levy::new(0.0).validate().is_err());
        assert!(DirectionSampler::new(0).validate().is_err());
        assert!(Beta::new(1.0, 1.0).validate().is_ok());
        assert!(StudentT::new(3.0).validate().is_ok());
    }

    #[test]
    fn heavy_tailed_and_transformed_distributions_sample_finite_values() {
        let mut entropy = CounterEntropy::new(11);
        assert!(Cauchy::new(0.0, 1.0).sample(&mut entropy).is_finite());
        assert!(Levy::new(1.5).sample(&mut entropy).is_finite());
        assert!(Beta::new(2.0, 3.0).sample(&mut entropy).is_finite());
        assert!(StudentT::new(5.0).sample(&mut entropy).is_finite());

        let truncated = Truncated::new(Gaussian::new(0.0, 10.0), -1.0, 1.0, 512);
        for _ in 0..32 {
            let value = truncated.sample(&mut entropy);
            assert!((-1.0..=1.0).contains(&value));
        }
    }

    #[test]
    fn direction_sampler_returns_unit_vectors() {
        let mut entropy = CounterEntropy::new(13);
        let sampler = DirectionSampler::new(4);
        let direction = sampler.sample(&mut entropy);
        assert_eq!(direction.len(), 4);
        let norm = direction.iter().map(|value| value * value).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1.0e-9);
    }

    #[test]
    fn constant_entropy_does_not_hang_gamma_sampling() {
        let mut entropy = ConstantEntropy(0.5);
        let gamma = Gamma::new(2.5, 1.0);
        let value = gamma.sample(&mut entropy);
        assert!(value.is_finite());
    }
}
