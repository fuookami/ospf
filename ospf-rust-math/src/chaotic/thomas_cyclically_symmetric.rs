//! Thomas 循环对称吸引子。
//! Thomas cyclically symmetric attractor.
//!
//! 与 [`super::thomas::ThomasAttractor`] 公式相同，仅默认参数不同。
//! Same formula as [`super::thomas::ThomasAttractor`], only default parameters differ.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

point3_system!(
    /// Thomas 循环对称吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Thomas cyclically symmetric attractor.
    ThomasCyclicallySymmetricAttractor,
    /// Thomas 循环对称吸引子序列生成器。
    /// Thomas cyclically symmetric attractor sequence generator.
    ThomasCyclicallySymmetricAttractorGenerator,
    [b, h],
    |system, state| {
        let dx = state.x().sin() - system.b * state.x();
        let dy = state.y().sin() - system.b * state.y();
        let dz = state.z().sin() - system.b * state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for ThomasCyclicallySymmetricAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.208186, "0.208186 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for ThomasCyclicallySymmetricAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(ThomasCyclicallySymmetricAttractor::default(), one_point3())
    }
}

/// 创建 Thomas 循环对称吸引子。
/// Create a Thomas cyclically symmetric attractor.
pub fn thomas_cyclically_symmetric_attractor<S: Field + Float>(
    b: S,
    h: S,
) -> ThomasCyclicallySymmetricAttractor<S> {
    ThomasCyclicallySymmetricAttractor::new(b, h)
}

/// 创建 Thomas 循环对称吸引子生成器。
/// Create a Thomas cyclically symmetric attractor generator.
pub fn thomas_cyclically_symmetric_attractor_generator<S: Field + Float>(
    b: S,
    h: S,
    x: Point3<S>,
) -> ThomasCyclicallySymmetricAttractorGenerator<S> {
    ThomasCyclicallySymmetricAttractorGenerator::new(
        ThomasCyclicallySymmetricAttractor::new(b, h),
        x,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thomas_cyclically_symmetric_step_formula() {
        let system = ThomasCyclicallySymmetricAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let sin1 = 1.0_f64.sin();
        let dx = sin1 - 0.208186;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
    }
}
