//! Rossler 吸引子。
//! Rossler attractor.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Rossler 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Rossler attractor.
    RosslerAttractor,
    /// Rossler 吸引子序列生成器。
    /// Rossler attractor sequence generator.
    RosslerAttractorGenerator,
    [alpha, beta, zeta, h],
    |system, state| {
        let dx = -state.y() - state.z();
        let dy = state.x() + system.alpha * state.y();
        let dz = system.beta + state.z() * (state.x() - system.zeta);
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for RosslerAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.2, "0.2 must be representable"),
            default_float(0.2, "0.2 must be representable"),
            default_float(5.7, "5.7 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for RosslerAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(RosslerAttractor::default(), one_point3())
    }
}

/// 创建 Rossler 吸引子。
/// Create a Rossler attractor.
pub fn rossler_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    zeta: S,
    h: S,
) -> RosslerAttractor<S> {
    RosslerAttractor::new(alpha, beta, zeta, h)
}

/// 创建 Rossler 吸引子生成器。
/// Create a Rossler attractor generator.
pub fn rossler_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    zeta: S,
    h: S,
    x: Point3<S>,
) -> RosslerAttractorGenerator<S> {
    RosslerAttractorGenerator::new(RosslerAttractor::new(alpha, beta, zeta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rossler_step_formula() {
        let system = RosslerAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = -1.0 - 1.0;
        let dy = 1.0 + 0.2;
        let dz = 0.2 + 1.0 * (1.0 - 5.7);
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
