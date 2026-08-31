//! Julia 集。
//! Julia set.

use crate::algebra::Field;
use crate::geometry::Point2;
use num_traits::Float;

/// Julia 集迭代函数 `z -> z^2 + c`。
/// Julia set iteration function `z -> z^2 + c`.
#[derive(Clone, Debug, PartialEq)]
pub struct JuliaSet<S: Field + Float = f64> {
    c: Point2<S>,
}

impl<S: Field + Float> JuliaSet<S> {
    /// 使用二维点形式的复常数创建 Julia 集迭代器。
    /// Create a Julia set iterator from a point-shaped complex constant.
    pub fn new(c: Point2<S>) -> Self {
        Self { c }
    }

    /// 使用实部和虚部创建 Julia 集迭代器。
    /// Create a Julia set iterator from real and imaginary parts.
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

    /// 执行一次 Julia 集迭代。
    /// Execute one Julia set iteration.
    pub fn iterate(&self, z: Point2<S>) -> Point2<S> {
        let two = S::one() + S::one();
        let real = z.x() * z.x() - z.y() * z.y() + self.c.x();
        let imag = two * z.x() * z.y() + self.c.y();
        Point2::new(real, imag)
    }

    /// 从指定初始值创建无限序列生成器。
    /// Create an infinite sequence generator from the given initial value.
    pub fn generator(self, z: Point2<S>) -> JuliaSetGenerator<S> {
        JuliaSetGenerator::new(self, z)
    }
}

impl<S: Field + Float> Default for JuliaSet<S> {
    fn default() -> Self {
        Self::from_parts(
            S::from(-0.7).expect("-0.7 must be representable"),
            S::from(0.27015).expect("0.27015 must be representable"),
        )
    }
}

/// Julia 集序列生成器。
/// Julia set sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct JuliaSetGenerator<S: Field + Float = f64> {
    julia_set: JuliaSet<S>,
    z: Point2<S>,
}

impl<S: Field + Float> JuliaSetGenerator<S> {
    /// 使用迭代函数和初始值创建生成器。
    /// Create a generator from an iteration function and an initial value.
    pub fn new(julia_set: JuliaSet<S>, z: Point2<S>) -> Self {
        Self { julia_set, z }
    }

    /// 使用实部、虚部和初始值创建生成器。
    /// Create a generator from real and imaginary parts plus an initial value.
    pub fn from_parts(real: S, imag: S, z: Point2<S>) -> Self {
        Self::new(JuliaSet::from_parts(real, imag), z)
    }

    /// 返回当前迭代函数。
    /// Return the current iteration function.
    pub fn julia_set(&self) -> &JuliaSet<S> {
        &self.julia_set
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
        self.z = self.julia_set.iterate(self.z.clone());
        z
    }
}

impl<S: Field + Float> Default for JuliaSetGenerator<S> {
    fn default() -> Self {
        Self::new(JuliaSet::default(), Point2::origin())
    }
}

impl<S: Field + Float> Iterator for JuliaSetGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 使用二维点形式的复常数创建 Julia 集迭代器。
/// Create a Julia set iterator from a point-shaped complex constant.
pub fn julia_set<S: Field + Float>(c: Point2<S>) -> JuliaSet<S> {
    JuliaSet::new(c)
}

/// 使用实部和虚部创建 Julia 集迭代器。
/// Create a Julia set iterator from real and imaginary parts.
pub fn julia_set_from_parts<S: Field + Float>(real: S, imag: S) -> JuliaSet<S> {
    JuliaSet::from_parts(real, imag)
}

/// 使用实部、虚部和初始值创建 Julia 集序列生成器。
/// Create a Julia set sequence generator from real and imaginary parts plus an initial value.
pub fn julia_set_generator<S: Field + Float>(
    real: S,
    imag: S,
    z: Point2<S>,
) -> JuliaSetGenerator<S> {
    JuliaSetGenerator::from_parts(real, imag, z)
}

