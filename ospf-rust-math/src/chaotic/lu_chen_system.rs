//! Lu-Chen 系统。
//! Lu-Chen system.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Lu-Chen 系统的一阶欧拉步进模型。
    /// First-order Euler step model for the Lu-Chen system.
    LuChenSystem,
    /// Lu-Chen 系统序列生成器。
    /// Lu-Chen system sequence generator.
    LuChenSystemGenerator,
    [a, b, c, d, h],
    |system, state| {
        let dx = system.a * (state.y() - state.x());
        let dy = state.x() - state.x() * state.z() + system.c * state.y() + system.d;
        let dz = state.x() * state.y() - system.b * state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for LuChenSystem<S> {
    fn default() -> Self {
        Self::new(
            default_float(36.0, "36.0 must be representable"),
            default_float(20.0, "20.0 must be representable"),
            default_float(3.0, "3.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for LuChenSystemGenerator<S> {
    fn default() -> Self {
        Self::new(LuChenSystem::default(), one_point3())
    }
}

/// 创建 Lu-Chen 系统。
/// Create a Lu-Chen system.
pub fn lu_chen_system<S: Field + Float>(a: S, b: S, c: S, d: S, h: S) -> LuChenSystem<S> {
    LuChenSystem::new(a, b, c, d, h)
}

/// 创建 Lu-Chen 系统生成器。
/// Create a Lu-Chen system generator.
pub fn lu_chen_system_generator<S: Field + Float>(
    a: S, b: S, c: S, d: S, h: S, x: Point3<S>,
) -> LuChenSystemGenerator<S> {
    LuChenSystemGenerator::new(LuChenSystem::new(a, b, c, d, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lu_chen_system_step_formula() {
        let system = LuChenSystem::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = 36.0 * (1.0 - 1.0);
        let dy = 1.0 - 1.0 + 3.0 + 1.0;
        let dz = 1.0 - 20.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
