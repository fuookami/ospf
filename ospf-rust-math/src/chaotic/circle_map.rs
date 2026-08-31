//! 圆映射。
//! Circle map.

use super::helpers::{default_float, mod_one};
use crate::algebra::Field;
use num_traits::Float;

scalar_map!(
    /// 圆映射的一阶欧拉步进模型。
    /// First-order Euler step model for the circle map.
    CircleMap,
    /// 圆映射序列生成器。
    /// Circle map sequence generator.
    CircleMapGenerator,
    [alpha, beta],
    |system, state| {
        let pi2 = default_float::<S>(std::f64::consts::PI * 2.0, "2 pi must be representable");
        mod_one(state + system.alpha - system.beta * (state * pi2).sin() / pi2)
    }
);

impl<S: Field + Float> Default for CircleMap<S> {
    fn default() -> Self {
        Self::new(
            S::one() / (S::one() + S::one()),
            default_float(std::f64::consts::PI, "pi must be representable"),
        )
    }
}

impl<S: Field + Float> Default for CircleMapGenerator<S> {
    fn default() -> Self {
        Self::new(CircleMap::default(), S::one() / (S::one() + S::one()))
    }
}

/// 创建圆映射。
/// Create a circle map.
pub fn circle_map<S: Field + Float>(alpha: S, beta: S) -> CircleMap<S> {
    CircleMap::new(alpha, beta)
}

/// 创建圆映射生成器。
/// Create a circle map generator.
pub fn circle_map_generator<S: Field + Float>(alpha: S, beta: S, x: S) -> CircleMapGenerator<S> {
    CircleMapGenerator::new(CircleMap::new(alpha, beta), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn scalar_maps_match_kotlin_formulas() {
        assert_close(CircleMap::new(0.25_f64, 0.0).step(0.5), 0.75);
    }
}
