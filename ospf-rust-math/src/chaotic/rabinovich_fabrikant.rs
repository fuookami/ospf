//! Rabinovich-Fabrikant 方程。
//! Rabinovich-Fabrikant equation.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Rabinovich-Fabrikant 方程的一阶欧拉步进模型。
    /// First-order Euler step model for the Rabinovich-Fabrikant equation.
    RabinovichFabrikantEquation,
    /// Rabinovich-Fabrikant 方程序列生成器。
    /// Rabinovich-Fabrikant equation sequence generator.
    RabinovichFabrikantEquationGenerator,
    [a, b, h],
    |system, state| {
        let one = S::one();
        let three = one + one + one;
        let dx = state.y() * (state.z() - one + state.x() * state.x()) + system.b * state.x();
        let dy = state.x() * (three * state.z() + one - state.x() * state.x()) + system.b * state.y();
        let dz = -(one + one) * state.z() * (system.a + state.x() * state.y());
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for RabinovichFabrikantEquation<S> {
    fn default() -> Self {
        Self::new(
            default_float(1.1, "1.1 must be representable"),
            default_float(0.9, "0.9 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for RabinovichFabrikantEquationGenerator<S> {
    fn default() -> Self {
        Self::new(RabinovichFabrikantEquation::default(), one_point3())
    }
}

/// 创建 Rabinovich-Fabrikant 方程。
/// Create a Rabinovich-Fabrikant equation.
pub fn rabinovich_fabrikant_equation<S: Field + Float>(a: S, b: S, h: S) -> RabinovichFabrikantEquation<S> {
    RabinovichFabrikantEquation::new(a, b, h)
}

/// 创建 Rabinovich-Fabrikant 方程序列生成器。
/// Create a Rabinovich-Fabrikant equation sequence generator.
pub fn rabinovich_fabrikant_equation_generator<S: Field + Float>(
    a: S, b: S, h: S, x: Point3<S>,
) -> RabinovichFabrikantEquationGenerator<S> {
    RabinovichFabrikantEquationGenerator::new(RabinovichFabrikantEquation::new(a, b, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rabinovich_fabrikant_step_formula() {
        let system = RabinovichFabrikantEquation::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        // dx = 1 * (1 - 1 + 1) + 0.9 = 1.9
        // dy = 1 * (3 + 1 - 1) + 0.9 = 3.9
        // dz = -2 * (1.1 + 1) = -4.2
        assert!((next.x() - (1.0 + 0.01 * 1.9)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * 3.9)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * (-4.2))).abs() < 1e-12);
    }
}
