//! 达芬映射。
//! Duffing map.

use super::helpers::{default_float, one_point2};
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;

point2_system!(
    /// 达芬映射。
    /// Duffing map.
    DuffingMap,
    /// 达芬映射序列生成器。
    /// Duffing map sequence generator.
    DuffingMapGenerator,
    [a, b],
    |system, state| {
        Point2::new(state.y(), -system.b * state.x() + system.a * state.y() - state.y() * state.y() * state.y())
    }
);

impl<S: Field + Float> Default for DuffingMap<S> {
    fn default() -> Self {
        Self::new(
            default_float(2.75, "2.75 must be representable"),
            default_float(0.2, "0.2 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for DuffingMapGenerator<S> {
    fn default() -> Self {
        Self::new(DuffingMap::default(), one_point2())
    }
}

/// 创建达芬映射。
/// Create a Duffing map.
pub fn duffing_map<S: Field + Float>(a: S, b: S) -> DuffingMap<S> {
    DuffingMap::new(a, b)
}

/// 创建达芬映射生成器。
/// Create a Duffing map generator.
pub fn duffing_map_generator<S: Field + Float>(a: S, b: S, x: Point2<S>) -> DuffingMapGenerator<S> {
    DuffingMapGenerator::new(DuffingMap::new(a, b), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duffing_step_formula() {
        let system = DuffingMap::<f64>::default();
        let next = system.step(Point2::new(1.0, 1.0));
        // x_{n+1} = y = 1.0
        // y_{n+1} = -0.2 * 1.0 + 2.75 * 1.0 - 1.0 = 1.55
        assert_eq!(next.x(), 1.0);
        assert!((next.y() - 1.55).abs() < 1e-12);
    }
}
