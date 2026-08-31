//! Chebyshev 映射。
//! Chebyshev map.

use crate::algebra::Field;
use num_traits::Float;

scalar_map!(
    /// Chebyshev 映射。
    /// Chebyshev map.
    ChebyshevMap,
    /// Chebyshev 映射序列生成器。
    /// Chebyshev map sequence generator.
    ChebyshevMapGenerator, [a], |system, state| {
    if state >= -S::one() && state <= S::one() {
        (system.a * state.acos()).cos()
    } else {
        S::zero()
    }
});

impl<S: Field + Float> Default for ChebyshevMap<S> {
    fn default() -> Self {
        Self::new(S::one() + S::one())
    }
}

impl<S: Field + Float> Default for ChebyshevMapGenerator<S> {
    fn default() -> Self {
        Self::new(ChebyshevMap::default(), S::one() / (S::one() + S::one()))
    }
}

/// 创建 Chebyshev 映射。
/// Create a Chebyshev map.
pub fn chebyshev_map<S: Field + Float>(a: S) -> ChebyshevMap<S> {
    ChebyshevMap::new(a)
}

/// 创建 Chebyshev 映射生成器。
/// Create a Chebyshev map generator.
pub fn chebyshev_map_generator<S: Field + Float>(a: S, x: S) -> ChebyshevMapGenerator<S> {
    ChebyshevMapGenerator::new(ChebyshevMap::new(a), x)
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
        assert_close(ChebyshevMap::new(2.0_f64).step(0.5), -0.5);
        assert_close(ChebyshevMap::new(2.0_f64).step(2.0), 0.0);
    }
}
