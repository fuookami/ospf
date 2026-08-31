//! 四翼吸引子。
//! Four-Wing attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// 四翼吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Four-Wing attractor.
    FourWingAttractor,
    /// 四翼吸引子序列生成器。
    /// Four-Wing attractor sequence generator.
    FourWingAttractorGenerator,
    [alpha, beta, delta, zeta, kappa, h],
    |system, state| {
        let dx = system.alpha * state.x() - system.beta * state.y() * state.z();
        let dy = -system.zeta * state.y() + state.x() * state.z();
        let dz = system.kappa * state.x() - system.delta * state.z() + state.x() * state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for FourWingAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(4.0, "4.0 must be representable"),
            default_float(6.0, "6.0 must be representable"),
            default_float(5.0, "5.0 must be representable"),
            default_float(10.0, "10.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for FourWingAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(FourWingAttractor::default(), one_point3())
    }
}

/// 创建四翼吸引子。
/// Create a Four-Wing attractor.
pub fn four_wing_attractor<S: Field + Float>(
    alpha: S, beta: S, delta: S, zeta: S, kappa: S, h: S,
) -> FourWingAttractor<S> {
    FourWingAttractor::new(alpha, beta, delta, zeta, kappa, h)
}

/// 创建四翼吸引子生成器。
/// Create a Four-Wing attractor generator.
pub fn four_wing_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, delta: S, zeta: S, kappa: S, h: S, x: Point3<S>,
) -> FourWingAttractorGenerator<S> {
    FourWingAttractorGenerator::new(FourWingAttractor::new(alpha, beta, delta, zeta, kappa, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_wing_step_formula() {
        let system = FourWingAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = 4.0 - 6.0;
        let dy = -10.0 + 1.0;
        let dz = 1.0 - 5.0 + 1.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
