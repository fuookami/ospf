//! Lotka-Volterra 系统（捕食者-猎物模型）。
//! Lotka-Volterra system (predator-prey model).

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point2;
use super::helpers::{default_float, one_point2};

point2_system!(
    /// Lotka-Volterra 系统的一阶欧拉步进模型。
    /// First-order Euler step model for the Lotka-Volterra system.
    LotkaVolterraSystem,
    /// Lotka-Volterra 系统序列生成器。
    /// Lotka-Volterra system sequence generator.
    LotkaVolterraSystemGenerator,
    [a, b, c, d, h],
    |system, state| {
        let dx = system.a * state.x() - system.b * state.x() * state.y();
        let dy = system.d * state.x() * state.y() - system.c * state.y();
        Point2::new(state.x() + system.h * dx, state.y() + system.h * dy)
    }
);

impl<S: Field + Float> Default for LotkaVolterraSystem<S> {
    fn default() -> Self {
        Self::new(
            default_float(1.0, "1.0 must be representable"),
            default_float(0.1, "0.1 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(0.1, "0.1 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for LotkaVolterraSystemGenerator<S> {
    fn default() -> Self {
        Self::new(LotkaVolterraSystem::default(), one_point2())
    }
}

/// 创建 Lotka-Volterra 系统。
/// Create a Lotka-Volterra system.
pub fn lotka_volterra_system<S: Field + Float>(a: S, b: S, c: S, d: S, h: S) -> LotkaVolterraSystem<S> {
    LotkaVolterraSystem::new(a, b, c, d, h)
}

/// 创建 Lotka-Volterra 系统生成器。
/// Create a Lotka-Volterra system generator.
pub fn lotka_volterra_system_generator<S: Field + Float>(
    a: S, b: S, c: S, d: S, h: S, x: Point2<S>,
) -> LotkaVolterraSystemGenerator<S> {
    LotkaVolterraSystemGenerator::new(LotkaVolterraSystem::new(a, b, c, d, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lotka_volterra_step_formula() {
        let system = LotkaVolterraSystem::<f64>::default();
        let next = system.step(Point2::new(1.0, 1.0));
        let dx = 1.0 - 0.1;
        let dy = 0.1 - 1.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
    }
}
