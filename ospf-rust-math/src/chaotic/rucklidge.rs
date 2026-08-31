//! Rucklidge 吸引子。
//! Rucklidge attractor.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Rucklidge 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Rucklidge attractor.
    RucklidgeAttractor,
    /// Rucklidge 吸引子序列生成器。
    /// Rucklidge attractor sequence generator.
    RucklidgeAttractorGenerator,
    [alpha, kappa, h],
    |system, state| {
        let dx = -system.kappa * state.x() + system.alpha * state.y() - state.y() * state.z();
        let dy = state.x();
        let dz = -state.z() + state.y() * state.y();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for RucklidgeAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(2.0, "2.0 must be representable"),
            default_float(6.7, "6.7 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for RucklidgeAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(RucklidgeAttractor::default(), one_point3())
    }
}

/// 创建 Rucklidge 吸引子。
/// Create a Rucklidge attractor.
pub fn rucklidge_attractor<S: Field + Float>(alpha: S, kappa: S, h: S) -> RucklidgeAttractor<S> {
    RucklidgeAttractor::new(alpha, kappa, h)
}

/// 创建 Rucklidge 吸引子生成器。
/// Create a Rucklidge attractor generator.
pub fn rucklidge_attractor_generator<S: Field + Float>(
    alpha: S,
    kappa: S,
    h: S,
    x: Point3<S>,
) -> RucklidgeAttractorGenerator<S> {
    RucklidgeAttractorGenerator::new(RucklidgeAttractor::new(alpha, kappa, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rucklidge_step_formula() {
        let system = RucklidgeAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = -6.7 + 2.0 - 1.0;
        let dy = 1.0;
        let dz = -1.0 + 1.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
