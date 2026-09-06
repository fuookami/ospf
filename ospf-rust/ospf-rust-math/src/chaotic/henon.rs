//! 埃农映射。
//! Henon map.

use super::helpers::{default_float, one_point2};
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;

point2_system!(
    /// 埃农映射。
    /// Henon map.
    HenonMap,
    /// 埃农映射序列生成器。
    /// Henon map sequence generator.
    HenonMapGenerator,
    [a, b],
    |system, state| {
        let one = S::one();
        Point2::new(one - system.a * state.x() * state.x() + state.y(), system.b * state.x())
    }
);

impl<S: Field + Float> Default for HenonMap<S> {
    fn default() -> Self {
        Self::new(
            default_float(1.4, "1.4 must be representable"),
            default_float(0.3, "0.3 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for HenonMapGenerator<S> {
    fn default() -> Self {
        Self::new(HenonMap::default(), one_point2())
    }
}

/// 创建埃农映射。
/// Create a Henon map.
pub fn henon_map<S: Field + Float>(a: S, b: S) -> HenonMap<S> {
    HenonMap::new(a, b)
}

/// 创建埃农映射生成器。
/// Create a Henon map generator.
pub fn henon_map_generator<S: Field + Float>(a: S, b: S, x: Point2<S>) -> HenonMapGenerator<S> {
    HenonMapGenerator::new(HenonMap::new(a, b), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn henon_step_formula() {
        let system = HenonMap::<f64>::default();
        let next = system.step(Point2::new(1.0, 1.0));
        // x_{n+1} = 1 - 1.4 * 1 + 1 = 0.6
        // y_{n+1} = 0.3 * 1 = 0.3
        assert!((next.x() - 0.6).abs() < 1e-12);
        assert!((next.y() - 0.3).abs() < 1e-12);
    }
}
