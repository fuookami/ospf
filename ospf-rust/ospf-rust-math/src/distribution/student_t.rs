//! 学生 t 分布 / Student's t distribution

use super::{Distribution, DistributionError, DistributionResult, Entropy, Gamma, standard_gaussian};

/// 学生 t 分布 / Student's t distribution
///
/// 自由度趋于无穷时退化为标准正态，自由度较小时具有更重的尾部。
/// Degenerates to the standard normal as the degrees of freedom grow, and has heavier tails for small degrees of freedom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StudentT {
    /// 自由度 ν / Degrees of freedom ν
    pub degrees_of_freedom: f64,
}

impl StudentT {
    /// 创建学生 t 分布 / Create a Student's t distribution
    pub fn new(degrees_of_freedom: f64) -> Self {
        Self {
            degrees_of_freedom,
        }
    }
}

impl Default for StudentT {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl Distribution for StudentT {
    fn validate(&self) -> DistributionResult<()> {
        if self.degrees_of_freedom > 0.0 {
            Ok(())
        } else {
            Err(DistributionError::new(
                "degreesOfFreedom must be positive / 自由度必须为正",
            ))
        }
    }

    fn sample(&self, entropy: &mut dyn Entropy) -> f64 {
        let dof = self.degrees_of_freedom;
        let z = standard_gaussian(entropy);
        let chi_squared = Gamma::new(dof / 2.0, 2.0).sample(entropy);
        if chi_squared <= 0.0 {
            return z;
        }
        z / (chi_squared / dof).sqrt()
    }
}
