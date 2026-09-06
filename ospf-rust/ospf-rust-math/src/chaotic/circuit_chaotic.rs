//! 电路混沌系统的一阶欧拉步进模型。
//! First-order Euler step model for the circuit chaotic system.

use super::helpers::one_point2;
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;

/// 电路混沌系统的一阶欧拉步进模型。
/// First-order Euler step model for the circuit chaotic system.
#[derive(Clone, Debug, PartialEq)]
pub struct CircuitChaotic<S: Field + Float = f64> {
    a: S,
    b: S,
    c: S,
    d: S,
}

impl<S: Field + Float> CircuitChaotic<S> {
    pub fn new(a: S, b: S, c: S, d: S) -> Self {
        Self { a, b, c, d }
    }

    pub fn a(&self) -> S {
        self.a
    }

    pub fn b(&self) -> S {
        self.b
    }

    pub fn c(&self) -> S {
        self.c
    }

    pub fn d(&self) -> S {
        self.d
    }

    pub fn step(&self, state: Point2<S>) -> Point2<S> {
        Point2::new(
            self.a * state.y() - self.d * state.y().powi(2),
            -self.b * state.x() + self.c * state.y(),
        )
    }

    pub fn generator(self, initial: Point2<S>) -> CircuitChaoticGenerator<S> {
        CircuitChaoticGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for CircuitChaotic<S> {
    fn default() -> Self {
        Self::new(S::one(), S::one(), S::one(), S::one())
    }
}

/// 电路混沌系统序列生成器。
/// Circuit chaotic system sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct CircuitChaoticGenerator<S: Field + Float = f64> {
    system: CircuitChaotic<S>,
    x: Point2<S>,
}

impl<S: Field + Float> CircuitChaoticGenerator<S> {
    pub fn new(system: CircuitChaotic<S>, x: Point2<S>) -> Self {
        Self { system, x }
    }

    pub fn system(&self) -> &CircuitChaotic<S> {
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

impl<S: Field + Float> Default for CircuitChaoticGenerator<S> {
    fn default() -> Self {
        Self::new(CircuitChaotic::default(), one_point2())
    }
}

impl<S: Field + Float> Iterator for CircuitChaoticGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建电路混沌系统。
/// Create a circuit chaotic system.
pub fn circuit_chaotic<S: Field + Float>(a: S, b: S, c: S, d: S) -> CircuitChaotic<S> {
    CircuitChaotic::new(a, b, c, d)
}

/// 创建电路混沌系统生成器。
/// Create a circuit chaotic system generator.
pub fn circuit_chaotic_generator<S: Field + Float>(
    a: S,
    b: S,
    c: S,
    d: S,
    x: Point2<S>,
) -> CircuitChaoticGenerator<S> {
    CircuitChaoticGenerator::new(CircuitChaotic::new(a, b, c, d), x)
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
    fn circuit_chaotic_matches_kotlin_formula() {
        assert_point2_close(
            CircuitChaotic::default().step(Point2::new(1.0, 2.0)),
            Point2::new(-2.0, 1.0),
        );
    }
}
