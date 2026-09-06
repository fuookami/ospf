//! Arnold 猫映射。
//! Arnold's cat map.

use super::helpers::{mod_one, one_point2};
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;
use std::marker::PhantomData;

/// Arnold 猫映射。
/// Arnold's cat map.
#[derive(Clone, Debug, PartialEq)]
pub struct ArnoldsCatMap<S: Field + Float = f64> {
    _marker: PhantomData<S>,
}

impl<S: Field + Float> ArnoldsCatMap<S> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    pub fn step(&self, x: Point2<S>) -> Point2<S> {
        Point2::new(
            mod_one((S::one() + S::one()) * x.x() + x.y()),
            mod_one(x.x() + x.y()),
        )
    }

    pub fn generator(self, initial: Point2<S>) -> ArnoldsCatMapGenerator<S> {
        ArnoldsCatMapGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for ArnoldsCatMap<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Arnold 猫映射生成器。
/// Arnold's cat map generator.
#[derive(Clone, Debug, PartialEq)]
pub struct ArnoldsCatMapGenerator<S: Field + Float = f64> {
    system: ArnoldsCatMap<S>,
    x: Point2<S>,
}

impl<S: Field + Float> ArnoldsCatMapGenerator<S> {
    pub fn new(system: ArnoldsCatMap<S>, x: Point2<S>) -> Self {
        Self { system, x }
    }

    pub fn system(&self) -> &ArnoldsCatMap<S> {
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

impl<S: Field + Float> Default for ArnoldsCatMapGenerator<S> {
    fn default() -> Self {
        Self::new(ArnoldsCatMap::default(), one_point2())
    }
}

impl<S: Field + Float> Iterator for ArnoldsCatMapGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建 Arnold 猫映射。
/// Create Arnold's cat map.
pub fn arnolds_cat_map<S: Field + Float>() -> ArnoldsCatMap<S> {
    ArnoldsCatMap::new()
}

/// 创建 Arnold 猫映射生成器。
/// Create Arnold's cat map generator.
pub fn arnolds_cat_map_generator<S: Field + Float>(x: Point2<S>) -> ArnoldsCatMapGenerator<S> {
    ArnoldsCatMapGenerator::new(ArnoldsCatMap::new(), x)
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
    fn arnolds_cat_map_matches_kotlin_formula() {
        assert_point2_close(
            ArnoldsCatMap::new().step(Point2::new(0.2, 0.3)),
            Point2::new(0.7, 0.5),
        );
    }
}
