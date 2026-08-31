//! Wang-Sun 吸引子。
//! Wang-Sun attractor.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Wang-Sun 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Wang-Sun attractor.
    WangSunAttractor,
    /// Wang-Sun 吸引子序列生成器。
    /// Wang-Sun attractor sequence generator.
    WangSunAttractorGenerator,
    [alpha, beta, delta, epsilon, zeta, xi, h],
    |system, state| {
        let dx = system.alpha * state.x() + system.zeta * state.y() * state.z();
        let dy = system.beta * state.x() + system.delta * state.y() - state.x() * state.z();
        let dz = system.epsilon * state.z() + system.xi * state.x() * state.y();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for WangSunAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.2, "0.2 must be representable"),
            default_float(0.001, "0.001 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(-0.4, "-0.4 must be representable"),
            default_float(-1.0, "-1.0 must be representable"),
            default_float(-1.0, "-1.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for WangSunAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(WangSunAttractor::default(), one_point3())
    }
}

/// 创建 Wang-Sun 吸引子。
/// Create a Wang-Sun attractor.
pub fn wang_sun_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    epsilon: S,
    zeta: S,
    xi: S,
    h: S,
) -> WangSunAttractor<S> {
    WangSunAttractor::new(alpha, beta, delta, epsilon, zeta, xi, h)
}

/// 创建 Wang-Sun 吸引子生成器。
/// Create a Wang-Sun attractor generator.
pub fn wang_sun_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    epsilon: S,
    zeta: S,
    xi: S,
    h: S,
    x: Point3<S>,
) -> WangSunAttractorGenerator<S> {
    WangSunAttractorGenerator::new(
        WangSunAttractor::new(alpha, beta, delta, epsilon, zeta, xi, h),
        x,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wang_sun_step_formula() {
        let system = WangSunAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = 0.2 - 1.0;
        let dy = 0.001 + 1.0 - 1.0;
        let dz = -0.4 - 1.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
