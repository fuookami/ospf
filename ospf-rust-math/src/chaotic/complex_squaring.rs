//! 复平方映射。
//! Complex squaring map.

use super::helpers::one_point2;
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;
use std::marker::PhantomData;

/// 复平方映射的一阶欧拉步进模型。
/// First-order Euler step model for the complex squaring map.
#[derive(Clone, Debug, PartialEq)]
pub struct ComplexSquaringMap<S: Field + Float = f64> {
    _marker: PhantomData<S>,
}

impl<S: Field + Float> ComplexSquaringMap<S> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    pub fn step(&self, x: Point2<S>) -> Point2<S> {
        Point2::new(
            x.x() * x.x() - x.y() * x.y(),
            (S::one() + S::one()) * x.x() * x.y(),
        )
    }

    pub fn generator(self, initial: Point2<S>) -> ComplexSquaringMapGenerator<S> {
        ComplexSquaringMapGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for ComplexSquaringMap<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComplexSquaringMapGenerator<S: Field + Float = f64> {
    system: ComplexSquaringMap<S>,
    x: Point2<S>,
}

impl<S: Field + Float> ComplexSquaringMapGenerator<S> {
    pub fn new(system: ComplexSquaringMap<S>, x: Point2<S>) -> Self {
        Self { system, x }
    }

    pub fn system(&self) -> &ComplexSquaringMap<S> {
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

impl<S: Field + Float> Default for ComplexSquaringMapGenerator<S> {
    fn default() -> Self {
        Self::new(ComplexSquaringMap::default(), one_point2())
    }
}

impl<S: Field + Float> Iterator for ComplexSquaringMapGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建复平方映射。
/// Create a complex squaring map.
pub fn complex_squaring_map<S: Field + Float>() -> ComplexSquaringMap<S> {
    ComplexSquaringMap::new()
}

/// 创建复平方映射生成器。
/// Create a complex squaring map generator.
pub fn complex_squaring_map_generator<S: Field + Float>(
    x: Point2<S>,
) -> ComplexSquaringMapGenerator<S> {
    ComplexSquaringMapGenerator::new(ComplexSquaringMap::new(), x)
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
            ComplexSquaringMap::new().step(Point2::new(1.0, 2.0)),
            Point2::new(-3.0, 4.0),
        );
    }
}
