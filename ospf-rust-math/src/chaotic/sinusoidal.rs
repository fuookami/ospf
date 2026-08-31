//! 正弦平方映射。
//! Sinusoidal map.

use super::helpers::default_float;
use crate::algebra::Field;
use num_traits::Float;

scalar_map!(
    /// 正弦平方映射。
    /// Sinusoidal map.
    SinusoidalMap,
    /// 正弦平方映射序列生成器。
    /// Sinusoidal map sequence generator.
    SinusoidalMapGenerator,
    [mu],
    |system, state| {
        let pi = S::from(std::f64::consts::PI).expect("PI must be representable");
        system.mu * state * state * (pi * state).sin()
    }
);

impl<S: Field + Float> Default for SinusoidalMap<S> {
    fn default() -> Self {
        Self::new(default_float(2.3, "2.3 must be representable"))
    }
}

impl<S: Field + Float> Default for SinusoidalMapGenerator<S> {
    fn default() -> Self {
        Self::new(
            SinusoidalMap::default(),
            S::from(0.5).expect("0.5 must be representable"),
        )
    }
}

/// 创建正弦平方映射。
/// Create a sinusoidal map.
pub fn sinusoidal_map<S: Field + Float>(mu: S) -> SinusoidalMap<S> {
    SinusoidalMap::new(mu)
}

/// 创建正弦平方映射生成器。
/// Create a sinusoidal map generator.
pub fn sinusoidal_map_generator<S: Field + Float>(mu: S, x: S) -> SinusoidalMapGenerator<S> {
    SinusoidalMapGenerator::new(SinusoidalMap::new(mu), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sinusoidal_step_formula() {
        let system = SinusoidalMap::new(2.3_f64);
        let next = system.step(0.5);
        let expected = 2.3 * 0.25 * (std::f64::consts::PI * 0.5).sin();
        assert!((next - expected).abs() < 1e-12);
    }
}
