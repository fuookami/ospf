//! 受击转子。
//! Kicked rotator.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point2;
use super::helpers::{default_float, one_point2};

point2_system!(
    /// 受击转子。
    /// Kicked rotator.
    KickedRotator,
    /// 受击转子序列生成器。
    /// Kicked rotator sequence generator.
    KickedRotatorGenerator,
    [k],
    |system, state| {
        let new_x = state.x() + system.k * state.y().sin();
        Point2::new(new_x, state.y() + new_x)
    }
);

impl<S: Field + Float> Default for KickedRotator<S> {
    fn default() -> Self {
        Self::new(default_float(0.971635, "0.971635 must be representable"))
    }
}

impl<S: Field + Float> Default for KickedRotatorGenerator<S> {
    fn default() -> Self {
        Self::new(KickedRotator::default(), one_point2())
    }
}

/// 创建受击转子。
/// Create a kicked rotator.
pub fn kicked_rotator<S: Field + Float>(k: S) -> KickedRotator<S> {
    KickedRotator::new(k)
}

/// 创建受击转子生成器。
/// Create a kicked rotator generator.
pub fn kicked_rotator_generator<S: Field + Float>(k: S, x: Point2<S>) -> KickedRotatorGenerator<S> {
    KickedRotatorGenerator::new(KickedRotator::new(k), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kicked_rotator_step_formula() {
        let system = KickedRotator::<f64>::default();
        let next = system.step(Point2::new(1.0, 1.0));
        let new_x = 1.0 + 0.971635 * 1.0_f64.sin();
        let new_y = 1.0 + new_x;
        assert!((next.x() - new_x).abs() < 1e-12);
        assert!((next.y() - new_y).abs() < 1e-12);
    }
}
