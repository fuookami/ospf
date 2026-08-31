//! Genesio-Tesi 吸引子。
//! Genesio-Tesi attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Genesio-Tesi 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Genesio-Tesi attractor.
    GenesioTesiAttractor,
    /// Genesio-Tesi 吸引子序列生成器。
    /// Genesio-Tesi attractor sequence generator.
    GenesioTesiAttractorGenerator,
    [alpha, beta, delta, h],
    |system, state| {
        let dx = state.y();
        let dy = state.z();
        let dz = -system.delta * state.x() - system.beta * state.y() - system.alpha * state.z()
            + state.x() * state.x();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for GenesioTesiAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.44, "0.44 must be representable"),
            default_float(1.1, "1.1 must be representable"),
            default_float(1.0, "1.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for GenesioTesiAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(GenesioTesiAttractor::default(), one_point3())
    }
}

/// 创建 Genesio-Tesi 吸引子。
/// Create a Genesio-Tesi attractor.
pub fn genesio_tesi_attractor<S: Field + Float>(alpha: S, beta: S, delta: S, h: S) -> GenesioTesiAttractor<S> {
    GenesioTesiAttractor::new(alpha, beta, delta, h)
}

/// 创建 Genesio-Tesi 吸引子生成器。
/// Create a Genesio-Tesi attractor generator.
pub fn genesio_tesi_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, delta: S, h: S, x: Point3<S>,
) -> GenesioTesiAttractorGenerator<S> {
    GenesioTesiAttractorGenerator::new(GenesioTesiAttractor::new(alpha, beta, delta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesio_tesi_step_formula() {
        let system = GenesioTesiAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dz = -1.0 - 1.1 - 0.44 + 1.0;
        assert!((next.x() - 1.01).abs() < 1e-12);
        assert!((next.y() - 1.01).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
