//! 达德拉斯吸引子。
//! Dadras attractor.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// 达德拉斯吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Dadras attractor.
    DadrasAttractor,
    /// 达德拉斯吸引子序列生成器。
    /// Dadras attractor sequence generator.
    DadrasAttractorGenerator,
    [gamma, epsilon, zeta, rho, sigma, h],
    |system, state| {
        let dx = state.y() - system.rho * state.x() + system.sigma * state.y() * state.z();
        let dy = system.gamma * state.y() - state.x() * state.z() + state.z();
        let dz = system.zeta * state.x() * state.y() - system.epsilon * state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for DadrasAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(1.7, "1.7 must be representable"),
            default_float(9.0, "9.0 must be representable"),
            default_float(2.0, "2.0 must be representable"),
            default_float(3.0, "3.0 must be representable"),
            default_float(2.7, "2.7 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for DadrasAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(DadrasAttractor::default(), one_point3())
    }
}

/// 创建达德拉斯吸引子。
/// Create a Dadras attractor.
pub fn dadras_attractor<S: Field + Float>(
    gamma: S,
    epsilon: S,
    zeta: S,
    rho: S,
    sigma: S,
    h: S,
) -> DadrasAttractor<S> {
    DadrasAttractor::new(gamma, epsilon, zeta, rho, sigma, h)
}

/// 创建达德拉斯吸引子生成器。
/// Create a Dadras attractor generator.
pub fn dadras_attractor_generator<S: Field + Float>(
    gamma: S,
    epsilon: S,
    zeta: S,
    rho: S,
    sigma: S,
    h: S,
    x: Point3<S>,
) -> DadrasAttractorGenerator<S> {
    DadrasAttractorGenerator::new(DadrasAttractor::new(gamma, epsilon, zeta, rho, sigma, h), x)
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
    fn dadras_step_formula() {
        let system = DadrasAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        assert_close(next.x(), 1.0 + 0.01 * (1.0 - 3.0 * 1.0 + 2.7 * 1.0));
        assert_close(next.y(), 1.0 + 0.01 * (1.7 * 1.0 - 1.0 + 1.0));
        assert_close(next.z(), 1.0 + 0.01 * (2.0 - 9.0));
    }
}
