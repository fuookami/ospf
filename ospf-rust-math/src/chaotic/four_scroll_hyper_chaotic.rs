//! 四涡卷超混沌吸引子。
//! Four-scroll hyper-chaotic attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point4;
use super::helpers::{default_float, one_point4};

point4_system!(
    /// 四涡卷超混沌吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the four-scroll hyper-chaotic attractor.
    FourScrollHyperChaoticAttractor,
    /// 四涡卷超混沌吸引子序列生成器。
    /// Four-scroll hyper-chaotic attractor sequence generator.
    FourScrollHyperChaoticAttractorGenerator,
    [a, b, c, d, h],
    |system, state| {
        let dx = -system.a * state.x() + state.y() * state.z() + state.z();
        let dy = system.b * state.y() - state.x() * state.z();
        let dz = -system.c * state.z() + state.x() * state.y() + state.w();
        let dw = state.y() - system.d * state.w();
        Point4::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
            state.w() + system.h * dw,
        )
    }
);

impl<S: Field + Float> Default for FourScrollHyperChaoticAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.5, "0.5 must be representable"),
            default_float(0.5, "0.5 must be representable"),
            default_float(0.5, "0.5 must be representable"),
            default_float(0.5, "0.5 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for FourScrollHyperChaoticAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(FourScrollHyperChaoticAttractor::default(), one_point4())
    }
}

/// 创建四涡卷超混沌吸引子。
/// Create a four-scroll hyper-chaotic attractor.
pub fn four_scroll_hyper_chaotic_attractor<S: Field + Float>(
    a: S, b: S, c: S, d: S, h: S,
) -> FourScrollHyperChaoticAttractor<S> {
    FourScrollHyperChaoticAttractor::new(a, b, c, d, h)
}

/// 创建四涡卷超混沌吸引子生成器。
/// Create a four-scroll hyper-chaotic attractor generator.
pub fn four_scroll_hyper_chaotic_attractor_generator<S: Field + Float>(
    a: S, b: S, c: S, d: S, h: S, x: Point4<S>,
) -> FourScrollHyperChaoticAttractorGenerator<S> {
    FourScrollHyperChaoticAttractorGenerator::new(FourScrollHyperChaoticAttractor::new(a, b, c, d, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_scroll_step_formula() {
        let system = FourScrollHyperChaoticAttractor::<f64>::default();
        let next = system.step(Point4::new(1.0, 1.0, 1.0, 1.0));
        let dx = -0.5 + 1.0 + 1.0;
        let dy = 0.5 - 1.0;
        let dz = -0.5 + 1.0 + 1.0;
        let dw = 1.0 - 0.5;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
        assert!((next.w() - (1.0 + 0.01 * dw)).abs() < 1e-12);
    }
}
