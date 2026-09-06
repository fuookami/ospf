//! Kaplan-Yorke 映射。
//! Kaplan-Yorke map.

use super::helpers::{default_float, one_point2};
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;

point2_system!(
    /// Kaplan-Yorke 映射。
    /// Kaplan-Yorke map.
    KaplanYorkeMap,
    /// Kaplan-Yorke 映射序列生成器。
    /// Kaplan-Yorke map sequence generator.
    KaplanYorkeMapGenerator,
    [a, four_pi],
    |system, state| {
        let one = S::one();
        let two = one + one;
        let new_x = (two * state.x()) - (two * state.x()).floor();
        let new_y = system.a * state.y() + (system.four_pi * state.x()).cos();
        Point2::new(new_x, new_y)
    }
);

impl<S: Field + Float> Default for KaplanYorkeMap<S> {
    fn default() -> Self {
        let four_pi = S::from(4.0).expect("4.0 must be representable")
            * S::from(std::f64::consts::PI).expect("PI must be representable");
        Self::new(default_float(0.2, "0.2 must be representable"), four_pi)
    }
}

impl<S: Field + Float> Default for KaplanYorkeMapGenerator<S> {
    fn default() -> Self {
        Self::new(KaplanYorkeMap::default(), one_point2())
    }
}

/// 创建 Kaplan-Yorke 映射。
/// Create a Kaplan-Yorke map.
pub fn kaplan_yorke_map<S: Field + Float>(a: S, four_pi: S) -> KaplanYorkeMap<S> {
    KaplanYorkeMap::new(a, four_pi)
}

/// 创建 Kaplan-Yorke 映射生成器。
/// Create a Kaplan-Yorke map generator.
pub fn kaplan_yorke_map_generator<S: Field + Float>(
    a: S,
    four_pi: S,
    x: Point2<S>,
) -> KaplanYorkeMapGenerator<S> {
    KaplanYorkeMapGenerator::new(KaplanYorkeMap::new(a, four_pi), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kaplan_yorke_step_formula() {
        let system = KaplanYorkeMap::<f64>::default();
        let next = system.step(Point2::new(0.25, 0.5));
        // new_x = (2 * 0.25) mod 1 = 0.5
        // new_y = 0.2 * 0.5 + cos(4*pi*0.25) = 0.1 + cos(pi) = 0.1 - 1.0
        assert!((next.x() - 0.5).abs() < 1e-12);
        assert!((next.y() - (0.1 - 1.0)).abs() < 1e-12);
    }
}
