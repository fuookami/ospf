//! Liu-Chen 吸引子。
//! Liu-Chen attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Liu-Chen 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Liu-Chen attractor.
    LiuChenAttractor,
    /// Liu-Chen 吸引子序列生成器。
    /// Liu-Chen attractor sequence generator.
    LiuChenAttractorGenerator,
    [alpha, beta, delta, epsilon, zeta, xi, rho, h],
    |system, state| {
        let dx = system.alpha * state.y() + system.beta * state.x() + system.zeta * state.y() * state.z();
        let dy = system.delta * state.y() - state.z() + system.epsilon * state.x() * state.z();
        let dz = system.xi * state.z() + system.rho * state.x() * state.y();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for LiuChenAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(2.4, "2.4 must be representable"),
            default_float(-3.78, "-3.78 must be representable"),
            default_float(14.0, "14.0 must be representable"),
            default_float(-11.0, "-11.0 must be representable"),
            default_float(4.0, "4.0 must be representable"),
            default_float(5.58, "5.58 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for LiuChenAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(LiuChenAttractor::default(), one_point3())
    }
}

/// 创建 Liu-Chen 吸引子。
/// Create a Liu-Chen attractor.
pub fn liu_chen_attractor<S: Field + Float>(
    alpha: S, beta: S, delta: S, epsilon: S, zeta: S, xi: S, rho: S, h: S,
) -> LiuChenAttractor<S> {
    LiuChenAttractor::new(alpha, beta, delta, epsilon, zeta, xi, rho, h)
}

/// 创建 Liu-Chen 吸引子生成器。
/// Create a Liu-Chen attractor generator.
pub fn liu_chen_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, delta: S, epsilon: S, zeta: S, xi: S, rho: S, h: S, x: Point3<S>,
) -> LiuChenAttractorGenerator<S> {
    LiuChenAttractorGenerator::new(LiuChenAttractor::new(alpha, beta, delta, epsilon, zeta, xi, rho, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liu_chen_step_formula() {
        let system = LiuChenAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = 2.4 - 3.78 + 4.0;
        let dy = 14.0 - 1.0 - 11.0;
        let dz = 5.58 + 1.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
