//! Lorenz-Stenflo 吸引子（四维）。
//! Lorenz-Stenflo attractor (4D).

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point4;
use super::helpers::{default_float, one_point4};

point4_system!(
    /// Lorenz-Stenflo 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Lorenz-Stenflo attractor.
    LorenzStenfloAttractor,
    /// Lorenz-Stenflo 吸引子序列生成器。
    /// Lorenz-Stenflo attractor sequence generator.
    LorenzStenfloAttractorGenerator,
    [alpha, beta, delta, zeta, h],
    |system, state| {
        let dx = system.alpha * (state.y() - state.x()) + system.delta * state.w();
        let dy = state.x() * (system.zeta - state.z()) - state.y();
        let dz = state.x() * state.y() - system.beta * state.z();
        let dw = -state.x() - system.alpha * state.w();
        Point4::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
            state.w() + system.h * dw,
        )
    }
);

impl<S: Field + Float> Default for LorenzStenfloAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(2.0, "2.0 must be representable"),
            default_float(0.7, "0.7 must be representable"),
            default_float(1.5, "1.5 must be representable"),
            default_float(26.0, "26.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for LorenzStenfloAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(LorenzStenfloAttractor::default(), one_point4())
    }
}

/// 创建 Lorenz-Stenflo 吸引子。
/// Create a Lorenz-Stenflo attractor.
pub fn lorenz_stenflo_attractor<S: Field + Float>(
    alpha: S, beta: S, delta: S, zeta: S, h: S,
) -> LorenzStenfloAttractor<S> {
    LorenzStenfloAttractor::new(alpha, beta, delta, zeta, h)
}

/// 创建 Lorenz-Stenflo 吸引子生成器。
/// Create a Lorenz-Stenflo attractor generator.
pub fn lorenz_stenflo_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, delta: S, zeta: S, h: S, x: Point4<S>,
) -> LorenzStenfloAttractorGenerator<S> {
    LorenzStenfloAttractorGenerator::new(LorenzStenfloAttractor::new(alpha, beta, delta, zeta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lorenz_stenflo_step_formula() {
        let system = LorenzStenfloAttractor::<f64>::default();
        let next = system.step(Point4::new(1.0, 1.0, 1.0, 1.0));
        let dx = 2.0 * (1.0 - 1.0) + 1.5 * 1.0;
        let dy = 1.0 * (26.0 - 1.0) - 1.0;
        let dz = 1.0 - 0.7;
        let dw = -1.0 - 2.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
        assert!((next.w() - (1.0 + 0.01 * dw)).abs() < 1e-12);
    }
}
