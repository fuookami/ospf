//! Shimizu-Morioka 吸引子。
//! Shimizu-Morioka attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Shimizu-Morioka 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Shimizu-Morioka attractor.
    ShimizuMoriokaAttractor,
    /// Shimizu-Morioka 吸引子序列生成器。
    /// Shimizu-Morioka attractor sequence generator.
    ShimizuMoriokaAttractorGenerator,
    [alpha, beta, h],
    |system, state| {
        let one = S::one();
        let dx = state.y();
        let dy = (one - state.z()) * state.x() - system.alpha * state.y();
        let dz = state.x() * state.x() - system.beta * state.z();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for ShimizuMoriokaAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.75, "0.75 must be representable"),
            default_float(0.45, "0.45 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for ShimizuMoriokaAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(ShimizuMoriokaAttractor::default(), one_point3())
    }
}

/// 创建 Shimizu-Morioka 吸引子。
/// Create a Shimizu-Morioka attractor.
pub fn shimizu_morioka_attractor<S: Field + Float>(alpha: S, beta: S, h: S) -> ShimizuMoriokaAttractor<S> {
    ShimizuMoriokaAttractor::new(alpha, beta, h)
}

/// 创建 Shimizu-Morioka 吸引子生成器。
/// Create a Shimizu-Morioka attractor generator.
pub fn shimizu_morioka_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, h: S, x: Point3<S>,
) -> ShimizuMoriokaAttractorGenerator<S> {
    ShimizuMoriokaAttractorGenerator::new(ShimizuMoriokaAttractor::new(alpha, beta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shimizu_morioka_step_formula() {
        let system = ShimizuMoriokaAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = 1.0;
        let dy = (1.0 - 1.0) * 1.0 - 0.75;
        let dz = 1.0 - 0.45;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
