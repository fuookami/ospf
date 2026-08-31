//! Sinus 映射。
//! Sinus map.
//!
//! 公式: x_{n+1} = 2.3 * x^(2*sin(pi*x))

use num_traits::Float;
use crate::algebra::Field;
use super::helpers::default_float;

scalar_map!(
    /// Sinus 映射。
    /// Sinus map.
    SinusMap,
    /// Sinus 映射序列生成器。
    /// Sinus map sequence generator.
    SinusMapGenerator,
    [c23, c2],
    |system, state| {
        let pi = S::from(std::f64::consts::PI).expect("PI must be representable");
        let sin_val = (pi * state).sin();
        let exponent = system.c2 * sin_val;
        system.c23 * state.powf(exponent)
    }
);

impl<S: Field + Float> Default for SinusMap<S> {
    fn default() -> Self {
        Self::new(
            default_float(2.3, "2.3 must be representable"),
            S::one() + S::one(),
        )
    }
}

impl<S: Field + Float> Default for SinusMapGenerator<S> {
    fn default() -> Self {
        Self::new(SinusMap::default(), S::from(0.5).expect("0.5 must be representable"))
    }
}

/// 创建 Sinus 映射。
/// Create a Sinus map.
pub fn sinus_map<S: Field + Float>(c23: S, c2: S) -> SinusMap<S> {
    SinusMap::new(c23, c2)
}

/// 创建 Sinus 映射生成器。
/// Create a Sinus map generator.
pub fn sinus_map_generator<S: Field + Float>(c23: S, c2: S, x: S) -> SinusMapGenerator<S> {
    SinusMapGenerator::new(SinusMap::new(c23, c2), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sinus_step_formula() {
        let system = SinusMap::<f64>::default();
        let next = system.step(0.5);
        let sin_val = (std::f64::consts::PI * 0.5).sin();
        let expected = 2.3 * 0.5_f64.powf(2.0 * sin_val);
        assert!((next - expected).abs() < 1e-12);
    }
}
