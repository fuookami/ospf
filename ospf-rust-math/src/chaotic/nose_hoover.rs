//! Nose-Hoover 吸引子。
//! Nose-Hoover attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Nose-Hoover 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Nose-Hoover attractor.
    NoseHooverAttractor,
    /// Nose-Hoover 吸引子序列生成器。
    /// Nose-Hoover attractor sequence generator.
    NoseHooverAttractorGenerator,
    [alpha, h],
    |system, state| {
        let dx = state.y();
        let dy = -state.x() + state.y() * state.z();
        let dz = system.alpha - state.y() * state.y();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for NoseHooverAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(1.5, "1.5 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for NoseHooverAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(NoseHooverAttractor::default(), one_point3())
    }
}

/// 创建 Nose-Hoover 吸引子。
/// Create a Nose-Hoover attractor.
pub fn nose_hoover_attractor<S: Field + Float>(alpha: S, h: S) -> NoseHooverAttractor<S> {
    NoseHooverAttractor::new(alpha, h)
}

/// 创建 Nose-Hoover 吸引子生成器。
/// Create a Nose-Hoover attractor generator.
pub fn nose_hoover_attractor_generator<S: Field + Float>(
    alpha: S, h: S, x: Point3<S>,
) -> NoseHooverAttractorGenerator<S> {
    NoseHooverAttractorGenerator::new(NoseHooverAttractor::new(alpha, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nose_hoover_step_formula() {
        let system = NoseHooverAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        // dx = 1, dy = -1 + 1 = 0, dz = 1.5 - 1 = 0.5
        assert!((next.x() - 1.01).abs() < 1e-12);
        assert!((next.y() - 1.0).abs() < 1e-12);
        assert!((next.z() - 1.005).abs() < 1e-12);
    }
}
