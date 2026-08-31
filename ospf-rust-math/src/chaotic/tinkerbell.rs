//! 丁克贝尔映射。
//! Tinkerbell map.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point2;
use super::helpers::{default_float, one_point2};

point2_system!(
    /// 丁克贝尔映射。
    /// Tinkerbell map.
    TinkerbellMap,
    /// 丁克贝尔映射序列生成器。
    /// Tinkerbell map sequence generator.
    TinkerbellMapGenerator,
    [a, b, c, d],
    |system, state| {
        let two = S::one() + S::one();
        Point2::new(
            state.x() * state.x() - state.y() * state.y() + system.a * state.x() + system.b * state.y(),
            two * state.x() * state.y() + system.c * state.x() + system.d * state.y(),
        )
    }
);

impl<S: Field + Float> Default for TinkerbellMap<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.9, "0.9 must be representable"),
            default_float(-0.6013, "-0.6013 must be representable"),
            default_float(2.0, "2.0 must be representable"),
            default_float(0.5, "0.5 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for TinkerbellMapGenerator<S> {
    fn default() -> Self {
        Self::new(TinkerbellMap::default(), one_point2())
    }
}

/// 创建丁克贝尔映射。
/// Create a Tinkerbell map.
pub fn tinkerbell_map<S: Field + Float>(a: S, b: S, c: S, d: S) -> TinkerbellMap<S> {
    TinkerbellMap::new(a, b, c, d)
}

/// 创建丁克贝尔映射生成器。
/// Create a Tinkerbell map generator.
pub fn tinkerbell_map_generator<S: Field + Float>(a: S, b: S, c: S, d: S, x: Point2<S>) -> TinkerbellMapGenerator<S> {
    TinkerbellMapGenerator::new(TinkerbellMap::new(a, b, c, d), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tinkerbell_step_formula() {
        let system = TinkerbellMap::<f64>::default();
        let next = system.step(Point2::new(1.0, 1.0));
        let expected_x = 1.0 - 1.0 + 0.9 - 0.6013;
        let expected_y = 2.0 + 2.0 + 0.5;
        assert!((next.x() - expected_x).abs() < 1e-12);
        assert!((next.y() - expected_y).abs() < 1e-12);
    }
}
