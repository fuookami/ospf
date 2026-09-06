//! 电容方程。
//! Capacitance equation.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// 电容方程的一阶欧拉步进模型。
    /// First-order Euler step model for the capacitance equation.
    CapacitanceEquation,
    /// 电容方程序列生成器。
    /// Capacitance equation sequence generator.
    CapacitanceEquationGenerator,
    [a, b, c, d, e, h],
    |system, state| {
        let g = if state.x() > S::one() {
            system.e * state.x() - (system.e - system.d)
        } else if state.x() < -S::one() {
            system.e * state.y() + (system.e - system.d)
        } else {
            system.d * state.x()
        };
        let dx = system.a * ((system.c - S::one()) * g + state.y());
        let dy = g - state.y() + state.z();
        let dz = -system.b * state.y();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for CapacitanceEquation<S> {
    fn default() -> Self {
        let half = S::one() / (S::one() + S::one());
        Self::new(
            half,
            half,
            half,
            half,
            half,
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for CapacitanceEquationGenerator<S> {
    fn default() -> Self {
        Self::new(CapacitanceEquation::default(), one_point3())
    }
}

/// 创建电容方程。
/// Create a capacitance equation.
pub fn capacitance_equation<S: Field + Float>(
    a: S,
    b: S,
    c: S,
    d: S,
    e: S,
    h: S,
) -> CapacitanceEquation<S> {
    CapacitanceEquation::new(a, b, c, d, e, h)
}

/// 创建电容方程序列生成器。
/// Create a capacitance equation generator.
pub fn capacitance_equation_generator<S: Field + Float>(
    a: S,
    b: S,
    c: S,
    d: S,
    e: S,
    h: S,
    x: Point3<S>,
) -> CapacitanceEquationGenerator<S> {
    CapacitanceEquationGenerator::new(CapacitanceEquation::new(a, b, c, d, e, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "actual={actual}, expected={expected}"
        );
    }

    fn assert_point3_close(actual: Point3<f64>, expected: Point3<f64>) {
        assert_close(actual.x(), expected.x());
        assert_close(actual.y(), expected.y());
        assert_close(actual.z(), expected.z());
    }

    #[test]
    fn biological_and_physical_models_match_kotlin_formulas() {
        assert_point3_close(
            CapacitanceEquation::default().step(Point3::new(-2.0, 3.0, 4.0)),
            Point3::new(-1.98875, 3.025, 3.985),
        );
    }
}
