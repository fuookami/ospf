//! Arneodo attractor system.
//! Arneodo 吸引子系统。

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Arneodo 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Arneodo attractor.
    ArneodoAttractor,
    /// Arneodo 吸引子序列生成器。
    /// Arneodo attractor sequence generator.
    ArneodoAttractorGenerator,
    [alpha, beta, delta, h],
    |system, state| {
        let dx = state.y();
        let dy = state.z();
        let dz = -system.alpha * state.x() - system.beta * state.y() - state.z()
            + system.delta * state.x().powi(3);
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for ArneodoAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(-5.5, "-5.5 must be representable"),
            default_float(3.5, "3.5 must be representable"),
            default_float(-1.0, "-1.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for ArneodoAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(ArneodoAttractor::default(), one_point3())
    }
}

/// 创建 Arneodo 吸引子。
/// Create an Arneodo attractor.
pub fn arneodo_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    h: S,
) -> ArneodoAttractor<S> {
    ArneodoAttractor::new(alpha, beta, delta, h)
}

/// 创建 Arneodo 吸引子生成器。
/// Create an Arneodo attractor generator.
pub fn arneodo_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    h: S,
    x: Point3<S>,
) -> ArneodoAttractorGenerator<S> {
    ArneodoAttractorGenerator::new(ArneodoAttractor::new(alpha, beta, delta, h), x)
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
    fn arneodo_attractor_matches_kotlin_formula() {
        assert_point3_close(
            ArneodoAttractor::default().step(Point3::new(1.0, 1.0, 1.0)),
            Point3::new(1.01, 1.01, 1.0),
        );
    }

    #[test]
    fn generator_returns_current_value_before_advancing() {
        let mut generator = ArneodoAttractorGenerator::<f64>::default();
        let first = generator.next_point();
        assert_eq!(first, one_point3());
    }
}
