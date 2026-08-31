//! 耦合 Lorenz 吸引子。
//! Coupled Lorenz attractor.

use super::helpers::{default_float, one_point3};
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

#[derive(Clone, Debug, PartialEq)]
pub struct CoupledLorenzAttractor<S: Field + Float = f64> {
    beta: S,
    gamma1: S,
    gamma2: S,
    epsilon: S,
    omicron: S,
    h: S,
}

impl<S: Field + Float> CoupledLorenzAttractor<S> {
    pub fn new(beta: S, gamma1: S, gamma2: S, epsilon: S, omicron: S, h: S) -> Self {
        Self {
            beta,
            gamma1,
            gamma2,
            epsilon,
            omicron,
            h,
        }
    }

    pub fn beta(&self) -> S {
        self.beta
    }

    pub fn gamma1(&self) -> S {
        self.gamma1
    }

    pub fn gamma2(&self) -> S {
        self.gamma2
    }

    pub fn epsilon(&self) -> S {
        self.epsilon
    }

    pub fn omicron(&self) -> S {
        self.omicron
    }

    pub fn h(&self) -> S {
        self.h
    }

    pub fn step(&self, x: (Point3<S>, Point3<S>)) -> (Point3<S>, Point3<S>) {
        let (x1, x2) = x;
        let dx1 = self.omicron * (x1.y() - x1.x());
        let dy1 = self.gamma1 * x1.x() - x1.y() - x1.x() * x1.z();
        let dz1 = self.beta * x1.z() + x1.x() * x1.y();
        let dx2 = self.omicron * (x2.y() - x2.x()) + self.epsilon * (x1.x() - x2.x());
        let dy2 = self.gamma2 * x2.x() - x2.y() - x2.x() * x2.z();
        let dz2 = self.beta * x2.z() + x2.x() * x2.y();
        (
            Point3::new(
                x1.x() + self.h * dx1,
                x1.y() + self.h * dy1,
                x1.z() + self.h * dz1,
            ),
            Point3::new(
                x2.x() + self.h * dx2,
                x2.y() + self.h * dy2,
                x2.z() + self.h * dz2,
            ),
        )
    }

    pub fn generator(self, x: Point3<S>, y: Point3<S>) -> CoupledLorenzAttractorGenerator<S> {
        CoupledLorenzAttractorGenerator::new(self, x, y)
    }
}

impl<S: Field + Float> Default for CoupledLorenzAttractor<S> {
    fn default() -> Self {
        Self::new(
            default_float(8.0 / 3.0, "8.0 / 3.0 must be representable"),
            default_float(35.0, "35.0 must be representable"),
            default_float(1.15, "1.15 must be representable"),
            default_float(2.85, "2.85 must be representable"),
            default_float(2.85, "2.85 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoupledLorenzAttractorGenerator<S: Field + Float = f64> {
    system: CoupledLorenzAttractor<S>,
    x: Point3<S>,
    y: Point3<S>,
}

impl<S: Field + Float> CoupledLorenzAttractorGenerator<S> {
    pub fn new(system: CoupledLorenzAttractor<S>, x: Point3<S>, y: Point3<S>) -> Self {
        Self { system, x, y }
    }

    pub fn system(&self) -> &CoupledLorenzAttractor<S> {
        &self.system
    }

    pub fn x(&self) -> &Point3<S> {
        &self.x
    }

    pub fn y(&self) -> &Point3<S> {
        &self.y
    }

    pub fn next_pair(&mut self) -> (Point3<S>, Point3<S>) {
        let x = self.x.clone();
        let y = self.y.clone();
        let (next_x, next_y) = self.system.step((x.clone(), y.clone()));
        self.x = next_x;
        self.y = next_y;
        (x, y)
    }
}

impl<S: Field + Float> Default for CoupledLorenzAttractorGenerator<S> {
    fn default() -> Self {
        Self::new(
            CoupledLorenzAttractor::default(),
            one_point3(),
            one_point3(),
        )
    }
}

impl<S: Field + Float> Iterator for CoupledLorenzAttractorGenerator<S> {
    type Item = (Point3<S>, Point3<S>);

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_pair())
    }
}

pub fn coupled_lorenz_attractor<S: Field + Float>(
    beta: S,
    gamma1: S,
    gamma2: S,
    epsilon: S,
    omicron: S,
    h: S,
) -> CoupledLorenzAttractor<S> {
    CoupledLorenzAttractor::new(beta, gamma1, gamma2, epsilon, omicron, h)
}

pub fn coupled_lorenz_attractor_generator<S: Field + Float>(
    beta: S,
    gamma1: S,
    gamma2: S,
    epsilon: S,
    omicron: S,
    h: S,
    x: Point3<S>,
    y: Point3<S>,
) -> CoupledLorenzAttractorGenerator<S> {
    CoupledLorenzAttractorGenerator::new(
        CoupledLorenzAttractor::new(beta, gamma1, gamma2, epsilon, omicron, h),
        x,
        y,
    )
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
        let coupled = CoupledLorenzAttractor::default();
        let (x1, x2) = coupled.step((Point3::new(1.0, 1.0, 1.0), Point3::new(2.0, 2.0, 2.0)));
        assert_point3_close(x1, Point3::new(1.0, 1.33, 1.0366666666666666));
        assert_point3_close(x2, Point3::new(1.9715, 1.963, 2.0933333333333333));
    }
}
