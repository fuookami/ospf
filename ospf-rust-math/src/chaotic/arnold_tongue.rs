//! Arnold 舌映射。
//! Arnold tongue map.

use super::helpers::default_float;
use crate::algebra::Field;
use num_traits::Float;

scalar_map!(
    /// Arnold 舌映射的一阶欧拉步进模型。
    /// First-order Euler step model for the Arnold tongue map.
    ArnoldTongue,
    /// Arnold 舌映射序列生成器。
    /// Arnold tongue map sequence generator.
    ArnoldTongueGenerator,
    [omega, kappa],
    |system, state| {
        let pi2 = default_float::<S>(std::f64::consts::PI * 2.0, "2 pi must be representable");
        state + system.omega - system.kappa / pi2 * (pi2 * state).sin()
    }
);

impl<S: Field + Float> Default for ArnoldTongue<S> {
    fn default() -> Self {
        Self::new(
            S::one() / (S::one() + S::one()),
            default_float(std::f64::consts::PI, "pi must be representable"),
        )
    }
}

impl<S: Field + Float> Default for ArnoldTongueGenerator<S> {
    fn default() -> Self {
        Self::new(ArnoldTongue::default(), S::one() / (S::one() + S::one()))
    }
}

/// 创建 Arnold 舌映射。
/// Create an Arnold tongue map.
pub fn arnold_tongue<S: Field + Float>(omega: S, kappa: S) -> ArnoldTongue<S> {
    ArnoldTongue::new(omega, kappa)
}

/// 创建 Arnold 舌映射生成器。
/// Create an Arnold tongue map generator.
pub fn arnold_tongue_generator<S: Field + Float>(
    omega: S,
    kappa: S,
    x: S,
) -> ArnoldTongueGenerator<S> {
    ArnoldTongueGenerator::new(ArnoldTongue::new(omega, kappa), x)
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
        assert_close(ArnoldTongue::new(0.25_f64, 0.0).step(0.5), 0.75);
    }
}