//! Newton-Leipnik 吸引子。
//! Newton-Leipnik attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{default_float, one_point3};

point3_system!(
    /// Newton-Leipnik 吸引子的一阶欧拉步进模型。
    /// First-order Euler step model for the Newton-Leipnik attractor.
    NewtonLeipnikAttractor,
    /// Newton-Leipnik 吸引子序列生成器。
    /// Newton-Leipnik attractor sequence generator.
    NewtonLeipnikAttractorGenerator,
    [alpha, beta, h],
    |system, state| {
        let c10 = S::from(10.0).expect("10.0 must be representable");
        let c5 = S::from(5.0).expect("5.0 must be representable");
        let c04 = S::from(0.4).expect("0.4 must be representable");
        let dx = -system.alpha * state.x() + state.y() + c10 * state.y() * state.z();
        let dy = -state.x() - c04 * state.y() + c5 * state.x() * state.z();
        let dz = system.beta * state.z() - c5 * state.x() * state.y();
        Point3::new(
            state.x() + system.h * dx,
            state.y() + system.h * dy,
            state.z() + system.h * dz,
        )
    }
);

impl<S: Field + Float> Default for NewtonLeipnikAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(0.4, "0.4 must be representable"),
            default_float(0.175, "0.175 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

impl<S: Field + Float> Default for NewtonLeipnikAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(NewtonLeipnikAttractor::default(), one_point3())
    }
}

/// 创建 Newton-Leipnik 吸引子。
/// Create a Newton-Leipnik attractor.
pub fn newton_leipnik_attractor<S: Field + Float>(alpha: S, beta: S, h: S) -> NewtonLeipnikAttractor<S> {
    NewtonLeipnikAttractor::new(alpha, beta, h)
}

/// 创建 Newton-Leipnik 吸引子生成器。
/// Create a Newton-Leipnik attractor generator.
pub fn newton_leipnik_attractor_generator<S: Field + Float>(
    alpha: S, beta: S, h: S, x: Point3<S>,
) -> NewtonLeipnikAttractorGenerator<S> {
    NewtonLeipnikAttractorGenerator::new(NewtonLeipnikAttractor::new(alpha, beta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newton_leipnik_step_formula() {
        let system = NewtonLeipnikAttractor::<f64>::default();
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        let dx = -0.4 + 1.0 + 10.0;
        let dy = -1.0 - 0.4 + 5.0;
        let dz = 0.175 - 5.0;
        assert!((next.x() - (1.0 + 0.01 * dx)).abs() < 1e-12);
        assert!((next.y() - (1.0 + 0.01 * dy)).abs() < 1e-12);
        assert!((next.z() - (1.0 + 0.01 * dz)).abs() < 1e-12);
    }
}
