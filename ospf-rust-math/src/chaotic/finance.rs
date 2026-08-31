//! 金融吸引子。
//! Finance attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// 金融吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Finance attractor.
    FinanceAttractor,
    /// 金融吸引子序列生成器。
    /// Finance attractor sequence generator.
    FinanceAttractorGenerator,
    [alpha, beta, zeta, h],
    |system, state| {
        let one = S::one();
        let dx = (one / system.beta - system.alpha) * state.x() + state.z() + state.x() * state.y();
        let dy = -system.beta * state.y() - state.x() * state.x();
        let dz = -state.x() - system.zeta * state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for FinanceAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.001, "0.001 must be representable"),
            default_float(0.2, "0.2 must be representable"),
            default_float(1.1, "1.1 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for FinanceAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(FinanceAttractor::default(), one_point3())
    }
}

/// 创建金融吸引子。
/// Create a Finance attractor.
pub fn finance_attractor<S: Field + Float>(alpha: S, beta: S, zeta: S, h: S) -> FinanceAttractor<S> {
    FinanceAttractor::new(alpha, beta, zeta, h)
}

/// 创建金融吸引子生成器。
/// Create a Finance attractor generator.
pub fn finance_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, zeta: S, h: S, x: Point3<S>,
) -> FinanceAttractorGenerator<S> {
    FinanceAttractorGenerator::new(FinanceAttractor::new(alpha, beta, zeta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-12, "actual={actual}, expected={expected}");
    }

    #[test]
    fn finance_step_formula() {
        let system = FinanceAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = (1.0 / 0.2 - 0.001) + 1.0 + 1.0;
        let dy = -0.2 - 1.0;
        let dz = -1.0 - 1.1;
        assert_close(next.x(), 1.0 + 0.01 * dx);
        assert_close(next.y(), 1.0 + 0.01 * dy);
        assert_close(next.z(), 1.0 + 0.01 * dz);
    }
}