/// 多重 Julia 集迭代函数 `z -> z^n + c`。
/// Multi Julia set iteration function `z -> z^n + c`.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiJuliaSet<S: Field + Float = f64> {
    c: Point2<S>,
    n: S,
}

impl<S: Field + Float> MultiJuliaSet<S> {
    pub fn new(c: Point2<S>, n: S) -> Self {
        Self { c, n }
    }

    pub fn from_parts(real: S, imag: S, n: S) -> Self {
        Self::new(Point2::new(real, imag), n)
    }

    pub fn c(&self) -> &Point2<S> {
        &self.c
    }
    pub fn n(&self) -> S {
        self.n
    }

    /// 执行一次多重 Julia 集迭代。
    /// Execute one multi Julia set iteration.
    pub fn iterate(&self, z: Point2<S>) -> Point2<S> {
        let two = S::one() + S::one();
        let half_n = self.n / two;
        let r2 = z.x() * z.x() + z.y() * z.y();
        let r_n = r2.powf(half_n);
        let theta = z.y().atan2(z.x());
        let n_theta = self.n * theta;
        Point2::new(
            r_n * n_theta.cos() + self.c.x(),
            r_n * n_theta.sin() + self.c.y(),
        )
    }

    pub fn generator(self, z: Point2<S>) -> MultiJuliaSetGenerator<S> {
        MultiJuliaSetGenerator::new(self, z)
    }
}

impl<S: Field + Float> Default for MultiJuliaSet<S> {
    fn default() -> Self {
        Self::from_parts(
            S::from(-0.7).expect("-0.7 must be representable"),
            S::from(0.27015).expect("0.27015 must be representable"),
            S::from(2.0).expect("2.0 must be representable"),
        )
    }
}

/// 多重 Julia 集序列生成器。
/// Multi Julia set sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiJuliaSetGenerator<S: Field + Float = f64> {
    multi_julia_set: MultiJuliaSet<S>,
    z: Point2<S>,
}

impl<S: Field + Float> MultiJuliaSetGenerator<S> {
    pub fn new(multi_julia_set: MultiJuliaSet<S>, z: Point2<S>) -> Self {
        Self { multi_julia_set, z }
    }

    pub fn multi_julia_set(&self) -> &MultiJuliaSet<S> {
        &self.multi_julia_set
    }
    pub fn z(&self) -> &Point2<S> {
        &self.z
    }

    pub fn next_point(&mut self) -> Point2<S> {
        let z = self.z.clone();
        self.z = self.multi_julia_set.iterate(self.z.clone());
        z
    }
}

impl<S: Field + Float> Default for MultiJuliaSetGenerator<S> {
    fn default() -> Self {
        Self::new(MultiJuliaSet::default(), Point2::origin())
    }
}

impl<S: Field + Float> Iterator for MultiJuliaSetGenerator<S> {
    type Item = Point2<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn julia_set_iterates_quadratic_map() {
        let julia = JuliaSet::from_parts(-0.5_f64, 0.5);
        let z = Point2::new(1.0, 1.0);
        let next = julia.iterate(z);
        assert_eq!(next, Point2::new(-0.5, 2.5));
    }

    #[test]
    fn julia_generator_returns_current_value_before_advancing() {
        let mut generator = JuliaSetGenerator::from_parts(-0.5_f64, 0.5, Point2::origin());
        assert_eq!(generator.next_point(), Point2::origin());
        assert_eq!(generator.z(), &Point2::new(-0.5, 0.5));
    }

    #[test]
    fn multi_julia_set_default_matches_julia() {
        let julia = JuliaSet::default();
        let multi = MultiJuliaSet::default();
        let z = Point2::new(1.0, 1.0);
        let j = julia.iterate(z.clone());
        let m = multi.iterate(z);
        // MultiJuliaSet uses polar coordinates, so results differ slightly
        assert!((j.x() - m.x()).abs() < 1e-10);
        assert!((j.y() - m.y()).abs() < 1e-10);
    }
}
