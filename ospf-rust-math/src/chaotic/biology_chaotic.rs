//! 生物混沌模型。
//! Biology chaotic model.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
use super::helpers::{one_point3};

point3_system!(
    /// 生物混沌模型的一阶欧拉步进模型。
    /// First-order Euler step model for the biology chaotic model.
    BiologyChaoticModel,
    /// 生物混沌模型序列生成器。
    /// Biology chaotic model sequence generator.
    BiologyChaoticModelGenerator,
    [a, b, c, r],
    |system, state| {
        Point3::new(
            system.r
                * state.x()
                * (S::one() - system.a * state.x() - system.b * state.y() - system.c * state.z()),
            state.x(),
            state.y(),
        )
    }
);

impl<S: Field + Float> Default for BiologyChaoticModel<S> {
    fn default() -> Self {
        let half = S::one() / (S::one() + S::one());
        Self::new(half, half, half, half)
    }
}

impl<S: Field + Float> Default for BiologyChaoticModelGenerator<S> {
    fn default() -> Self {
        Self::new(BiologyChaoticModel::default(), one_point3())
    }
}

/// 创建生物混沌模型。
/// Create a biology chaotic model.
pub fn biology_chaotic_model<S: Field + Float>(a: S, b: S, c: S, r: S) -> BiologyChaoticModel<S> {
    BiologyChaoticModel::new(a, b, c, r)
}

/// 创建生物混沌模型生成器。
/// Create a biology chaotic model generator.
pub fn biology_chaotic_model_generator<S: Field + Float>(
    a: S,
    b: S,
    c: S,
    r: S,
    x: Point3<S>,
) -> BiologyChaoticModelGenerator<S> {
    BiologyChaoticModelGenerator::new(BiologyChaoticModel::new(a, b, c, r), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "actual={actual}, expected={expected}"
        );
    }

    fn assert_point3_close(actual: Point3<f64>, expected: Point3<f64>) {
        assert_close(actual.x(), expected.x());
        assert_close(actual.y(), expected.y());
        assert_close(actual.z(), expected.z());
    }

    #[test]
    fn biological_and_physical_models_match_kotlin_formulas() {
        assert_point3_close(
            BiologyChaoticModel::default().step(Point3::new(1.0, 1.0, 1.0)),
            Point3::new(-0.25, 1.0, 1.0),
        );
    }
}