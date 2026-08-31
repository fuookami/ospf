//! Gauss 映射。
//! Gauss map.

use crate::algebra::Field;
use num_traits::Float;

scalar_map!(
    /// Gauss 映射。
    /// Gauss map.
    GaussMap,
    /// Gauss 映射序列生成器。
    /// Gauss map sequence generator.
    GaussMapGenerator, [mu], |system, state| {
    if state == S::zero() {
        S::zero()
    } else {
        system.mu / state
    }
});

impl<S: Field + Float> Default for GaussMap<S> {
    fn default() -> Self {
        Self::new(S::one())
    }
}

impl<S: Field + Float> Default for GaussMapGenerator<S> {
    fn default() -> Self {
        Self::new(GaussMap::default(), S::one())
    }
}

/// 创建 Gauss 映射。
/// Create a Gauss map.
pub fn gauss_map<S: Field + Float>(mu: S) -> GaussMap<S> {
    GaussMap::new(mu)
}

/// 创建 Gauss 映射生成器。
/// Create a Gauss map generator.
pub fn gauss_map_generator<S: Field + Float>(mu: S, x: S) -> GaussMapGenerator<S> {
    GaussMapGenerator::new(GaussMap::new(mu), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn scalar_maps_match_kotlin_formulas() {
        assert_close(GaussMap::new(2.0_f64).step(4.0), 0.5);
        assert_close(GaussMap::new(2.0_f64).step(0.0), 0.0);
    }

    #[test]
    fn generators_return_current_value_before_advancing() {
        let mut scalar = GaussMapGenerator::new(GaussMap::new(2.0), 4.0);
        assert_eq!(scalar.next_value(), 4.0);
        assert_close(scalar.x(), 0.5);
    }
}