//! Bouali attractor system.
//! Bouali 吸引子系统。

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Bouali 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Bouali attractor.
    BoualiAttractor,
    /// Bouali 吸引子序列生成器。
    /// Bouali attractor sequence generator.
    BoualiAttractorGenerator,
    [alpha, zeta, h],
    |system, state| {
        let half = S::one() / (S::one() + S::one());
        let four = S::one() + S::one() + S::one() + S::one();
        let one_point_five = S::one() + half;
        let dx = state.x() * (four - state.y()) + system.alpha * state.z();
        let dy = -state.y() * (S::one() - state.x() * state.x());
        let dz = -state.x() * (one_point_five - system.zeta * state.z())
            - default_float::<S>(0.05, "0.05 must be representable") * state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for BoualiAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.3, "0.3 must be representable"),
            S::one(),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for BoualiAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(BoualiAttractor::default(), one_point3())
    }
}

/// 创建 Bouali 吸引子。
/// Create a Bouali attractor.
pub fn bouali_attractor<S: Field + Float>(alpha: S, zeta: S, h: S) -> BoualiAttractor<S> {
    BoualiAttractor::new(alpha, zeta, h)
}

/// 创建 Bouali 吸引子生成器。
/// Create a Bouali attractor generator.
pub fn bouali_attractor_generator<S: Field + Float>(
    alpha: S,
    zeta: S,
    h: S,
    x: Point3<S>,
) -> BoualiAttractorGenerator<S> {
    BoualiAttractorGenerator::new(BoualiAttractor::new(alpha, zeta, h), x)
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

    fn assert_point3_close(actual: Point3<f64>, expected: Point3<f64>) {
        assert_close(actual.x(), expected.x());
        assert_close(actual.y(), expected.y());
        assert_close(actual.z(), expected.z());
    }

    #[test]
    fn bouali_attractor_matches_kotlin_formula() {
        assert_point3_close(
            BoualiAttractor::default().step(Point3::new(1.0, 1.0, 1.0)),
            Point3::new(1.033, 1.0, 0.9945),
        );
    }

    #[test]
    fn generator_returns_current_value_before_advancing() {
        let mut generator = BoualiAttractorGenerator::<f64>::default();
        let first = generator.next_point();
        assert_eq!(first, one_point3());
    }
}
