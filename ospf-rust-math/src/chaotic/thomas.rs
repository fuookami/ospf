//! Thomas 吸引子。
//! Thomas attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Thomas 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Thomas attractor.
    ThomasAttractor,
    /// Thomas 吸引子序列生成器。
    /// Thomas attractor sequence generator.
    ThomasAttractorGenerator,
    [beta, h],
    |system, state| {
        let dx = state.x().sin() - system.beta * state.x();
        let dy = state.y().sin() - system.beta * state.y();
        let dz = state.z().sin() - system.beta * state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for ThomasAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.19, "0.19 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for ThomasAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(ThomasAttractor::default(), one_point3())
    }
}

/// 创建 Thomas 吸引子。
/// Create a Thomas attractor.
pub fn thomas_attractor<S: Field + Float>(beta: S, h: S) -> ThomasAttractor<S> {
    ThomasAttractor::new(beta, h)
}

/// 创建 Thomas 吸引子生成器。
/// Create a Thomas attractor generator.
pub fn thomas_attractor_generator<S: Field + Float>(beta: S, h: S, x: Point3<S>) -> ThomasAttractorGenerator<S> {
    ThomasAttractorGenerator::new(ThomasAttractor::new(beta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thomas_step_formula() {
        let system = ThomasAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let sin1 = 1.0_f64.sin();
        let dx = sin1 - 0.19;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
    }
}
