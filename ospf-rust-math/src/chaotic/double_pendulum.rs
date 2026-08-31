//! 双摆系统。
//! Double pendulum system.

use super::helpers::{default_float, one_point2};
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;

/// 双摆系统的一阶欧拉步进模型。
/// First-order Euler step model for the double pendulum system.
#[derive(Clone, Debug, PartialEq)]
pub struct DoublePendulumSystem<S: Field + Float = f64> {
    m: S,
    l: S,
    g: S,
    h: S,
}

impl<S: Field + Float> DoublePendulumSystem<S> {
    pub fn new(m: S, l: S, g: S, h: S) -> Self {
        Self { m, l, g, h }
    }

    pub fn m(&self) -> S {
        self.m
    }

    pub fn l(&self) -> S {
        self.l
    }

    pub fn g(&self) -> S {
        self.g
    }

    pub fn h(&self) -> S {
        self.h
    }

    pub fn step(&self, x: Point2<S>, y: Point2<S>) -> (Point2<S>, Point2<S>) {
        let two = S::one() + S::one();
        let three = two + S::one();
        let six = three * two;
        let eight = default_float::<S>(8.0, "8.0 must be representable");
        let nine = three * three;
        let sixteen = default_float::<S>(16.0, "16.0 must be representable");
        let theta1 = x.x();
        let omega1 = x.y();
        let theta2 = y.x();
        let omega2 = y.y();
        let sin_temp = (theta1 - theta2).sin();
        let cos_temp = (theta1 - theta2).cos();
        let d_theta_denominator = (self.m * self.l.powi(2)) * (sixteen - nine * cos_temp.powi(2));
        let d_theta1 = six * (two * omega1 - three * cos_temp * omega2) / d_theta_denominator;
        let d_theta2 = six * (eight * omega2 - three * cos_temp * omega1) / d_theta_denominator;
        let d_omega_coefficient = -self.m * self.l.powi(2) / two;
        let d_omega1 = d_omega_coefficient
            * (d_theta1 * d_theta2 * sin_temp + three * self.g / self.l * theta1.sin());
        let d_omega2 = d_omega_coefficient
            * (-d_theta1 * d_theta2 * sin_temp + self.g / self.l * theta2.sin());
        (
            Point2::new(theta1 + self.h * d_theta1, omega1 + self.h * d_omega1),
            Point2::new(omega2 + self.h * d_theta2, omega2 + self.h * d_omega2),
        )
    }

    pub fn generator(self, x: Point2<S>, y: Point2<S>) -> DoublePendulumSystemGenerator<S> {
        DoublePendulumSystemGenerator::new(self, x, y)
    }
}

impl<S: Field + Float> Default for DoublePendulumSystem<S> {
    fn default() -> Self {
        Self::new(
            default_float(10.0, "10.0 must be representable"),
            S::one(),
            default_float(9.80665, "9.80665 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DoublePendulumSystemGenerator<S: Field + Float = f64> {
    system: DoublePendulumSystem<S>,
    x: Point2<S>,
    y: Point2<S>,
}

impl<S: Field + Float> DoublePendulumSystemGenerator<S> {
    pub fn new(system: DoublePendulumSystem<S>, x: Point2<S>, y: Point2<S>) -> Self {
        Self { system, x, y }
    }

    pub fn system(&self) -> &DoublePendulumSystem<S> {
        &self.system
    }

    pub fn x(&self) -> &Point2<S> {
        &self.x
    }

    pub fn y(&self) -> &Point2<S> {
        &self.y
    }

    pub fn next_pair(&mut self) -> (Point2<S>, Point2<S>) {
        let x = self.x.clone();
        let y = self.y.clone();
        let (next_x, next_y) = self.system.step(x.clone(), y.clone());
        self.x = next_x;
        self.y = next_y;
        (x, y)
    }
}

impl<S: Field + Float> Default for DoublePendulumSystemGenerator<S> {
    fn default() -> Self {
        Self::new(DoublePendulumSystem::default(), one_point2(), one_point2())
    }
}

impl<S: Field + Float> Iterator for DoublePendulumSystemGenerator<S> {
    type Item = (Point2<S>, Point2<S>);

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_pair())
    }
}

/// 创建双摆系统。
/// Create a double pendulum system.
pub fn double_pendulum_system<S: Field + Float>(m: S, l: S, g: S, h: S) -> DoublePendulumSystem<S> {
    DoublePendulumSystem::new(m, l, g, h)
}

pub fn double_pendulum_system_generator<S: Field + Float>(
    m: S,
    l: S,
    g: S,
    h: S,
    x: Point2<S>,
    y: Point2<S>,
) -> DoublePendulumSystemGenerator<S> {
    DoublePendulumSystemGenerator::new(DoublePendulumSystem::new(m, l, g, h), x, y)
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

    fn assert_point2_close(actual: Point2<f64>, expected: Point2<f64>) {
        assert_close(actual.x(), expected.x());
        assert_close(actual.y(), expected.y());
    }

    #[test]
    fn biological_and_physical_models_match_kotlin_formulas() {
        let pendulum = DoublePendulumSystem::default();
        let (x, y) = pendulum.step(Point2::new(0.0, 1.0), Point2::new(0.0, 1.0));
        assert_point2_close(x, Point2::new(-0.0008571428571428571, 1.0));
        assert_point2_close(y, Point2::new(1.0042857142857142, 1.0));
    }
}