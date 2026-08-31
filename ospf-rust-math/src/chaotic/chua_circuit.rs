//! Chua circuit system.
//! Chua 电路系统。

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Chua 电路的一阶欧拉步进模型。
    /// First-order Euler step model for the Chua circuit.
    ChuaCircuit,
    /// Chua 电路序列生成器。
    /// Chua circuit sequence generator.
    ChuaCircuitGenerator,
    [a, b, c, d, h],
    |system, state| {
        let half = S::one() / (S::one() + S::one());
        let f = system.c * state.x()
            + half
                * (system.d - system.c)
                * ((state.x() + S::one()).abs() - (state.x() - S::one()).abs());
        let dx = system.a * (state.y() - state.x() - f);
        let dy = state.x() - state.y() + state.z();
        let dz = -system.b * state.y();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for ChuaCircuit<S> {
    fn default() -> Self {
        Self::new(
            default_float(15.6, "15.6 must be representable"),
            default_float(28.0, "28.0 must be representable"),
            default_float(-0.71, "-0.71 must be representable"),
            default_float(-1.14, "-1.14 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for ChuaCircuitGenerator<S> {
    fn default() -> Self {
        Self::new(ChuaCircuit::default(), one_point3())
    }
}

/// 创建 Chua 电路。
/// Create a Chua circuit.
pub fn chua_circuit<S: Field + Float>(a: S, b: S, c: S, d: S, h: S) -> ChuaCircuit<S> {
    ChuaCircuit::new(a, b, c, d, h)
}

/// 创建 Chua 电路生成器。
/// Create a Chua circuit generator.
pub fn chua_circuit_generator<S: Field + Float>(
    a: S,
    b: S,
    c: S,
    d: S,
    h: S,
    x: Point3<S>,
) -> ChuaCircuitGenerator<S> {
    ChuaCircuitGenerator::new(ChuaCircuit::new(a, b, c, d, h), x)
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
    fn chua_circuit_matches_kotlin_formula() {
        assert_point3_close(
            ChuaCircuit::default().step(Point3::new(1.0, 1.0, 1.0)),
            Point3::new(1.17784, 1.01, 0.72),
        );
    }

    #[test]
    fn generator_returns_current_value_before_advancing() {
        let mut generator = ChuaCircuitGenerator::new(ChuaCircuit::default(), Point3::new(1.0, 1.0, 1.0));
        assert_eq!(generator.next_point(), Point3::new(1.0, 1.0, 1.0));
        assert_point3_close(
            generator.x().clone(),
            Point3::new(1.17784, 1.01, 0.72),
        );
    }
}
