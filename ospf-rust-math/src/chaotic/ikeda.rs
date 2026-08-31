//! 池田映射。
//! Ikeda map.

use super::helpers::{default_float, one_point2};
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;

point2_system!(
    /// 池田映射。
    /// Ikeda map.
    IkedaMap,
    /// 池田映射序列生成器。
    /// Ikeda map sequence generator.
    IkedaMapGenerator,
    [u, t0, t1],
    |system, state| {
        let one = S::one();
        let t = system.t0 - system.t1 / (one + state.x() * state.x() + state.y() * state.y());
        let cost = t.cos();
        let sint = t.sin();
        Point2::new(
            one + system.u * (state.x() * cost - state.y() * sint),
            system.u * (state.x() * sint + state.y() * cost),
        )
    }
);

impl<S: Field + Float> Default for IkedaMap<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.918, "0.918 must be representable"),
            default_float(0.4, "0.4 must be representable"),
            default_float(6.0, "6.0 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for IkedaMapGenerator<S> {
    fn default() -> Self {
        Self::new(IkedaMap::default(), one_point2())
    }
}

/// 创建池田映射。
/// Create an Ikeda map.
pub fn ikeda_map<S: Field + Float>(u: S, t0: S, t1: S) -> IkedaMap<S> {
    IkedaMap::new(u, t0, t1)
}

/// 创建池田映射生成器。
/// Create an Ikeda map generator.
pub fn ikeda_map_generator<S: Field + Float>(
    u: S,
    t0: S,
    t1: S,
    x: Point2<S>,
) -> IkedaMapGenerator<S> {
    IkedaMapGenerator::new(IkedaMap::new(u, t0, t1), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ikeda_step_formula() {
        let system = IkedaMap::<f64>::default();
        let next = system.step(Point2::new(1.0, 1.0));
        let t = 0.4 - 6.0 / (1.0 + 1.0 + 1.0);
        let cost = t.cos();
        let sint = t.sin();
        let expected_x = 1.0 + 0.918 * (cost - sint);
        let expected_y = 0.918 * (sint + cost);
        assert!((next.x() - expected_x).abs() < 1e-12);
        assert!((next.y() - expected_y).abs() < 1e-12);
    }
}
