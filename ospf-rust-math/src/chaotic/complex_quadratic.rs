//! 复二次多项式。
//! Complex quadratic polynomial.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point2;
use super::helpers::one_point2;

/// 复二次多项式的一阶欧拉步进模型。
/// First-order Euler step model for the complex quadratic polynomial.
#[derive(Clone, Debug, PartialEq)]
pub struct ComplexQuadraticPolynomial<S: Field + Float = f64> {
    c: Point2<S>,
    d: S,
}

impl<S: Field + Float> ComplexQuadraticPolynomial<S> {
    pub fn new(c: Point2<S>, d: S) -> Self {
        Self { c, d }
    }

    pub fn c(&self) -> &Point2<S> {
        &self.c
    }

    pub fn d(&self) -> S {
        self.d
    }

    pub fn step(&self, x: Point2<S>) -> Point2<S> {
        let radius = (x.x() * x.x() + x.y() * x.y()).sqrt();
        let theta = x.y().atan2(x.x());
        let magnitude = radius.powf(self.d);
        let angle = self.d * theta;
        Point2::new(
            magnitude * angle.cos() + self.c.x(),
            magnitude * angle.sin() + self.c.y(),
        )
    }

    pub fn generator(self, initial: Point2<S>) -> ComplexQuadraticPolynomialGenerator<S> {
        ComplexQuadraticPolynomialGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for ComplexQuadraticPolynomial<S> {
    fn default() -> Self {
        Self::new(Point2::new(S::zero(), S::zero()), S::one() + S::one())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComplexQuadraticPolynomialGenerator<S: Field + Float = f64> {
    system: ComplexQuadraticPolynomial<S>,
    x: Point2<S>,
}

impl<S: Field + Float> ComplexQuadraticPolynomialGenerator<S> {
    pub fn new(system: ComplexQuadraticPolynomial<S>, x: Point2<S>) -> Self {
        Self { system, x }
    }

    pub fn system(&self) -> &ComplexQuadraticPolynomial<S> {
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

impl<S: Field + Float> Default for ComplexQuadraticPolynomialGenerator<S> {
    fn default() -> Self {
        Self::new(ComplexQuadraticPolynomial::default(), one_point2())
    }
}

impl<S: Field + Float> Iterator for ComplexQuadraticPolynomialGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建复二次多项式。
/// Create a complex quadratic polynomial.
pub fn complex_quadratic_polynomial<S: Field + Float>(
    c: Point2<S>,
    d: S,
) -> ComplexQuadraticPolynomial<S> {
    ComplexQuadraticPolynomial::new(c, d)
}

/// 创建复二次多项式生成器。
/// Create a complex quadratic polynomial generator.
pub fn complex_quadratic_polynomial_generator<S: Field + Float>(
    c: Point2<S>,
    d: S,
    x: Point2<S>,
) -> ComplexQuadraticPolynomialGenerator<S> {
    ComplexQuadraticPolynomialGenerator::new(ComplexQuadraticPolynomial::new(c, d), x)
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
    fn complex_maps_match_kotlin_formulas() {
        assert_point2_close(
            ComplexQuadraticPolynomial::new(Point2::new(1.0, 1.0), 2.0).step(Point2::new(1.0, 2.0)),
            Point2::new(-2.0, 5.0),
        );
    }
}