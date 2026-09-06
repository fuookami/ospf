//! Zaslavskii 映射。
//! Zaslavskii map.

use super::helpers::{default_float, one_point2};
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;

point2_system!(
    /// Zaslavskii 映射。
    /// Zaslavskii map.
    ZaslavskiiMap,
    /// Zaslavskii 映射序列生成器。
    /// Zaslavskii map sequence generator.
    ZaslavskiiMapGenerator,
    [epsilon, upsilon, r, mu, two_pi],
    |system, state| {
        let one = S::one();
        let cos_val = (system.two_pi * state.x()).cos();
        let exp_r = (-system.r).exp();
        let new_x = (state.x() + system.upsilon * (one + system.mu * state.y())
            + system.epsilon * system.upsilon * system.mu * cos_val)
            - (state.x() + system.upsilon * (one + system.mu * state.y())
                + system.epsilon * system.upsilon * system.mu * cos_val)
                .floor();
        let new_y = exp_r * (state.y() + system.epsilon * cos_val);
        Point2::new(new_x, new_y)
    }
);

impl<S: Field + Float> Default for ZaslavskiiMap<S> {
    fn default() -> Self {
        let epsilon = default_float(5.0, "5.0 must be representable");
        let upsilon = default_float(0.2, "0.2 must be representable");
        let r: S = default_float(2.0, "2.0 must be representable");
        let one = S::one();
        let mu = (one - (-r).exp()) / r;
        let two_pi = S::from(2.0 * std::f64::consts::PI).expect("2*PI must be representable");
        Self::new(epsilon, upsilon, r, mu, two_pi)
    }
}

impl<S: Field + Float> Default for ZaslavskiiMapGenerator<S> {
    fn default() -> Self {
        Self::new(ZaslavskiiMap::default(), one_point2())
    }
}

/// 创建 Zaslavskii 映射。
/// Create a Zaslavskii map.
pub fn zaslavskii_map<S: Field + Float>(
    epsilon: S,
    upsilon: S,
    r: S,
    mu: S,
    two_pi: S,
) -> ZaslavskiiMap<S> {
    ZaslavskiiMap::new(epsilon, upsilon, r, mu, two_pi)
}

/// 创建 Zaslavskii 映射生成器。
/// Create a Zaslavskii map generator.
pub fn zaslavskii_map_generator<S: Field + Float>(
    epsilon: S,
    upsilon: S,
    r: S,
    mu: S,
    two_pi: S,
    x: Point2<S>,
) -> ZaslavskiiMapGenerator<S> {
    ZaslavskiiMapGenerator::new(ZaslavskiiMap::new(epsilon, upsilon, r, mu, two_pi), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zaslavskii_default_mu() {
        let system = ZaslavskiiMap::<f64>::default();
        let expected_mu = (1.0 - (-2.0_f64).exp()) / 2.0;
        assert!((system.mu() - expected_mu).abs() < 1e-12);
    }
}
