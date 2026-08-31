//! Brusselator 反应模型的一阶欧拉步进模型。
//! First-order Euler step model for the Brusselator reaction model.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point2;
use super::helpers::{default_float, one_point2};

/// Brusselator 反应模型的一阶欧拉步进模型。
/// First-order Euler step model for the Brusselator reaction model.
#[derive(Clone, Debug, PartialEq)]
pub struct Brusselator<S: Field + Float = f64> {
    a: S,
    b: S,
    h: S,
}

impl<S: Field + Float> Brusselator<S> {
    pub fn new(a: S, b: S, h: S) -> Self {
        Self { a, b, h }
    }

    pub fn a(&self) -> S {
        self.a
    }

    pub fn b(&self) -> S {
        self.b
    }

    pub fn h(&self) -> S {
        self.h
    }

    pub fn step(&self, state: Point2<S>) -> Point2<S> {
        let temp1 = self.a * state.x().powi(2) * state.y();
        let temp2 = self.b * state.x();
        let dx = temp1 - temp2 - state.x() + S::one();
        let dy = temp2 - temp1;
        Point2::new(state.x() + self.h * dx, state.y() + self.h * dy)
    }

    pub fn generator(self, initial: Point2<S>) -> BrusselatorGenerator<S> {
        BrusselatorGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for Brusselator<S> {
    fn default() -> Self {
        Self::new(
            S::one(),
            S::one() + S::one() + S::one(),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

/// Brusselator 反应模型序列生成器。
/// Brusselator reaction model sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct BrusselatorGenerator<S: Field + Float = f64> {
    system: Brusselator<S>,
    x: Point2<S>,
}

impl<S: Field + Float> BrusselatorGenerator<S> {
    pub fn new(system: Brusselator<S>, x: Point2<S>) -> Self {
        Self { system, x }
    }

    pub fn system(&self) -> &Brusselator<S> {
        &self.system
    }

    pub fn x(&self) -> &Point2<S> {
        &self.x
    }

    pub fn next_point(&mut self) -> Point2<S> {
        let x = self.x.clone();
        self.x = self.system.step(self.x.clone());
        x
    }
}

impl<S: Field + Float> Default for BrusselatorGenerator<S> {
    fn default() -> Self {
        Self::new(Brusselator::default(), one_point2())
    }
}

impl<S: Field + Float> Iterator for BrusselatorGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建 Brusselator 反应模型。
/// Create a Brusselator reaction model.
pub fn brusselator<S: Field + Float>(a: S, b: S, h: S) -> Brusselator<S> {
    Brusselator::new(a, b, h)
}

/// 创建 Brusselator 反应模型生成器。
/// Create a Brusselator reaction model generator.
pub fn brusselator_generator<S: Field + Float>(
    a: S,
    b: S,
    h: S,
    x: Point2<S>,
) -> BrusselatorGenerator<S> {
    BrusselatorGenerator::new(Brusselator::new(a, b, h), x)
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
    fn brusselator_matches_kotlin_formula() {
        assert_point2_close(
            Brusselator::default().step(Point2::new(1.0, 1.0)),
            Point2::new(0.98, 1.02),
        );
    }
}
