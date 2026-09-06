//! Halvorsen 吸引子。
//! Halvorsen attractor.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Halvorsen 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Halvorsen attractor.
    HalvorsenAttractor,
    /// Halvorsen 吸引子序列生成器。
    /// Halvorsen attractor sequence generator.
    HalvorsenAttractorGenerator,
    [alpha, h],
    |system, state| {
        let four = S::one() + S::one() + S::one() + S::one();
        let dx = -system.alpha * state.x() - four * state.y() - four * state.z() - state.y() * state.y();
        let dy = -system.alpha * state.y() - four * state.z() - four * state.x() - state.z() * state.z();
        let dz = -system.alpha * state.z() - four * state.x() - four * state.y() - state.x() * state.x();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for HalvorsenAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(1.4, "1.4 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for HalvorsenAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(HalvorsenAttractor::default(), one_point3())
    }
}

/// 创建 Halvorsen 吸引子。
/// Create a Halvorsen attractor.
pub fn halvorsen_attractor<S: Field + Float>(alpha: S, h: S) -> HalvorsenAttractor<S> {
    HalvorsenAttractor::new(alpha, h)
}

/// 创建 Halvorsen 吸引子生成器。
/// Create a Halvorsen attractor generator.
pub fn halvorsen_attractor_generator<S: Field + Float>(
    alpha: S,
    h: S,
    x: Point3<S>,
) -> HalvorsenAttractorGenerator<S> {
    HalvorsenAttractorGenerator::new(HalvorsenAttractor::new(alpha, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halvorsen_step_formula() {
        let system = HalvorsenAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = -1.4 - 4.0 - 4.0 - 1.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
    }
}
