//! Mandelbrot 集。
//! Mandelbrot set.

use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;

/// Mandelbrot 集迭代函数 `z -> z^2 + c`。
/// Mandelbrot set iteration function `z -> z^2 + c`.
#[derive(Clone, Debug, PartialEq)]
pub struct MandelbrotSet<S: Field + Float = f64> {
    c: Point2<S>,
}

impl<S: Field + Float> MandelbrotSet<S> {
    /// 使用二维点形式的复常数创建 Mandelbrot 集迭代器。
    /// Create a Mandelbrot set iterator from a point-shaped complex constant.
    pub fn new(c: Point2<S>) -> Self {
        Self { c }
    }

    /// 使用实部和虚部创建 Mandelbrot 集迭代器。
    /// Create a Mandelbrot set iterator from real and imaginary parts.
    pub fn from_parts(real: S, imag: S) -> Self {
        Self::new(Point2::new(real, imag))
    }

    /// 返回复常数 `c`。
    /// Return the complex constant `c`.
    pub fn c(&self) -> &Point2<S> {
        &self.c
    }

    /// 返回复常数的实部。
    /// Return the real part of the complex constant.
    pub fn real(&self) -> S {
        self.c.x()
    }

    /// 返回复常数的虚部。
    /// Return the imaginary part of the complex constant.
    pub fn imag(&self) -> S {
        self.c.y()
    }

    /// 执行一次 Mandelbrot 迭代。
    /// Execute one Mandelbrot iteration.
    pub fn iterate(&self, z: Point2<S>) -> Point2<S> {
        let real = z.x() * z.x() - z.y() * z.y() + self.c.x();
        let imag = (S::one() + S::one()) * z.x() * z.y() + self.c.y();
        Point2::new(real, imag)
    }

    /// 从指定初始值创建无限序列生成器。
    /// Create an infinite sequence generator from the given initial value.
    pub fn generator(self, z: Point2<S>) -> MandelbrotSetGenerator<S> {
        MandelbrotSetGenerator::new(self, z)
    }

    /// 从原点创建无限序列生成器。
    /// Create an infinite sequence generator from the origin.
    pub fn generator_from_origin(self) -> MandelbrotSetGenerator<S> {
        MandelbrotSetGenerator::new(self, Point2::origin())
    }
}

impl<S: Field + Float> Default for MandelbrotSet<S> {
    fn default() -> Self {
        Self::from_parts(S::one(), S::one())
    }
}

/// Mandelbrot 集序列生成器。
/// Mandelbrot set sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct MandelbrotSetGenerator<S: Field + Float = f64> {
    mandelbrot_set: MandelbrotSet<S>,
    z: Point2<S>,
}

impl<S: Field + Float> MandelbrotSetGenerator<S> {
    /// 使用迭代函数和初始值创建生成器。
    /// Create a generator from an iteration function and an initial value.
    pub fn new(mandelbrot_set: MandelbrotSet<S>, z: Point2<S>) -> Self {
        Self { mandelbrot_set, z }
    }

    /// 使用实部、虚部和初始值创建生成器。
    /// Create a generator from real and imaginary parts plus an initial value.
    pub fn from_parts(real: S, imag: S, z: Point2<S>) -> Self {
        Self::new(MandelbrotSet::from_parts(real, imag), z)
    }

    /// 返回当前迭代函数。
    /// Return the current iteration function.
    pub fn mandelbrot_set(&self) -> &MandelbrotSet<S> {
        &self.mandelbrot_set
    }

    /// 返回当前状态。
    /// Return the current state.
    pub fn z(&self) -> &Point2<S> {
        &self.z
    }

    /// 返回当前状态并推进一次迭代。
    /// Return the current state and advance one iteration.
    pub fn next_point(&mut self) -> Point2<S> {
        let z = self.z.clone();
        self.z = self.mandelbrot_set.iterate(self.z.clone());
        z
    }
}

impl<S: Field + Float> Default for MandelbrotSetGenerator<S> {
    fn default() -> Self {
        Self::new(MandelbrotSet::default(), Point2::origin())
    }
}

impl<S: Field + Float> Iterator for MandelbrotSetGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 使用二维点形式的复常数创建 Mandelbrot 集迭代器。
/// Create a Mandelbrot set iterator from a point-shaped complex constant.
pub fn mandelbrot_set<S: Field + Float>(c: Point2<S>) -> MandelbrotSet<S> {
    MandelbrotSet::new(c)
}

/// 使用实部和虚部创建 Mandelbrot 集迭代器。
/// Create a Mandelbrot set iterator from real and imaginary parts.
pub fn mandelbrot_set_from_parts<S: Field + Float>(real: S, imag: S) -> MandelbrotSet<S> {
    MandelbrotSet::from_parts(real, imag)
}

/// 使用实部、虚部和初始值创建 Mandelbrot 序列生成器。
/// Create a Mandelbrot sequence generator from real and imaginary parts plus an initial value.
pub fn mandelbrot_set_generator<S: Field + Float>(
    real: S,
    imag: S,
    z: Point2<S>,
) -> MandelbrotSetGenerator<S> {
    MandelbrotSetGenerator::from_parts(real, imag, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mandelbrot_set_iterates_quadratic_map() {
        let mandelbrot = MandelbrotSet::from_parts(-0.5_f64, 0.5);
        let z = Point2::new(1.0, 1.0);
        let next = mandelbrot.iterate(z);
        assert_eq!(next, Point2::new(-0.5, 2.5));
    }

    #[test]
    fn generator_returns_current_value_before_advancing() {
        let mut generator = MandelbrotSetGenerator::from_parts(-0.5_f64, 0.5, Point2::origin());
        assert_eq!(generator.next_point(), Point2::origin());
        assert_eq!(generator.z(), &Point2::new(-0.5, 0.5));
        assert_eq!(generator.next(), Some(Point2::new(-0.5, 0.5)));
    }

    #[test]
    fn default_generator_matches_kotlin_default_constant() {
        let mut generator = MandelbrotSetGenerator::<f64>::default();
        assert_eq!(generator.mandelbrot_set().c(), &Point2::new(1.0, 1.0));
        assert_eq!(generator.next(), Some(Point2::origin()));
        assert_eq!(generator.next(), Some(Point2::new(1.0, 1.0)));
    }
}
