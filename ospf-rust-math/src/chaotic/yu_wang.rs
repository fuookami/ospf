//! Yu-Wang 吸引子。
//! Yu-Wang attractor.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Yu-Wang 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Yu-Wang attractor.
    YuWangAttractor,
    /// Yu-Wang 吸引子序列生成器。
    /// Yu-Wang attractor sequence generator.
    YuWangAttractorGenerator,
    [alpha, beta, delta, zeta, h],
    |system, state| {
        let dx = system.alpha * (state.y() - state.x());
        let dy = system.beta * state.x() - system.zeta * state.x() * state.z();
        let dz = (state.x() * state.y()).exp() - system.delta * state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for YuWangAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(10.0, "10.0 must be representable"),
            default_float(40.0, "40.0 must be representable"),
            default_float(2.5, "2.5 must be representable"),
            default_float(2.0, "2.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for YuWangAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(YuWangAttractor::default(), one_point3())
    }
}

/// 创建 Yu-Wang 吸引子。
/// Create a Yu-Wang attractor.
pub fn yu_wang_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    zeta: S,
    h: S,
) -> YuWangAttractor<S> {
    YuWangAttractor::new(alpha, beta, delta, zeta, h)
}

/// 创建 Yu-Wang 吸引子生成器。
/// Create a Yu-Wang attractor generator.
pub fn yu_wang_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    zeta: S,
    h: S,
    x: Point3<S>,
) -> YuWangAttractorGenerator<S> {
    YuWangAttractorGenerator::new(YuWangAttractor::new(alpha, beta, delta, zeta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yu_wang_step_formula() {
        let system = YuWangAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = 10.0 * (1.0 - 1.0);
        let dy = 40.0 - 2.0;
        let dz = 1.0_f64.exp() - 2.5;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
