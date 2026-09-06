//! Qi 吸引子（四维超混沌）。
//! Qi attractor (4D hyperchaotic).

use super::helpers::{default_float, one_point4};
use crate::algebra::Field;
use crate::geometry::Point4;
use num_traits::Float;

point4_system!(
    /// Qi 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Qi attractor.
    QiAttractor,
    /// Qi 吸引子序列生成器。
    /// Qi attractor sequence generator.
    QiAttractorGenerator,
    [alpha, beta, delta, zeta, h],
    |system, state| {
        let dx = system.alpha * (state.y() - state.x()) + state.y() * state.z() * state.w();
        let dy = system.beta * (state.x() + state.y()) - state.x() * state.z() * state.w();
        let dz = -system.zeta * state.z() + state.x() * state.y() * state.w();
        let dw = -system.delta * state.w() + state.x() * state.y() * state.z();
        Point4::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
            state.w() + system.h * dw,
        )
    }
);

impl<S: Field + Float> Default for QiAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(30.0, "30.0 must be representable"),
            default_float(10.0, "10.0 must be representable"),
            default_float(10.0, "10.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(0.001, "0.001 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for QiAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(QiAttractor::default(), one_point4())
    }
}

/// 创建 Qi 吸引子。
/// Create a Qi attractor.
pub fn qi_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    zeta: S,
    h: S,
) -> QiAttractor<S> {
    QiAttractor::new(alpha, beta, delta, zeta, h)
}

/// 创建 Qi 吸引子生成器。
/// Create a Qi attractor generator.
pub fn qi_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    zeta: S,
    h: S,
    x: Point4<S>,
) -> QiAttractorGenerator<S> {
    QiAttractorGenerator::new(QiAttractor::new(alpha, beta, delta, zeta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qi_attractor_step_formula() {
        let system = QiAttractor::<f64>::default();
        let next = system.step(Point4::new(1.0, 1.0, 1.0, 1.0));
        let dx = 30.0 * (1.0 - 1.0) + 1.0;
        let dy = 10.0 * (1.0 + 1.0) - 1.0;
        let dz = -1.0 + 1.0;
        let dw = -10.0 + 1.0;
        assert!((next.x() - (1.0 + 0.001 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.001 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.001 * dz)).abs() < 1e-12);
        assert!((next.w() - (1.0 + 0.001 * dw)).abs() < 1e-12);
    }
}
