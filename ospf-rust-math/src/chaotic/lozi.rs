//! 洛兹映射。
//! Lozi map.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point2;
use super::helpers::{default_float, one_point2};

point2_system!(
    /// 洛兹映射。
    /// Lozi map.
    LoziMap,
    /// 洛兹映射序列生成器。
    /// Lozi map sequence generator.
    LoziMapGenerator,
    [a, b],
    |system, state| {
        let one = S::one();
        Point2::new(one + state.y() - system.a * state.x().abs(), system.b * state.x())
    }
);

impl<S: Field + Float> Default for LoziMap<S> {
    fn default() -> Self {
        Self::new(
            default_float(1.7, "1.7 must be representable"),
            default_float(0.5, "0.5 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for LoziMapGenerator<S> {
    fn default() -> Self {
        Self::new(LoziMap::default(), one_point2())
    }
}

/// 创建洛兹映射。
/// Create a Lozi map.
pub fn lozi_map<S: Field + Float>(a: S, b: S) -> LoziMap<S> {
    LoziMap::new(a, b)
}

/// 创建洛兹映射生成器。
/// Create a Lozi map generator.
pub fn lozi_map_generator<S: Field + Float>(a: S, b: S, x: Point2<S>) -> LoziMapGenerator<S> {
    LoziMapGenerator::new(LoziMap::new(a, b), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lozi_step_formula() {
        let system = LoziMap::<f64>::default();
        let next = system.step(Point2::new(1.0, 1.0));
        assert_eq!(next, Point2::new(1.0 + 1.0 - 1.7, 0.5));
    }
}
