//! Hadley 吸引子。
//! Hadley attractor.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Hadley 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Hadley attractor.
    HadleyAttractor,
    /// Hadley 吸引子序列生成器。
    /// Hadley attractor sequence generator.
    HadleyAttractorGenerator,
    [alpha, beta, delta, zeta, h],
    |system, state| {
        let dx = -state.y() * state.y() - state.z() * state.z() - system.alpha * state.x()
            + system.alpha * system.zeta;
        let dy = state.x() * state.y() - system.beta * state.x() * state.z() - state.y() + system.delta;
        let dz = system.beta * state.x() * state.y() + state.x() * state.z() - state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for HadleyAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.2, "0.2 must be representable"),
            default_float(4.0, "4.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(8.0, "8.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for HadleyAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(HadleyAttractor::default(), one_point3())
    }
}

/// 创建 Hadley 吸引子。
/// Create a Hadley attractor.
pub fn hadley_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    zeta: S,
    h: S,
) -> HadleyAttractor<S> {
    HadleyAttractor::new(alpha, beta, delta, zeta, h)
}

/// 创建 Hadley 吸引子生成器。
/// Create a Hadley attractor generator.
pub fn hadley_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    zeta: S,
    h: S,
    x: Point3<S>,
) -> HadleyAttractorGenerator<S> {
    HadleyAttractorGenerator::new(HadleyAttractor::new(alpha, beta, delta, zeta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hadley_step_formula() {
        let system = HadleyAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = -1.0 - 1.0 - 0.2 + 1.6;
        let dy = 1.0 - 4.0 - 1.0 + 1.0;
        let dz = 4.0 + 1.0 - 1.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
