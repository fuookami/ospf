//! 洛伦兹系统。
//! Lorenz system.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;

/// 洛伦兹系统的一阶欧拉步进模型。
/// First-order Euler step model for the Lorenz system.
#[derive(Clone, Debug, PartialEq)]
pub struct LorenzSystem<S: Field + Float = f64> {
    a: S,
    b: S,
    c: S,
    h: S,
}

impl<S: Field + Float> LorenzSystem<S> {
    /// 创建洛伦兹系统。
    /// Create a Lorenz system.
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

    /// 执行一次洛伦兹系统步进。
    /// Execute one Lorenz system step.
    pub fn step(&self, x: Point3<S>) -> Point3<S> {
        let dx = self.a * (x.y() - x.x());
        let dy = self.c * x.x() - x.x() * x.z() - x.y();
        let dz = x.x() * x.y() - self.b * x.z();
        Point3::new(
            x.x() + self.h * dx,
            x.y() + self.h * dy,
            x.z() + self.h * dz,
        )
    }

    /// 从指定初始值创建无限序列生成器。
    /// Create an infinite sequence generator from the given initial value.
    pub fn generator(self, initial: Point3<S>) -> LorenzSystemGenerator<S> {
        LorenzSystemGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for LorenzSystem<S> {
    fn default() -> Self {
        Self::new(
            S::from(10.0).expect("10.0 must be representable"),
            S::from(28.0).expect("28.0 must be representable"),
            S::from(8.0 / 3.0).expect("8.0 / 3.0 must be representable"),
            S::from(0.01).expect("0.01 must be representable"),
        )
    }
}

/// 洛伦兹系统序列生成器。
/// Lorenz system sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct LorenzSystemGenerator<S: Field + Float = f64> {
    lorenz_system: LorenzSystem<S>,
    x: Point3<S>,
}

impl<S: Field + Float> LorenzSystemGenerator<S> {
    /// 使用系统和初始值创建生成器。
    /// Create a generator from a system and an initial value.
    pub fn new(lorenz_system: LorenzSystem<S>, x: Point3<S>) -> Self {
        Self { lorenz_system, x }
    }

    /// 使用系统参数和初始值创建生成器。
    /// Create a generator from system parameters and an initial value.
    pub fn from_parts(a: S, b: S, c: S, h: S, x: Point3<S>) -> Self {
        Self::new(LorenzSystem::new(a, b, c, h), x)
    }

    /// 返回当前洛伦兹系统。
    /// Return the current Lorenz system.
    pub fn lorenz_system(&self) -> &LorenzSystem<S> {
        &self.lorenz_system
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
        self.x = self.lorenz_system.step(self.x.clone());
        x
    }
}

impl<S: Field + Float> Default for LorenzSystemGenerator<S> {
    fn default() -> Self {
        let one = S::one();
        Self::new(LorenzSystem::default(), Point3::new(one, one, one))
    }
}

impl<S: Field + Float> Iterator for LorenzSystemGenerator<S> {
    type Item = Point3<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建洛伦兹系统。
/// Create a Lorenz system.
pub fn lorenz_system<S: Field + Float>(a: S, b: S, c: S, h: S) -> LorenzSystem<S> {
    LorenzSystem::new(a, b, c, h)
}

/// 使用系统参数和初始值创建洛伦兹系统生成器。
/// Create a Lorenz system generator from system parameters and an initial value.
pub fn lorenz_system_generator<S: Field + Float>(
    a: S,
    b: S,
    c: S,
    h: S,
    x: Point3<S>,
) -> LorenzSystemGenerator<S> {
    LorenzSystemGenerator::from_parts(a, b, c, h, x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lorenz_step_matches_kotlin_formula() {
        let system = LorenzSystem::new(10.0_f64, 28.0, 8.0 / 3.0, 0.01);
        let next = system.step(Point3::new(1.0, 1.0, 1.0));
        assert_eq!(next, Point3::new(1.0, 1.0066666666666666, 0.73));
    }

    #[test]
    fn generator_returns_current_value_before_advancing() {
        let system = LorenzSystem::new(10.0_f64, 28.0, 8.0 / 3.0, 0.01);
        let mut generator = LorenzSystemGenerator::new(system, Point3::new(1.0, 1.0, 1.0));
        assert_eq!(generator.next_point(), Point3::new(1.0, 1.0, 1.0));
        assert_eq!(generator.x(), &Point3::new(1.0, 1.0066666666666666, 0.73));
        assert_eq!(
            generator.next(),
            Some(Point3::new(1.0, 1.0066666666666666, 0.73))
        );
    }

    #[test]
    fn default_parameters_match_kotlin_defaults() {
        let system = LorenzSystem::<f64>::default();
        assert_eq!(system.a(), 10.0);
        assert_eq!(system.b(), 28.0);
        assert_eq!(system.c(), 8.0 / 3.0);
        assert_eq!(system.h(), 0.01);
    }
}
