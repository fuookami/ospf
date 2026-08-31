//! Anishchenko-Astakhov attractor system.
//! Anishchenko-Astakhov 吸引子系统。

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Anishchenko-Astakhov 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Anishchenko-Astakhov attractor.
    AnishchenkoAstakhovAttractor,
    /// Anishchenko-Astakhov 吸引子序列生成器。
    /// Anishchenko-Astakhov attractor sequence generator.
    AnishchenkoAstakhovAttractorGenerator,
    [mu, eta, h],
    |system, state| {
        let i = if state.x() >= S::zero() {
            S::one()
        } else {
            S::zero()
        };
        let dx = system.mu * state.x() + state.y() - state.x() * state.z();
        let dy = -state.x();
        let dz = -system.eta * state.z() + system.eta * i * state.x() * state.x();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for AnishchenkoAstakhovAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(1.2, "1.2 must be representable"),
            default_float(0.5, "0.5 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for AnishchenkoAstakhovAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(AnishchenkoAstakhovAttractor::default(), one_point3())
    }
}

/// 创建 Anishchenko-Astakhov 吸引子。
/// Create an Anishchenko-Astakhov attractor.
pub fn anishchenko_astakhov_attractor<S: Field + Float>(
    mu: S,
    eta: S,
    h: S,
) -> AnishchenkoAstakhovAttractor<S> {
    AnishchenkoAstakhovAttractor::new(mu, eta, h)
}

/// 创建 Anishchenko-Astakhov 吸引子生成器。
/// Create an Anishchenko-Astakhov attractor generator.
pub fn anishchenko_astakhov_attractor_generator<S: Field + Float>(
    mu: S,
    eta: S,
    h: S,
    x: Point3<S>,
) -> AnishchenkoAstakhovAttractorGenerator<S> {
    AnishchenkoAstakhovAttractorGenerator::new(AnishchenkoAstakhovAttractor::new(mu, eta, h), x)
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
    fn anishchenko_astakhov_attractor_matches_kotlin_formula() {
        assert_point3_close(
            AnishchenkoAstakhovAttractor::default().step(Point3::new(1.0, 1.0, 1.0)),
            Point3::new(1.012, 0.99, 1.0),
        );
    }

    #[test]
    fn generator_returns_current_value_before_advancing() {
        let mut generator = AnishchenkoAstakhovAttractorGenerator::<f64>::default();
        let first = generator.next_point();
        assert_eq!(first, one_point3());
    }
}
