//! Baker 映射。
//! Baker's map.

use super::helpers::{mod_one, one_point2};
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;
use std::marker::PhantomData;

/// Baker 映射。
/// Baker's map.
#[derive(Clone, Debug, PartialEq)]
pub struct BakersMap<S: Field + Float = f64> {
    _marker: PhantomData<S>,
}

impl<S: Field + Float> BakersMap<S> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    pub fn step(&self, x: Point2<S>) -> Point2<S> {
        let two = S::one() + S::one();
        Point2::new(mod_one(two * x.x()), ((two * x.x()).floor() + x.y()) / two)
    }

    pub fn generator(self, initial: Point2<S>) -> BakersMapGenerator<S> {
        BakersMapGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for BakersMap<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Baker 映射生成器。
/// Baker's map generator.
#[derive(Clone, Debug, PartialEq)]
pub struct BakersMapGenerator<S: Field + Float = f64> {
    system: BakersMap<S>,
    x: Point2<S>,
}

impl<S: Field + Float> BakersMapGenerator<S> {
    pub fn new(system: BakersMap<S>, x: Point2<S>) -> Self {
        Self { system, x }
    }

    pub fn system(&self) -> &BakersMap<S> {
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

impl<S: Field + Float> Default for BakersMapGenerator<S> {
    fn default() -> Self {
        Self::new(BakersMap::default(), one_point2())
    }
}

impl<S: Field + Float> Iterator for BakersMapGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建 Baker 映射。
/// Create Baker's map.
pub fn bakers_map<S: Field + Float>() -> BakersMap<S> {
    BakersMap::new()
}

/// 创建 Baker 映射生成器。
/// Create Baker's map generator.
pub fn bakers_map_generator<S: Field + Float>(x: Point2<S>) -> BakersMapGenerator<S> {
    BakersMapGenerator::new(BakersMap::new(), x)
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
    fn bakers_map_matches_kotlin_formula() {
        assert_point2_close(
            BakersMap::new().step(Point2::new(0.6, 0.3)),
            Point2::new(0.19999999999999996, 0.65),
        );
    }

    #[test]
    fn generator_returns_current_value_before_advancing() {
        let mut map = BakersMapGenerator::new(BakersMap::new(), Point2::new(0.6, 0.3));
        assert_eq!(map.next_point(), Point2::new(0.6, 0.3));
        assert_point2_close(map.x().clone(), Point2::new(0.19999999999999996, 0.65));
    }
}
