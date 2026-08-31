//! Lorenz 84 模型。
//! Lorenz 84 model.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Lorenz 84 模型的一阶欧拉步进模型。
    /// First-order Euler step model for the Lorenz 84 model.
    Lorenz84Model,
    /// Lorenz 84 模型序列生成器。
    /// Lorenz 84 model sequence generator.
    Lorenz84ModelGenerator,
    [a, b, f, g, h],
    |system, state| {
        let dx = -state.y() * state.y() - state.z() * state.z() - system.a * state.x() + system.a * system.f;
        let dy = state.x() * state.y() - system.b * state.x() * state.z() - state.y() + system.g;
        let dz = system.b * state.x() * state.y() + state.x() * state.z() - state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for Lorenz84Model<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.25, "0.25 must be representable"),
            default_float(4.0, "4.0 must be representable"),
            default_float(8.0, "8.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for Lorenz84ModelGenerator<S> {
    fn default() -> Self {
        Self::new(Lorenz84Model::default(), one_point3())
    }
}

/// 创建 Lorenz 84 模型。
/// Create a Lorenz 84 model.
pub fn lorenz84_model<S: Field + Float>(a: S, b: S, f: S, g: S, h: S) -> Lorenz84Model<S> {
    Lorenz84Model::new(a, b, f, g, h)
}

/// 创建 Lorenz 84 模型生成器。
/// Create a Lorenz 84 model generator.
pub fn lorenz84_model_generator<S: Field + Float>(
    a: S, b: S, f: S, g: S, h: S, x: Point3<S>,
) -> Lorenz84ModelGenerator<S> {
    Lorenz84ModelGenerator::new(Lorenz84Model::new(a, b, f, g, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lorenz84_step_formula() {
        let system = Lorenz84Model::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = -1.0 - 1.0 - 0.25 + 2.0;
        let dy = 1.0 - 4.0 - 1.0 + 1.0;
        let dz = 4.0 + 1.0 - 1.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
