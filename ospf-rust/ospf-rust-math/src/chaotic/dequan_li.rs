//! Dequan Li 吸引子。
//! Dequan Li attractor.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Dequan Li 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Dequan Li attractor.
    DequanLiAttractor,
    /// Dequan Li 吸引子序列生成器。
    /// Dequan Li attractor sequence generator.
    DequanLiAttractorGenerator,
    [alpha, beta, delta, epsilon, zeta, rho, h],
    |system, state| {
        let dx = system.alpha * (state.y() - state.x()) + system.delta * state.x() * state.z();
        let dy = system.rho * state.x() + system.zeta * state.y() - state.x() * state.z();
        let dz = system.beta * state.z() + state.x() * state.y() - system.epsilon * state.x() * state.x();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for DequanLiAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(40.0, "40.0 must be representable"),
            default_float(1.833, "1.833 must be representable"),
            default_float(0.16, "0.16 must be representable"),
            default_float(0.65, "0.65 must be representable"),
            default_float(20.0, "20.0 must be representable"),
            default_float(55.0, "55.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for DequanLiAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(DequanLiAttractor::default(), one_point3())
    }
}

/// 创建 Dequan Li 吸引子。
/// Create a Dequan Li attractor.
pub fn dequan_li_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    epsilon: S,
    zeta: S,
    rho: S,
    h: S,
) -> DequanLiAttractor<S> {
    DequanLiAttractor::new(alpha, beta, delta, epsilon, zeta, rho, h)
}

/// 创建 Dequan Li 吸引子生成器。
/// Create a Dequan Li attractor generator.
pub fn dequan_li_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    epsilon: S,
    zeta: S,
    rho: S,
    h: S,
    x: Point3<S>,
) -> DequanLiAttractorGenerator<S> {
    DequanLiAttractorGenerator::new(
        DequanLiAttractor::new(alpha, beta, delta, epsilon, zeta, rho, h),
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

    #[test]
    fn dequan_li_step_formula() {
        let system = DequanLiAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = 40.0 * (1.0 - 1.0) + 0.16 * 1.0;
        let dy = 55.0 + 20.0 - 1.0;
        let dz = 1.833 + 1.0 - 0.65;
        assert_close(next.x(), 1.0 + 0.01 * dx);
        assert_close(next.y(), 1.0 + 0.01 * dy);
        assert_close(next.z(), 1.0 + 0.01 * dz);
    }
}
