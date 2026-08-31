//! 正弦映射。
//! Sine map.

use crate::algebra::Field;
use num_traits::Float;
scalar_map!(
    /// 正弦映射。
    /// Sine map.
    SineMap,
    /// 正弦映射序列生成器。
    /// Sine map sequence generator.
    SineMapGenerator,
    [mu],
    |system, state| {
        let pi = S::from(std::f64::consts::PI).expect("PI must be representable");
        system.mu * (pi * state).sin()
    }
);

impl<S: Field + Float> Default for SineMap<S> {
    fn default() -> Self {
        Self::new(S::one())
    }
}

impl<S: Field + Float> Default for SineMapGenerator<S> {
    fn default() -> Self {
        Self::new(
            SineMap::default(),
            S::from(0.5).expect("0.5 must be representable"),
        )
    }
}

/// 创建正弦映射。
/// Create a sine map.
pub fn sine_map<S: Field + Float>(mu: S) -> SineMap<S> {
    SineMap::new(mu)
}

/// 创建正弦映射生成器。
/// Create a sine map generator.
pub fn sine_map_generator<S: Field + Float>(mu: S, x: S) -> SineMapGenerator<S> {
    SineMapGenerator::new(SineMap::new(mu), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_step_formula() {
        let system = SineMap::new(1.0_f64);
        let next = system.step(0.5);
        assert!((next - (std::f64::consts::PI * 0.5).sin()).abs() < 1e-12);
    }
}
