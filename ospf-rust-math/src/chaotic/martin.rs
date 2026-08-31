//! Martin 迭代。
//! Martin iterate.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point2;
use super::helpers::{default_float, one_point2};

point2_system!(
    /// Martin 迭代。
    /// Martin iterate.
    MartinIterate,
    /// Martin 迭代序列生成器。
    /// Martin iterate sequence generator.
    MartinIterateGenerator,
    [a, b, c],
    |system, state| {
        let temp = (system.b * state.x() - system.c).abs().sqrt();
        let g = if state.x() > S::zero() {
            state.y() - temp
        } else if state.x() < -S::zero() {
            state.y() + temp
        } else {
            state.y()
        };
        Point2::new(g, system.a - state.x())
    }
);

impl<S: Field + Float> Default for MartinIterate<S> {
    fn default() -> Self {
        Self::new(
            default_float(68.0, "68.0 must be representable"),
            default_float(75.0, "75.0 must be representable"),
            default_float(83.0, "83.0 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for MartinIterateGenerator<S> {
    fn default() -> Self {
        Self::new(MartinIterate::default(), one_point2())
    }
}

/// 创建 Martin 迭代。
/// Create a Martin iterate.
pub fn martin_iterate<S: Field + Float>(a: S, b: S, c: S) -> MartinIterate<S> {
    MartinIterate::new(a, b, c)
}

/// 创建 Martin 迭代生成器。
/// Create a Martin iterate generator.
pub fn martin_iterate_generator<S: Field + Float>(a: S, b: S, c: S, x: Point2<S>) -> MartinIterateGenerator<S> {
    MartinIterateGenerator::new(MartinIterate::new(a, b, c), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn martin_step_formula() {
        let system = MartinIterate::<f64>::default();
        let next = system.step(Point2::new(1.0, 1.0));
        let temp = (75.0 - 83.0_f64).abs().sqrt();
        let g = 1.0 - temp;
        assert!((next.x() - g).abs() < 1e-12);
        assert!((next.y() - 67.0).abs() < 1e-12);
    }
}
