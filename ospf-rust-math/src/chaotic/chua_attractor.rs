//! Chua 吸引子的一阶欧拉步进模型。
//! First-order Euler step model for the Chua attractor.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

/// Chua 吸引子的一阶欧拉步进模型。
/// First-order Euler step model for the Chua attractor.
#[derive(Clone, Debug, PartialEq)]
pub struct ChuaAttractor<S: Field + Float = f64> {
    alpha: S,
    beta: S,
    delta: S,
    epsilon: S,
    zeta: S,
    h: S,
}

impl<S: Field + Float> ChuaAttractor<S> {
    pub fn new(alpha: S, beta: S, delta: S, epsilon: S, zeta: S, h: S) -> Self {
        Self {
            alpha,
            beta,
            delta,
            epsilon,
            zeta,
            h,
        }
    }

    pub fn alpha(&self) -> S {
        self.alpha
    }

    pub fn beta(&self) -> S {
        self.beta
    }

    pub fn delta(&self) -> S {
        self.delta
    }

    pub fn epsilon(&self) -> S {
        self.epsilon
    }

    pub fn zeta(&self) -> S {
        self.zeta
    }

    pub fn h(&self) -> S {
        self.h
    }

    pub fn step(&self, x: Point3<S>) -> Point3<S> {
        let g = self.epsilon * x.x()
            + (self.delta - self.epsilon) * ((x.x() + S::one()).abs() - (x.x() - S::one()).abs());
        let dx = self.alpha * (x.y() - x.x() - g);
        let dy = self.beta * (x.x() - x.y() + x.z());
        let dz = -self.zeta * x.y();
        Point3::new(
            x.x() + self.h * dx,
            x.y() + self.h * dy,
            x.z() + self.h * dz,
        )
    }

    pub fn generator(self, initial: Point3<S>) -> ChuaAttractorGenerator<S> {
        ChuaAttractorGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for ChuaAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(15.6, "15.6 must be representable"),
            S::one(),
            default_float(-1.0, "-1.0 must be representable"),
            S::zero(),
            default_float(25.58, "25.58 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

/// Chua 吸引子序列生成器。
/// Chua attractor sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct ChuaAttractorGenerator<S: Field + Float = f64> {
    system: ChuaAttractor<S>,
    x: Point3<S>,
}

impl<S: Field + Float> ChuaAttractorGenerator<S> {
    pub fn new(system: ChuaAttractor<S>, x: Point3<S>) -> Self {
        Self { system, x }
    }

    pub fn system(&self) -> &ChuaAttractor<S> {
        &self.system
    }

    pub fn x(&self) -> &Point3<S> {
        &self.x
    }

    pub fn next_point(&mut self) -> Point3<S> {
        self.x = self.system.step(self.x.clone());
        self.x.clone()
    }
}

impl<S: Field + Float> Default for ChuaAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(ChuaAttractor::default(), one_point3())
    }
}

impl<S: Field + Float> Iterator for ChuaAttractorGenerator<S> {
    type Item = Point3<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建 Chua 吸引子。
/// Create a Chua attractor.
pub fn chua_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    epsilon: S,
    zeta: S,
    h: S,
) -> ChuaAttractor<S> {
    ChuaAttractor::new(alpha, beta, delta, epsilon, zeta, h)
}

/// 创建 Chua 吸引子生成器。
/// Create a Chua attractor generator.
pub fn chua_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    epsilon: S,
    zeta: S,
    h: S,
    x: Point3<S>,
) -> ChuaAttractorGenerator<S> {
    ChuaAttractorGenerator::new(ChuaAttractor::new(alpha, beta, delta, epsilon, zeta, h), x)
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
    fn chua_attractor_matches_kotlin_formula() {
        let next = ChuaAttractor::default().step(Point3::new(1.0, 1.0, 1.0));
        assert_point3_close(next.clone(), Point3::new(1.312, 1.01, 0.7442));
    }

    #[test]
    fn chua_attractor_generator_semantics() {
        let mut attractor_generator =
            ChuaAttractorGenerator::new(ChuaAttractor::default(), Point3::new(1.0, 1.0, 1.0));
        assert_point3_close(attractor_generator.next_point(), Point3::new(1.312, 1.01, 0.7442));
        assert_point3_close(
            attractor_generator.x().clone(),
            Point3::new(1.312, 1.01, 0.7442),
        );
    }
}
