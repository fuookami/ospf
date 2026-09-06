//! 陈氏系统的一阶欧拉步进模型。
//! First-order Euler step model for the Chen system.

use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

/// 陈氏系统的一阶欧拉步进模型。
/// First-order Euler step model for the Chen system.
#[derive(Clone, Debug, PartialEq)]
pub struct ChenSystem<S: Field + Float = f64> {
    a: S,
    b: S,
    c: S,
    h: S,
}

impl<S: Field + Float> ChenSystem<S> {
    /// 创建陈氏系统。
    /// Create a Chen system.
    pub fn new(a: S, b: S, c: S, h: S) -> Self {
        Self { a, b, c, h }
    }

    /// 返回系统参数 `a`。
    /// Return system parameter `a`.
    pub fn a(&self) -> S {
        self.a
    }

    /// 返回系统参数 `b`。
    /// Return system parameter `b`.
    pub fn b(&self) -> S {
        self.b
    }

    /// 返回系统参数 `c`。
    /// Return system parameter `c`.
    pub fn c(&self) -> S {
        self.c
    }

    /// 返回时间步长 `h`。
    /// Return time step `h`.
    pub fn h(&self) -> S {
        self.h
    }

    /// 执行一次陈氏系统步进。
    /// Execute one Chen system step.
    pub fn step(&self, x: Point3<S>) -> Point3<S> {
        let dx = self.a * (x.y() - x.x());
        let dy = (self.c - self.a) * x.x() - x.x() * x.z() + self.c * x.y();
        let dz = x.x() * x.y() - self.b * x.z();
        Point3::new(
            x.x() + self.h * dx,
            x.y() + self.h * dy,
            x.z() + self.h * dz,
        )
    }

    /// 从指定初始值创建无限序列生成器。
    /// Create an infinite sequence generator from the given initial value.
    pub fn generator(self, initial: Point3<S>) -> ChenSystemGenerator<S> {
        ChenSystemGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for ChenSystem<S> {
    fn default() -> Self {
        Self::new(
            S::from(10.0).expect("10.0 must be representable"),
            S::from(8.0 / 3.0).expect("8.0 / 3.0 must be representable"),
            S::from(137.0 / 5.0).expect("137.0 / 5.0 must be representable"),
            S::from(0.01).expect("0.01 must be representable"),
        )
    }
}

/// 陈氏系统序列生成器。
/// Chen system sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct ChenSystemGenerator<S: Field + Float = f64> {
    chen_system: ChenSystem<S>,
    x: Point3<S>,
}

impl<S: Field + Float> ChenSystemGenerator<S> {
    /// 使用系统和初始值创建生成器。
    /// Create a generator from a system and an initial value.
    pub fn new(chen_system: ChenSystem<S>, x: Point3<S>) -> Self {
        Self { chen_system, x }
    }

    /// 使用系统参数和初始值创建生成器。
    /// Create a generator from system parameters and an initial value.
    pub fn from_parts(a: S, b: S, c: S, h: S, x: Point3<S>) -> Self {
        Self::new(ChenSystem::new(a, b, c, h), x)
    }

    /// 返回当前陈氏系统。
    /// Return the current Chen system.
    pub fn chen_system(&self) -> &ChenSystem<S> {
        &self.chen_system
    }

    /// 返回当前状态。
    /// Return the current state.
    pub fn x(&self) -> &Point3<S> {
        &self.x
    }

    /// 返回当前状态并推进一次迭代。
    /// Return the current state and advance one iteration.
    pub fn next_point(&mut self) -> Point3<S> {
        let x = self.x.clone();
        self.x = self.chen_system.step(self.x.clone());
        x
    }
}

impl<S: Field + Float> Default for ChenSystemGenerator<S> {
    fn default() -> Self {
        let one = S::one();
        Self::new(ChenSystem::default(), Point3::new(one, one, one))
    }
}

impl<S: Field + Float> Iterator for ChenSystemGenerator<S> {
    type Item = Point3<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建陈氏系统。
/// Create a Chen system.
pub fn chen_system<S: Field + Float>(a: S, b: S, c: S, h: S) -> ChenSystem<S> {
    ChenSystem::new(a, b, c, h)
}

/// 使用系统参数和初始值创建陈氏系统生成器。
/// Create a Chen system generator from system parameters and an initial value.
pub fn chen_system_generator<S: Field + Float>(
    a: S,
    b: S,
    c: S,
    h: S,
    x: Point3<S>,
) -> ChenSystemGenerator<S> {
    ChenSystemGenerator::from_parts(a, b, c, h, x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chen_step_matches_kotlin_formula() {
        let system = ChenSystem::new(10.0_f64, 8.0 / 3.0, 137.0 / 5.0, 0.01);
        let next = system.step(Point3::new(1.0, 1.0, 1.0));

        assert_eq!(next, Point3::new(1.0, 1.438, 0.9833333333333333));
    }

    #[test]
    fn chen_generator_returns_current_value_before_advancing() {
        let system = ChenSystem::new(10.0_f64, 8.0 / 3.0, 137.0 / 5.0, 0.01);
        let mut generator = ChenSystemGenerator::new(system, Point3::new(1.0, 1.0, 1.0));

        assert_eq!(generator.next_point(), Point3::new(1.0, 1.0, 1.0));
        assert_eq!(generator.x(), &Point3::new(1.0, 1.438, 0.9833333333333333));
        assert_eq!(
            generator.next(),
            Some(Point3::new(1.0, 1.438, 0.9833333333333333))
        );
    }

    #[test]
    fn chen_default_parameters_match_kotlin_defaults() {
        let system = ChenSystem::<f64>::default();

        assert_eq!(system.a(), 10.0);
        assert_eq!(system.b(), 8.0 / 3.0);
        assert_eq!(system.c(), 137.0 / 5.0);
        assert_eq!(system.h(), 0.01);
    }
}
