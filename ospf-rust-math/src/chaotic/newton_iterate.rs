//! Newton 迭代。
//! Newton iterate.
//!
//! 求解 z^3 = 1 的 Newton 法迭代。
//! Newton's method iteration for solving z^3 = 1.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point2;

/// Newton 迭代。
/// Newton iterate.
///
/// 公式: z_{n+1} = 2/3 * z + 1/(3*z^2)
/// 在复平面上用二维点表示。
#[derive(Clone, Debug, PartialEq)]
pub struct NewtonIterate<S: Field + Float = f64> {
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Field + Float> NewtonIterate<S> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }

    /// 执行一次 Newton 迭代。
    /// Execute one Newton iteration.
    pub fn step(&self, p: Point2<S>) -> Point2<S> {
        let zero = S::zero();
        let two = S::one() + S::one();
        let three = two + S::one();
        let four = two + two;
        let two_thirds = two / three;

        let x2 = p.x() * p.x();
        let y2 = p.y() * p.y();
        let d = three * ((x2 - y2) * (x2 - y2) + four * x2 * y2);

        if d == zero {
            return Point2::new(zero, zero);
        }

        Point2::new(
            two_thirds * p.x() + (x2 - y2) / d,
            two_thirds * p.y() - (two * p.x() * p.y()) / d,
        )
    }

    pub fn generator(self, initial: Point2<S>) -> NewtonIterateGenerator<S> {
        NewtonIterateGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for NewtonIterate<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Newton 迭代序列生成器。
/// Newton iterate sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct NewtonIterateGenerator<S: Field + Float = f64> {
    map: NewtonIterate<S>,
    x: Point2<S>,
}

impl<S: Field + Float> NewtonIterateGenerator<S> {
    pub fn new(map: NewtonIterate<S>, x: Point2<S>) -> Self {
        Self { map, x }
    }

    pub fn map(&self) -> &NewtonIterate<S> { &self.map }
    pub fn x(&self) -> &Point2<S> { &self.x }

    pub fn next_point(&mut self) -> Point2<S> {
        let x = self.x.clone();
        self.x = self.map.step(self.x.clone());
        x
    }
}

impl<S: Field + Float> Default for NewtonIterateGenerator<S> {
    fn default() -> Self {
        use super::helpers::one_point2;
        Self::new(NewtonIterate::default(), one_point2())
    }
}

impl<S: Field + Float> Iterator for NewtonIterateGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建 Newton 迭代。
/// Create a Newton iterate.
pub fn newton_iterate<S: Field + Float>() -> NewtonIterate<S> {
    NewtonIterate::new()
}

/// 创建 Newton 迭代生成器。
/// Create a Newton iterate generator.
pub fn newton_iterate_generator<S: Field + Float>(x: Point2<S>) -> NewtonIterateGenerator<S> {
    NewtonIterateGenerator::new(NewtonIterate::new(), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newton_iterate_zero_d() {
        let system = NewtonIterate::<f64>::default();
        let next = system.step(Point2::new(0.0, 0.0));
        assert_eq!(next, Point2::new(0.0, 0.0));
    }

    #[test]
    fn newton_iterate_formula() {
        let system = NewtonIterate::<f64>::default();
        let next = system.step(Point2::new(1.0, 0.0));
        // x^2=1, y^2=0, d=3*1=3
        // new_x = 2/3 + 1/3 = 1.0
        // new_y = 0
        assert!((next.x() - 1.0).abs() < 1e-12);
        assert!((next.y() - 0.0).abs() < 1e-12);
    }

    #[test]
    fn newton_iterate_negative_x() {
        let system = NewtonIterate::<f64>::default();
        let next = system.step(Point2::new(-1.0, 0.0));
        // x^2=1, y^2=0, d=3*1=3
        // new_x = -2/3 + 1/3 = -1/3
        // new_y = 0
        assert!((next.x() - (-1.0 / 3.0)).abs() < 1e-12);
        assert!((next.y() - 0.0).abs() < 1e-12);
    }
}
