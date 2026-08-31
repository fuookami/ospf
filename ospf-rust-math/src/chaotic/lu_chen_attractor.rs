//! Lu-Chen 吸引子。
//! Lu-Chen attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Lu-Chen 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Lu-Chen attractor.
    LuChenAttractor,
    /// Lu-Chen 吸引子序列生成器。
    /// Lu-Chen attractor sequence generator.
    LuChenAttractorGenerator,
    [alpha, beta, zeta, h],
    |system, state| {
        let dx = -(system.alpha * system.beta) / (system.alpha + system.beta) * state.x()
            - state.y() * state.z() + system.zeta;
        let dy = system.alpha * state.y() + state.x() * state.z();
        let dz = system.beta * state.z() + state.x() * state.y();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for LuChenAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(-10.0, "-10.0 must be representable"),
            default_float(-4.0, "-4.0 must be representable"),
            default_float(18.1, "18.1 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for LuChenAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(LuChenAttractor::default(), one_point3())
    }
}

/// 创建 Lu-Chen 吸引子。
/// Create a Lu-Chen attractor.
pub fn lu_chen_attractor<S: Field + Float>(alpha: S, beta: S, zeta: S, h: S) -> LuChenAttractor<S> {
    LuChenAttractor::new(alpha, beta, zeta, h)
}

/// 创建 Lu-Chen 吸引子生成器。
/// Create a Lu-Chen attractor generator.
pub fn lu_chen_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, zeta: S, h: S, x: Point3<S>,
) -> LuChenAttractorGenerator<S> {
    LuChenAttractorGenerator::new(LuChenAttractor::new(alpha, beta, zeta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lu_chen_attractor_step_formula() {
        let system = LuChenAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = -(-10.0 * -4.0) / (-10.0 + -4.0) - 1.0 + 18.1;
        let dy = -10.0 + 1.0;
        let dz = -4.0 + 1.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
