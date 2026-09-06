//! 高斯迭代映射。
//! Gauss iterated map.

use super::helpers::default_float;
use crate::algebra::Field;
use num_traits::Float;

scalar_map!(
    /// 高斯迭代映射。
    /// Gauss iterated map.
    GaussIteratedMap,
    /// 高斯迭代映射序列生成器。
    /// Gauss iterated map sequence generator.
    GaussIteratedMapGenerator,
    [a, b],
    |system, state| {
        (-system.a * state * state).exp() + system.b
    }
);

impl<S: Field + Float> Default for GaussIteratedMap<S> {
    fn default() -> Self {
        Self::new(
            default_float(4.9, "4.9 must be representable"),
            default_float(0.0, "0.0 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for GaussIteratedMapGenerator<S> {
    fn default() -> Self {
        Self::new(
            GaussIteratedMap::default(),
            S::from(0.5).expect("0.5 must be representable"),
        )
    }
}

/// 创建高斯迭代映射。
/// Create a Gauss iterated map.
pub fn gauss_iterated_map<S: Field + Float>(a: S, b: S) -> GaussIteratedMap<S> {
    GaussIteratedMap::new(a, b)
}

/// 创建高斯迭代映射生成器。
/// Create a Gauss iterated map generator.
pub fn gauss_iterated_map_generator<S: Field + Float>(
    a: S,
    b: S,
    x: S,
) -> GaussIteratedMapGenerator<S> {
    GaussIteratedMapGenerator::new(GaussIteratedMap::new(a, b), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauss_iterated_step_formula() {
        let system = GaussIteratedMap::new(4.9_f64, 0.0);
        let next = system.step(0.5);
        let expected = (-4.9 * 0.25_f64).exp();
        assert!((next - expected).abs() < 1e-12);
    }
}
