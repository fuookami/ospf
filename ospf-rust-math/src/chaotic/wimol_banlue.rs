//! Wimol-Banlue 吸引子。
//! Wimol-Banlue attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Wimol-Banlue 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Wimol-Banlue attractor.
    WimolBanlueAttractor,
    /// Wimol-Banlue 吸引子序列生成器。
    /// Wimol-Banlue attractor sequence generator.
    WimolBanlueAttractorGenerator,
    [alpha, h],
    |system, state| {
        let dx = state.y() - state.x();
        let dy = -state.z() * state.x().tan();
        let dz = -system.alpha + state.x() * state.y() + state.x().abs();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for WimolBanlueAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(2.0, "2.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for WimolBanlueAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(WimolBanlueAttractor::default(), one_point3())
    }
}

/// 创建 Wimol-Banlue 吸引子。
/// Create a Wimol-Banlue attractor.
pub fn wimol_banlue_attractor<S: Field + Float>(alpha: S, h: S) -> WimolBanlueAttractor<S> {
    WimolBanlueAttractor::new(alpha, h)
}

/// 创建 Wimol-Banlue 吸引子生成器。
/// Create a Wimol-Banlue attractor generator.
pub fn wimol_banlue_attractor_generator<S: Field + Float>(
    alpha: S, h: S, x: Point3<S>,
) -> WimolBanlueAttractorGenerator<S> {
    WimolBanlueAttractorGenerator::new(WimolBanlueAttractor::new(alpha, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wimol_banlue_step_formula() {
        let system = WimolBanlueAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = 1.0 - 1.0;
        let dy = -1.0_f64.tan();
        let dz = -2.0 + 1.0 + 1.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
