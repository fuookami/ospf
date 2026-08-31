//! 姜饼人映射。
//! Gingerbreadman map.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point2;

/// 姜饼人映射。
/// Gingerbreadman map.
#[derive(Clone, Debug, PartialEq)]
pub struct GingerbreadmanMap<S: Field + Float = f64> {
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Field + Float> GingerbreadmanMap<S> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }

    pub fn step(&self, x: Point2<S>) -> Point2<S> {
        let one = S::one();
        Point2::new(one - x.y() + x.x().abs(), x.x())
    }

    pub fn generator(self, initial: Point2<S>) -> GingerbreadmanMapGenerator<S> {
        GingerbreadmanMapGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for GingerbreadmanMap<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// 姜饼人映射序列生成器。
/// Gingerbreadman map sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct GingerbreadmanMapGenerator<S: Field + Float = f64> {
    map: GingerbreadmanMap<S>,
    x: Point2<S>,
}

impl<S: Field + Float> GingerbreadmanMapGenerator<S> {
    pub fn new(map: GingerbreadmanMap<S>, x: Point2<S>) -> Self {
        Self { map, x }
    }

    pub fn map(&self) -> &GingerbreadmanMap<S> { &self.map }
    pub fn x(&self) -> &Point2<S> { &self.x }

    pub fn next_point(&mut self) -> Point2<S> {
        let x = self.x.clone();
        self.x = self.map.step(self.x.clone());
        x
    }
}

impl<S: Field + Float> Default for GingerbreadmanMapGenerator<S> {
    fn default() -> Self {
        use super::helpers::one_point2;
        Self::new(GingerbreadmanMap::default(), one_point2())
    }
}

impl<S: Field + Float> Iterator for GingerbreadmanMapGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建姜饼人映射。
/// Create a Gingerbreadman map.
pub fn gingerbreadman_map<S: Field + Float>() -> GingerbreadmanMap<S> {
    GingerbreadmanMap::new()
}

/// 创建姜饼人映射生成器。
/// Create a Gingerbreadman map generator.
pub fn gingerbreadman_map_generator<S: Field + Float>(x: Point2<S>) -> GingerbreadmanMapGenerator<S> {
    GingerbreadmanMapGenerator::new(GingerbreadmanMap::new(), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gingerbreadman_step_formula() {
        let system = GingerbreadmanMap::<f64>::default();
        let next = system.step(Point2::new(1.0, 1.0));
        assert_eq!(next, Point2::new(1.0, 1.0));
    }
}
