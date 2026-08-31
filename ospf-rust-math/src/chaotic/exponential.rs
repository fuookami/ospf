//! 指数映射。
//! Exponential map.

use num_traits::Float;
use crate::algebra::Field;
use super::helpers::default_float;

scalar_map!(
    /// 指数映射。
    /// Exponential map.
    ExponentialMap,
    /// 指数映射序列生成器。
    /// Exponential map sequence generator.
    ExponentialMapGenerator,
    [c],
    |system, state| {
        state.exp() + system.c
    }
);

impl<S: Field + Float> Default for ExponentialMap<S> {
    fn default() -> Self {
        Self::new(default_float(0.5, "0.5 must be representable"))
    }
}

impl<S: Field + Float> Default for ExponentialMapGenerator<S> {
    fn default() -> Self {
        Self::new(ExponentialMap::default(), S::from(0.5).expect("0.5 must be representable"))
    }
}

/// 创建指数映射。
/// Create an exponential map.
pub fn exponential_map<S: Field + Float>(c: S) -> ExponentialMap<S> {
    ExponentialMap::new(c)
}

/// 创建指数映射生成器。
/// Create an exponential map generator.
pub fn exponential_map_generator<S: Field + Float>(c: S, z: S) -> ExponentialMapGenerator<S> {
    ExponentialMapGenerator::new(ExponentialMap::new(c), z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_step_formula() {
        let system = ExponentialMap::new(0.5_f64);
        let next = system.step(1.0);
        assert!((next - (1.0_f64.exp() + 0.5)).abs() < 1e-12);
    }
}
