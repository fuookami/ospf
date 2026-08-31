//! 范德波尔系统。
//! Van der Pol system.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point2;
use super::helpers::{default_float, one_point2};

point2_system!(
    /// 范德波尔系统的一阶欧拉步进模型。
    /// First-order Euler step model for the Van der Pol system.
    VanDerPolSystem,
    /// 范德波尔系统序列生成器。
    /// Van der Pol system sequence generator.
    VanDerPolSystemGenerator,
    [a, h],
    |system, state| {
        let three = S::one() + S::one() + S::one();
        let dx = system.a * (state.x() - state.x() * state.x() * state.x() / three) - state.y();
        let dy = state.x() / system.a;
        Point2::new(state.x() + system.h * dx, state.y() + system.h * dy)
    }
);

impl<S: Field + Float> Default for VanDerPolSystem<S> {
    fn default() -> Self {
        Self::new(
            default_float(1.0, "1.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for VanDerPolSystemGenerator<S> {
    fn default() -> Self {
        Self::new(VanDerPolSystem::default(), one_point2())
    }
}

/// 创建范德波尔系统。
/// Create a Van der Pol system.
pub fn van_der_pol_system<S: Field + Float>(a: S, h: S) -> VanDerPolSystem<S> {
    VanDerPolSystem::new(a, h)
}

/// 创建范德波尔系统生成器。
/// Create a Van der Pol system generator.
pub fn van_der_pol_system_generator<S: Field + Float>(a: S, h: S, x: Point2<S>) -> VanDerPolSystemGenerator<S> {
    VanDerPolSystemGenerator::new(VanDerPolSystem::new(a, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn van_der_pol_step_formula() {
        let system = VanDerPolSystem::<f64>::default();
        let next = system.step(Point2::new(1.0, 1.0));
        let dx = 1.0 * (1.0 - 1.0 / 3.0) - 1.0;
        let dy = 1.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
    }
}
