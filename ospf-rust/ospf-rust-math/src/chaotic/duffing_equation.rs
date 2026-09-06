//! 达芬方程。
//! Duffing equation.

use super::helpers::default_float;
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// 达芬方程的一阶欧拉步进模型。
    /// First-order Euler step model for the Duffing equation.
    DuffingEquation,
    /// 达芬方程序列生成器。
    /// Duffing equation sequence generator.
    DuffingEquationGenerator,
    [alpha, beta, gamma, delta, omega, h],
    |system, state| {
        let dx = state.y();
        let dy = -system.alpha * state.x() - system.gamma * state.y()
            - system.beta * state.x() * state.x() * state.x()
            + system.delta * (system.omega * state.z()).cos();
        let dt = S::one();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dt,
        )
    }
);

impl<S: Field + Float> Default for DuffingEquation<S> {
    fn default() -> Self {
        Self::new(
            default_float(1.0, "1.0 must be representable"),
            default_float(5.0, "5.0 must be representable"),
            default_float(0.02, "0.02 must be representable"),
            default_float(8.0, "8.0 must be representable"),
            default_float(0.5, "0.5 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for DuffingEquationGenerator<S> {
    fn default() -> Self {
        Self::new(
            DuffingEquation::default(),
            Point3::new(S::zero(), S::zero(), S::zero()),
        )
    }
}

/// 创建达芬方程。
/// Create a Duffing equation.
pub fn duffing_equation<S: Field + Float>(
    alpha: S,
    beta: S,
    gamma: S,
    delta: S,
    omega: S,
    h: S,
) -> DuffingEquation<S> {
    DuffingEquation::new(alpha, beta, gamma, delta, omega, h)
}

/// 创建达芬方程序列生成器。
/// Create a Duffing equation sequence generator.
pub fn duffing_equation_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    gamma: S,
    delta: S,
    omega: S,
    h: S,
    x: Point3<S>,
) -> DuffingEquationGenerator<S> {
    DuffingEquationGenerator::new(DuffingEquation::new(alpha, beta, gamma, delta, omega, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duffing_equation_step_formula() {
        let system = DuffingEquation::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 0.0));
        let dx = 1.0;
        let dy = -1.0 - 0.02 - 5.0 + 8.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - 0.01).abs() < 1e-12);
    }
}
