//! Hindmarsh-Rose 神经元模型。
//! Hindmarsh-Rose neuron model.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Hindmarsh-Rose 神经元模型的一阶欧拉步进模型。
    /// First-order Euler step model for the Hindmarsh-Rose neuron model.
    HindmarshRoseModel,
    /// Hindmarsh-Rose 神经元模型序列生成器。
    /// Hindmarsh-Rose neuron model sequence generator.
    HindmarshRoseModelGenerator,
    [a, b, c, d, s, r, xr, i, h],
    |system, state| {
        let phi = -system.a * state.x() * state.x() * state.x() + system.b * state.x() * state.x();
        let psi = system.c - system.d * state.x() * state.x();
        let dx = state.y() + phi - state.z() + system.i;
        let dy = psi - state.y();
        let dz = system.r * (system.s * (state.x() - system.xr) - state.z());
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for HindmarshRoseModel<S> {
    fn default() -> Self {
        Self::new(
            default_float(1.0, "1.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for HindmarshRoseModelGenerator<S> {
    fn default() -> Self {
        Self::new(HindmarshRoseModel::default(), one_point3())
    }
}

/// 创建 Hindmarsh-Rose 神经元模型。
/// Create a Hindmarsh-Rose neuron model.
pub fn hindmarsh_rose_model<S: Field + Float>(
    a: S,
    b: S,
    c: S,
    d: S,
    s: S,
    r: S,
    xr: S,
    i: S,
    h: S,
) -> HindmarshRoseModel<S> {
    HindmarshRoseModel::new(a, b, c, d, s, r, xr, i, h)
}

/// 创建 Hindmarsh-Rose 神经元模型生成器。
/// Create a Hindmarsh-Rose neuron model generator.
pub fn hindmarsh_rose_model_generator<S: Field + Float>(
    a: S,
    b: S,
    c: S,
    d: S,
    s: S,
    r: S,
    xr: S,
    i: S,
    h: S,
    x: Point3<S>,
) -> HindmarshRoseModelGenerator<S> {
    HindmarshRoseModelGenerator::new(HindmarshRoseModel::new(a, b, c, d, s, r, xr, i, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hindmarsh_rose_step_formula() {
        let system = HindmarshRoseModel::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        // phi = -1 + 1 = 0, psi = 1 - 1 = 0
        // dx = 1 + 0 - 1 + 1 = 1, dy = 0 - 1 = -1, dz = 1 * (1 * 0 - 1) = -1
        assert!((next.x() - 1.01).abs() < 1e-12);
        assert!((next.y() - 0.99).abs() < 1e-12);
        assert!((next.z() - 0.99).abs() < 1e-12);
    }
}
