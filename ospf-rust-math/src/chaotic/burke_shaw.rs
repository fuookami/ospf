//! Burke-Shaw attractor system.
//! Burke-Shaw 吸引子系统。

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Burke-Shaw 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Burke-Shaw attractor.
    BurkeShawAttractor,
    /// Burke-Shaw 吸引子序列生成器。
    /// Burke-Shaw attractor sequence generator.
    BurkeShawAttractorGenerator,
    [zeta, nu, h],
    |system, state| {
        let dx = -system.zeta * (state.x() + state.y());
        let dy = -state.y() - system.zeta * state.x() * state.z();
        let dz = system.zeta * state.x() * state.y() + system.nu;
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for BurkeShawAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(10.0, "10.0 must be representable"),
            default_float(4.272, "4.272 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for BurkeShawAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(BurkeShawAttractor::default(), one_point3())
    }
}

/// 创建 Burke-Shaw 吸引子。
/// Create a Burke-Shaw attractor.
pub fn burke_shaw_attractor<S: Field + Float>(zeta: S, nu: S, h: S) -> BurkeShawAttractor<S> {
    BurkeShawAttractor::new(zeta, nu, h)
}

/// 创建 Burke-Shaw 吸引子生成器。
/// Create a Burke-Shaw attractor generator.
pub fn burke_shaw_attractor_generator<S: Field + Float>(
    zeta: S,
    nu: S,
    h: S,
    x: Point3<S>,
) -> BurkeShawAttractorGenerator<S> {
    BurkeShawAttractorGenerator::new(BurkeShawAttractor::new(zeta, nu, h), x)
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
    fn burke_shaw_attractor_matches_kotlin_formula() {
        assert_point3_close(
            BurkeShawAttractor::default().step(Point3::new(1.0, 1.0, 1.0)),
            Point3::new(0.8, 0.89, 1.14272),
        );
    }

    #[test]
    fn generator_returns_current_value_before_advancing() {
        let mut generator = BurkeShawAttractorGenerator::<f64>::default();
        let first = generator.next_point();
        assert_eq!(first, one_point3());
    }
}
