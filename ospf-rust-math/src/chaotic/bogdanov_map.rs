//! Bogdanov 映射的一阶欧拉步进模型。
//! First-order Euler step model for the Bogdanov map.

use super::helpers::one_point2;
use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;

/// Bogdanov 映射的一阶欧拉步进模型。
/// First-order Euler step model for the Bogdanov map.
#[derive(Clone, Debug, PartialEq)]
pub struct BogdanovMap<S: Field + Float = f64> {
    epsilon: S,
    kappa: S,
    mu: S,
}

impl<S: Field + Float> BogdanovMap<S> {
    pub fn new(epsilon: S, kappa: S, mu: S) -> Self {
        Self { epsilon, kappa, mu }
    }

    pub fn epsilon(&self) -> S {
        self.epsilon
    }

    pub fn kappa(&self) -> S {
        self.kappa
    }

    pub fn mu(&self) -> S {
        self.mu
    }

    pub fn step(&self, state: Point2<S>) -> Point2<S> {
        let temp = state.y()
            + self.epsilon * state.y()
            + self.kappa * state.x() * (S::one() - state.x())
            + self.mu * state.x() * state.y();
        Point2::new(state.x() + temp, temp)
    }

    pub fn generator(self, initial: Point2<S>) -> BogdanovMapGenerator<S> {
        BogdanovMapGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for BogdanovMap<S> {
    fn default() -> Self {
        let half = S::one() / (S::one() + S::one());
        Self::new(half, half, half)
    }
}

/// Bogdanov 映射序列生成器。
/// Bogdanov map sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct BogdanovMapGenerator<S: Field + Float = f64> {
    system: BogdanovMap<S>,
    x: Point2<S>,
}

impl<S: Field + Float> BogdanovMapGenerator<S> {
    pub fn new(system: BogdanovMap<S>, x: Point2<S>) -> Self {
        Self { system, x }
    }

    pub fn system(&self) -> &BogdanovMap<S> {
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

impl<S: Field + Float> Default for BogdanovMapGenerator<S> {
    fn default() -> Self {
        Self::new(BogdanovMap::default(), one_point2())
    }
}

impl<S: Field + Float> Iterator for BogdanovMapGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建 Bogdanov 映射。
/// Create a Bogdanov map.
pub fn bogdanov_map<S: Field + Float>(epsilon: S, kappa: S, mu: S) -> BogdanovMap<S> {
    BogdanovMap::new(epsilon, kappa, mu)
}

/// 创建 Bogdanov 映射生成器。
/// Create a Bogdanov map generator.
pub fn bogdanov_map_generator<S: Field + Float>(
    epsilon: S,
    kappa: S,
    mu: S,
    x: Point2<S>,
) -> BogdanovMapGenerator<S> {
    BogdanovMapGenerator::new(BogdanovMap::new(epsilon, kappa, mu), x)
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
    fn bogdanov_map_matches_kotlin_formula() {
        assert_point2_close(
            BogdanovMap::default().step(Point2::new(0.2, 0.3)),
            Point2::new(0.76, 0.56),
        );
    }
}
