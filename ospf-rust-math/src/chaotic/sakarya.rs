//! Sakarya 吸引子。
//! Sakarya attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Sakarya 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Sakarya attractor.
    SakaryaAttractor,
    /// Sakarya 吸引子序列生成器。
    /// Sakarya attractor sequence generator.
    SakaryaAttractorGenerator,
    [alpha, beta, h],
    |system, state| {
        let dx = -state.x() + state.y() + state.y() * state.z();
        let dy = -state.x() - state.y() + system.alpha * state.x() * state.z();
        let dz = state.z() - system.beta * state.x() * state.y();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for SakaryaAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.4, "0.4 must be representable"),
            default_float(0.3, "0.3 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for SakaryaAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(SakaryaAttractor::default(), one_point3())
    }
}

/// 创建 Sakarya 吸引子。
/// Create a Sakarya attractor.
pub fn sakarya_attractor<S: Field + Float>(alpha: S, beta: S, h: S) -> SakaryaAttractor<S> {
    SakaryaAttractor::new(alpha, beta, h)
}

/// 创建 Sakarya 吸引子生成器。
/// Create a Sakarya attractor generator.
pub fn sakarya_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, h: S, x: Point3<S>,
) -> SakaryaAttractorGenerator<S> {
    SakaryaAttractorGenerator::new(SakaryaAttractor::new(alpha, beta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sakarya_step_formula() {
        let system = SakaryaAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = -1.0 + 1.0 + 1.0;
        let dy = -1.0 - 1.0 + 0.4;
        let dz = 1.0 - 0.3;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
