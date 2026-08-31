//! 三涡卷统一混沌系统 TSUCS2。
//! Three-Scroll Unified Chaotic System TSUCS2.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// 三涡卷统一混沌系统 TSUCS2 的一阶欧拉步进模型。
    /// First-order Euler step model for the Three-Scroll Unified Chaotic System TSUCS2.
    ThreeScrollUnifiedChaoticSystemTsucs2Attractor,
    /// 三涡卷统一混沌系统 TSUCS2 序列生成器。
    /// Three-Scroll Unified Chaotic System TSUCS2 sequence generator.
    ThreeScrollUnifiedChaoticSystemTsucs2AttractorGenerator,
    [alpha, beta, delta, zeta, rho, h],
    |system, state| {
        let three = S::one() + S::one() + S::one();
        let dx = system.alpha * (state.y() - state.x()) + system.delta * state.x() * state.z();
        let dy = system.rho * state.x() - state.x() * state.z() + system.zeta * state.y();
        let dz = system.beta * state.z() + state.x() * state.y() / three;
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for ThreeScrollUnifiedChaoticSystemTsucs2Attractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(40.0, "40.0 must be representable"),
            default_float(0.833, "0.833 must be representable"),
            default_float(0.5, "0.5 must be representable"),
            default_float(20.0, "20.0 must be representable"),
            default_float(55.0, "55.0 must be representable"),
            default_float(0.001, "0.001 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for ThreeScrollUnifiedChaoticSystemTsucs2AttractorGenerator<S> {
    fn default() -> Self {
        Self::new(ThreeScrollUnifiedChaoticSystemTsucs2Attractor::default(), one_point3())
    }
}

/// 创建三涡卷统一混沌系统 TSUCS2。
/// Create a Three-Scroll Unified Chaotic System TSUCS2.
pub fn three_scroll_tsucs2_attractor<S: Field + Float>(
    alpha: S, beta: S, delta: S, zeta: S, rho: S, h: S,
) -> ThreeScrollUnifiedChaoticSystemTsucs2Attractor<S> {
    ThreeScrollUnifiedChaoticSystemTsucs2Attractor::new(alpha, beta, delta, zeta, rho, h)
}

/// 创建三涡卷统一混沌系统 TSUCS2 生成器。
/// Create a Three-Scroll Unified Chaotic System TSUCS2 generator.
pub fn three_scroll_tsucs2_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, delta: S, zeta: S, rho: S, h: S, x: Point3<S>,
) -> ThreeScrollUnifiedChaoticSystemTsucs2AttractorGenerator<S> {
    ThreeScrollUnifiedChaoticSystemTsucs2AttractorGenerator::new(
        ThreeScrollUnifiedChaoticSystemTsucs2Attractor::new(alpha, beta, delta, zeta, rho, h),
        x,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsucs2_step_formula() {
        let system = ThreeScrollUnifiedChaoticSystemTsucs2Attractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = 40.0 * (1.0 - 1.0) + 0.5;
        let dy = 55.0 - 1.0 + 20.0;
        let dz = 0.833 + 1.0 / 3.0;
        assert!((next.x() - (1.0 + 0.001 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.001 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.001 * dz)).abs() < 1e-12);
    }
}
