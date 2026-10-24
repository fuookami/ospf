//! 方向采样器 / Direction sampler

use super::{
    DistributionError, DistributionResult, Entropy, UNIT_EPSILON, standard_gaussian,
};

/// 单位超球面方向采样器 / Unit hypersphere direction sampler
///
/// 逐维采样标准正态后归一化，得到单位超球面上均匀分布的方向向量；与 [`super::Levy`] 组合即可得到
/// 完整的高维莱维飞行步长。
/// Samples per-dimension standard normals and normalizes them into a uniformly distributed direction on the unit
/// hypersphere; combining it with [`super::Levy`] yields a complete high-dimensional Lévy flight step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectionSampler {
    /// 维度 / Dimensionality
    pub dimension: usize,
}

impl DirectionSampler {
    /// 创建方向采样器 / Create a direction sampler
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    /// 校验采样维度 / Validate the sampling dimension
    pub fn validate(&self) -> DistributionResult<()> {
        if self.dimension > 0 {
            Ok(())
        } else {
            Err(DistributionError::new(
                "dimension must be positive / 维度必须为正",
            ))
        }
    }

    /// 采样一个单位方向向量，假定维度已校验 / Sample a unit direction vector, assuming the dimension was validated
    pub fn sample(&self, entropy: &mut dyn Entropy) -> Vec<f64> {
        let raw: Vec<f64> = (0..self.dimension)
            .map(|_| standard_gaussian(entropy))
            .collect();
        let norm = raw.iter().map(|value| value * value).sum::<f64>().sqrt();
        if norm <= UNIT_EPSILON {
            let mut fallback = vec![0.0; self.dimension];
            fallback[0] = 1.0;
            fallback
        } else {
            raw.into_iter().map(|value| value / norm).collect()
        }
    }

    /// 校验后采样 / Validate then sample
    pub fn sample_checked(
        &self,
        entropy: &mut dyn Entropy,
    ) -> DistributionResult<Vec<f64>> {
        self.validate()?;
        Ok(self.sample(entropy))
    }
}
