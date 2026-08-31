//! 辛映射。
//! Symplectic map.

use super::helpers::{default_float, one_point2};
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;

point2_system!(
    /// 辛映射。
    /// Symplectic map.
    SymplecticMap,
    /// 辛映射序列生成器。
    /// Symplectic map sequence generator.
    SymplecticMapGenerator,
    [h],
    |system, state| {
        let one = S::one();
        let one_plus_x = one + state.x();
        let dx = state.x() / one_plus_x;
        let dy = state.y() * one_plus_x * one_plus_x;
        Point2::new(state.x() + system.h * dx, state.y() + system.h * dy)
    }
);

impl<S: Field + Float> Default for SymplecticMap<S> {
    fn default() -> Self {
        Self::new(default_float(0.01, "0.01 must be representable"))
    }
}

impl<S: Field + Float> Default for SymplecticMapGenerator<S> {
    fn default() -> Self {
        Self::new(SymplecticMap::default(), one_point2())
    }
}

/// 创建辛映射。
/// Create a symplectic map.
pub fn symplectic_map<S: Field + Float>(h: S) -> SymplecticMap<S> {
    SymplecticMap::new(h)
}

/// 创建辛映射生成器。
/// Create a symplectic map generator.
pub fn symplectic_map_generator<S: Field + Float>(h: S, x: Point2<S>) -> SymplecticMapGenerator<S> {
    SymplecticMapGenerator::new(SymplecticMap::new(h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symplectic_step_formula() {
        let system = SymplecticMap::<f64>::default();
        let next = system.step(Point2::new(1.0, 1.0));
        let dx = 1.0 / 2.0;
        let dy = 1.0 * 4.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
    }
}
