//! Qi-Chen 吸引子。
//! Qi-Chen attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Qi-Chen 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Qi-Chen attractor.
    QiChenAttractor,
    /// Qi-Chen 吸引子序列生成器。
    /// Qi-Chen attractor sequence generator.
    QiChenAttractorGenerator,
    [alpha, beta, zeta, h],
    |system, state| {
        let dx = system.alpha * (state.y() - state.x()) + state.y() * state.z();
        let dy = system.zeta * state.x() + state.y() - state.x() * state.z();
        let dz = state.x() * state.y() - system.beta * state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for QiChenAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(38.0, "38.0 must be representable"),
            default_float(8.0 / 3.0, "8/3 must be representable"),
            default_float(80.0, "80.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for QiChenAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(QiChenAttractor::default(), one_point3())
    }
}

/// 创建 Qi-Chen 吸引子。
/// Create a Qi-Chen attractor.
pub fn qi_chen_attractor<S: Field + Float>(alpha: S, beta: S, zeta: S, h: S) -> QiChenAttractor<S> {
    QiChenAttractor::new(alpha, beta, zeta, h)
}

/// 创建 Qi-Chen 吸引子生成器。
/// Create a Qi-Chen attractor generator.
pub fn qi_chen_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, zeta: S, h: S, x: Point3<S>,
) -> QiChenAttractorGenerator<S> {
    QiChenAttractorGenerator::new(QiChenAttractor::new(alpha, beta, zeta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qi_chen_step_formula() {
        let system = QiChenAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = 38.0 * (1.0 - 1.0) + 1.0;
        let dy = 80.0 + 1.0 - 1.0;
        let dz = 1.0 - 8.0 / 3.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
