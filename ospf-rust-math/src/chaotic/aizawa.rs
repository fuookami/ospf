//! Aizawa attractor system.
//! Aizawa 吸引子系统。

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Aizawa 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Aizawa attractor.
    AizawaAttractor,
    /// Aizawa 吸引子序列生成器。
    /// Aizawa attractor sequence generator.
    AizawaAttractorGenerator,
    [alpha, beta, gamma, delta, epsilon, zeta, h],
    |system, state| {
        let three = S::one() + S::one() + S::one();
        let dy = system.delta * state.x() + (state.z() - system.beta) * state.y();
        let dx = (state.z() - system.beta) * state.x() - dy;
        let dz = system.gamma + system.alpha * state.x()
            - state.z().powi(3) / three
            - (state.x() * state.x() + state.y() * state.y())
                * (S::one() + system.epsilon * state.z())
            + system.zeta * state.z() * state.x().powi(3);
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dz,
            state.z() + system.h * dy,
        )
    }
);

impl<S: Field + Float> Default for AizawaAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.95, "0.95 must be representable"),
            default_float(0.7, "0.7 must be representable"),
            default_float(0.6, "0.6 must be representable"),
            default_float(3.5, "3.5 must be representable"),
            default_float(0.25, "0.25 must be representable"),
            default_float(0.1, "0.1 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for AizawaAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(AizawaAttractor::default(), one_point3())
    }
}

/// 创建 Aizawa 吸引子。
/// Create an Aizawa attractor.
pub fn aizawa_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    gamma: S,
    delta: S,
    epsilon: S,
    zeta: S,
    h: S,
) -> AizawaAttractor<S> {
    AizawaAttractor::new(alpha, beta, gamma, delta, epsilon, zeta, h)
}

/// 创建 Aizawa 吸引子生成器。
/// Create an Aizawa attractor generator.
pub fn aizawa_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    gamma: S,
    delta: S,
    epsilon: S,
    zeta: S,
    h: S,
    x: Point3<S>,
) -> AizawaAttractorGenerator<S> {
    AizawaAttractorGenerator::new(
        AizawaAttractor::new(alpha, beta, gamma, delta, epsilon, zeta, h),
        x,
    )
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
    fn aizawa_attractor_matches_kotlin_formula() {
        assert_point3_close(
            AizawaAttractor::default().step(Point3::new(1.0, 1.0, 1.0)),
            Point3::new(0.965, 0.9881666666666666, 1.038),
        );
    }

    #[test]
    fn generator_returns_current_value_before_advancing() {
        let mut generator = AizawaAttractorGenerator::<f64>::default();
        let first = generator.next_point();
        assert_eq!(first, one_point3());
    }
}
