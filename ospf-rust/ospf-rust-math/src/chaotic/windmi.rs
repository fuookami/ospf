//! Windmi 混沌吸引子 / Windmi chaotic attractor
//!
//! 由三个常微分方程组成的混沌系统，采用前向欧拉法推进一步。
//! A chaotic system of three ordinary differential equations, advanced by one forward Euler step.
//!
//! 公式 / Formula:
//! x' = a * (y - x) + d * x * z
//! y' = b * x - x * z + f * y
//! z' = c * z + x * y - e * x * x
//!
//! `g` 为保留参数（当前方程未使用），`h` 为时间步长。
//! `g` is a reserved parameter unused by the current equations; `h` is the time step size.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Windmi 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Windmi attractor.
    WindmiAttractor,
    /// Windmi 吸引子序列生成器。
    /// Windmi attractor sequence generator.
    WindmiAttractorGenerator,
    [a, b, c, d, e, f, g, h],
    |system, state| {
        let x = state.x();
        let y = state.y();
        let z = state.z();

        let dx = system.a * (y - x) + system.d * x * z;
        let dy = system.b * x - x * z + system.f * y;
        let dz = system.c * z + x * y - system.e * x * x;

        Point3::new(
            x + system.h * dx,
            y + system.h * dy,
            z + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for WindmiAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.2, "0.2 must be representable"),
            default_float(0.2, "0.2 must be representable"),
            default_float(2.2, "2.2 must be representable"),
            default_float(0.5, "0.5 must be representable"),
            default_float(0.2, "0.2 must be representable"),
            default_float(5.7, "5.7 must be representable"),
            default_float(0.5, "0.5 must be representable"),
            default_float(0.1, "0.1 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for WindmiAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(WindmiAttractor::default(), one_point3())
    }
}

/// 创建 Windmi 吸引子 / Create a Windmi attractor
#[allow(clippy::too_many_arguments)]
pub fn windmi_attractor<S: Field + Float>(
    a: S,
    b: S,
    c: S,
    d: S,
    e: S,
    f: S,
    g: S,
    h: S,
) -> WindmiAttractor<S> {
    WindmiAttractor::new(a, b, c, d, e, f, g, h)
}

/// 创建 Windmi 序列生成器 / Create a Windmi sequence generator
pub fn windmi_attractor_generator<S: Field + Float>(
    attractor: WindmiAttractor<S>,
    initial: Point3<S>,
) -> WindmiAttractorGenerator<S> {
    WindmiAttractorGenerator::new(attractor, initial)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windmi_step_follows_the_equations() {
        let attractor = WindmiAttractor::<f64>::default();
        let state = Point3::new(0.1, 0.2, 0.3);
        let next = attractor.step(state);
        let dx = 0.2 * (0.2 - 0.1) + 0.5 * 0.1 * 0.3;
        let dy = 0.2 * 0.1 - 0.1 * 0.3 + 5.7 * 0.2;
        let dz = 2.2 * 0.3 + 0.1 * 0.2 - 0.2 * 0.1 * 0.1;
        assert!((next.x() - (0.1 + 0.1 * dx)).abs() < 1e-12);
        assert!((next.y() - (0.2 + 0.1 * dy)).abs() < 1e-12);
        assert!((next.z() - (0.3 + 0.1 * dz)).abs() < 1e-12);
    }

    #[test]
    fn windmi_generator_advances_its_state() {
        let mut generator = WindmiAttractorGenerator::<f64>::default();
        let first = generator.next_point();
        let second = generator.next_point();
        assert_ne!(first, second);
    }
}
