//! Rayleigh-Benard 吸引子。
//! Rayleigh-Benard attractor.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Rayleigh-Benard 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Rayleigh-Benard attractor.
    RayleighBenardAttractor,
    /// Rayleigh-Benard 吸引子序列生成器。
    /// Rayleigh-Benard attractor sequence generator.
    RayleighBenardAttractorGenerator,
    [alpha, beta, gamma, h],
    |system, state| {
        let dx = -system.alpha * state.x() + system.alpha * state.y();
        let dy = system.gamma * state.x() - state.y() - state.x() * state.z();
        let dz = state.x() * state.y() - system.beta * state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for RayleighBenardAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(9.0, "9.0 must be representable"),
            default_float(5.0, "5.0 must be representable"),
            default_float(12.0, "12.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for RayleighBenardAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(RayleighBenardAttractor::default(), one_point3())
    }
}

/// 创建 Rayleigh-Benard 吸引子。
/// Create a Rayleigh-Benard attractor.
pub fn rayleigh_benard_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    gamma: S,
    h: S,
) -> RayleighBenardAttractor<S> {
    RayleighBenardAttractor::new(alpha, beta, gamma, h)
}

/// 创建 Rayleigh-Benard 吸引子生成器。
/// Create a Rayleigh-Benard attractor generator.
pub fn rayleigh_benard_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    gamma: S,
    h: S,
    x: Point3<S>,
) -> RayleighBenardAttractorGenerator<S> {
    RayleighBenardAttractorGenerator::new(RayleighBenardAttractor::new(alpha, beta, gamma, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rayleigh_benard_step_formula() {
        let system = RayleighBenardAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = -9.0 + 9.0;
        let dy = 12.0 - 1.0 - 1.0;
        let dz = 1.0 - 5.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
