//! 三涡卷统一混沌系统 TSUCS1。
//! Three-Scroll Unified Chaotic System TSUCS1.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// 三涡卷统一混沌系统 TSUCS1 的一阶欧拉步进模型。
    /// First-order Euler step model for the Three-Scroll Unified Chaotic System TSUCS1.
    ThreeScrollUnifiedChaoticSystemTsucs1Attractor,
    /// 三涡卷统一混沌系统 TSUCS1 序列生成器。
    /// Three-Scroll Unified Chaotic System TSUCS1 sequence generator.
    ThreeScrollUnifiedChaoticSystemTsucs1AttractorGenerator,
    [alpha, beta, delta, epsilon, zeta, rho, h],
    |system, state| {
        let dx = system.alpha * (state.y() - state.x()) + system.delta * state.x() * state.z();
        let dy = system.rho * state.x() - state.x() * state.z() + system.zeta * state.y();
        let dz = system.beta * state.z() + state.x() * state.y() - system.epsilon * state.x() * state.x();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for ThreeScrollUnifiedChaoticSystemTsucs1Attractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(40.0, "40.0 must be representable"),
            default_float(0.833, "0.833 must be representable"),
            default_float(0.5, "0.5 must be representable"),
            default_float(0.65, "0.65 must be representable"),
            default_float(20.0, "20.0 must be representable"),
            default_float(55.0, "55.0 must be representable"),
            default_float(0.001, "0.001 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for ThreeScrollUnifiedChaoticSystemTsucs1AttractorGenerator<S> {
    fn default() -> Self {
        Self::new(ThreeScrollUnifiedChaoticSystemTsucs1Attractor::default(), one_point3())
    }
}

/// 创建三涡卷统一混沌系统 TSUCS1。
/// Create a Three-Scroll Unified Chaotic System TSUCS1.
pub fn three_scroll_tsucs1_attractor<S: Field + Float>(
    alpha: S, beta: S, delta: S, epsilon: S, zeta: S, rho: S, h: S,
) -> ThreeScrollUnifiedChaoticSystemTsucs1Attractor<S> {
    ThreeScrollUnifiedChaoticSystemTsucs1Attractor::new(alpha, beta, delta, epsilon, zeta, rho, h)
}

/// 创建三涡卷统一混沌系统 TSUCS1 生成器。
/// Create a Three-Scroll Unified Chaotic System TSUCS1 generator.
pub fn three_scroll_tsucs1_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, delta: S, epsilon: S, zeta: S, rho: S, h: S, x: Point3<S>,
) -> ThreeScrollUnifiedChaoticSystemTsucs1AttractorGenerator<S> {
    ThreeScrollUnifiedChaoticSystemTsucs1AttractorGenerator::new(
        ThreeScrollUnifiedChaoticSystemTsucs1Attractor::new(alpha, beta, delta, epsilon, zeta, rho, h),
        x,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsucs1_step_formula() {
        let system = ThreeScrollUnifiedChaoticSystemTsucs1Attractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = 40.0 * (1.0 - 1.0) + 0.5;
        let dy = 55.0 - 1.0 + 20.0;
        let dz = 0.833 + 1.0 - 0.65;
        assert!((next.x() - (1.0 + 0.001 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.001 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.001 * dz)).abs() < 1e-12);
    }
}
