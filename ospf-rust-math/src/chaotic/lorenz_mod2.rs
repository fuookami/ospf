//! Lorenz 修正 2 吸引子。
//! Lorenz Mod 2 attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Lorenz 修正 2 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Lorenz Mod 2 attractor.
    LorenzMod2Attractor,
    /// Lorenz 修正 2 吸引子序列生成器。
    /// Lorenz Mod 2 attractor sequence generator.
    LorenzMod2AttractorGenerator,
    [alpha, beta, delta, zeta, h],
    |system, state| {
        let dx = -system.alpha * state.x() + state.y() * state.y() - state.z() * state.z()
            + system.alpha * system.zeta;
        let dy = state.x() * (state.y() - system.beta * state.z()) + system.delta;
        let dz = -state.z() + state.x() * (system.beta * state.y() + state.z());
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for LorenzMod2Attractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.9, "0.9 must be representable"),
            default_float(5.0, "5.0 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(9.9, "9.9 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for LorenzMod2AttractorGenerator<S> {
    fn default() -> Self {
        Self::new(LorenzMod2Attractor::default(), one_point3())
    }
}

/// 创建 Lorenz 修正 2 吸引子。
/// Create a Lorenz Mod 2 attractor.
pub fn lorenz_mod2_attractor<S: Field + Float>(
    alpha: S, beta: S, delta: S, zeta: S, h: S,
) -> LorenzMod2Attractor<S> {
    LorenzMod2Attractor::new(alpha, beta, delta, zeta, h)
}

/// 创建 Lorenz 修正 2 吸引子生成器。
/// Create a Lorenz Mod 2 attractor generator.
pub fn lorenz_mod2_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, delta: S, zeta: S, h: S, x: Point3<S>,
) -> LorenzMod2AttractorGenerator<S> {
    LorenzMod2AttractorGenerator::new(LorenzMod2Attractor::new(alpha, beta, delta, zeta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lorenz_mod2_step_formula() {
        let system = LorenzMod2Attractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = -0.9 + 1.0 - 1.0 + 8.91;
        let dy = 1.0 * (1.0 - 5.0) + 1.0;
        let dz = -1.0 + 1.0 * (5.0 + 1.0);
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
