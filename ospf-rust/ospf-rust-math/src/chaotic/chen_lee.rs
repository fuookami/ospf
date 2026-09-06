//! Chen-Lee 吸引子的一阶欧拉步进模型。
//! First-order Euler step model for the Chen-Lee attractor.

use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

/// Chen-Lee 吸引子的一阶欧拉步进模型。
/// First-order Euler step model for the Chen-Lee attractor.
#[derive(Clone, Debug, PartialEq)]
pub struct ChenLeeAttractor<S: Field + Float = f64> {
    alpha: S,
    beta: S,
    delta: S,
    h: S,
}

impl<S: Field + Float> ChenLeeAttractor<S> {
    /// 创建 Chen-Lee 吸引子。
    /// Create a Chen-Lee attractor.
    pub fn new(alpha: S, beta: S, delta: S, h: S) -> Self {
        Self {
            alpha,
            beta,
            delta,
            h,
        }
    }

    /// 返回系统参数 `alpha`。
    /// Return system parameter `alpha`.
    pub fn alpha(&self) -> S {
        self.alpha
    }

    /// 返回系统参数 `beta`。
    /// Return system parameter `beta`.
    pub fn beta(&self) -> S {
        self.beta
    }

    /// 返回系统参数 `delta`。
    /// Return system parameter `delta`.
    pub fn delta(&self) -> S {
        self.delta
    }

    /// 返回时间步长 `h`。
    /// Return time step `h`.
    pub fn h(&self) -> S {
        self.h
    }

    /// 执行一次 Chen-Lee 吸引子步进。
    /// Execute one Chen-Lee attractor step.
    pub fn step(&self, x: Point3<S>) -> Point3<S> {
        let three = S::one() + S::one() + S::one();
        let dx = self.alpha * x.x() - x.y() * x.z();
        let dy = self.beta * x.y() + x.x() * x.z();
        let dz = self.delta * x.z() + x.x() * x.y() / three;
        Point3::new(
            x.x() + self.h * dx,
            x.y() + self.h * dy,
            x.z() + self.h * dz,
        )
    }

    /// 从指定初始值创建无限序列生成器。
    /// Create an infinite sequence generator from the given initial value.
    pub fn generator(self, initial: Point3<S>) -> ChenLeeAttractorGenerator<S> {
        ChenLeeAttractorGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for ChenLeeAttractor<S> {
    fn default() -> Self {
        Self::new(
            S::from(5.0).expect("5.0 must be representable"),
            S::from(-10.0).expect("-10.0 must be representable"),
            S::from(0.38).expect("0.38 must be representable"),
            S::from(0.01).expect("0.01 must be representable"),
        )
    }
}

/// Chen-Lee 吸引子序列生成器。
/// Chen-Lee attractor sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct ChenLeeAttractorGenerator<S: Field + Float = f64> {
    chen_lee_attractor: ChenLeeAttractor<S>,
    x: Point3<S>,
}

impl<S: Field + Float> ChenLeeAttractorGenerator<S> {
    /// 使用吸引子和初始值创建生成器。
    /// Create a generator from an attractor and an initial value.
    pub fn new(chen_lee_attractor: ChenLeeAttractor<S>, x: Point3<S>) -> Self {
        Self {
            chen_lee_attractor,
            x,
        }
    }

    /// 使用系统参数和初始值创建生成器。
    /// Create a generator from system parameters and an initial value.
    pub fn from_parts(alpha: S, beta: S, delta: S, h: S, x: Point3<S>) -> Self {
        Self::new(ChenLeeAttractor::new(alpha, beta, delta, h), x)
    }

    /// 返回当前 Chen-Lee 吸引子。
    /// Return the current Chen-Lee attractor.
    pub fn chen_lee_attractor(&self) -> &ChenLeeAttractor<S> {
        &self.chen_lee_attractor
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
        self.x = self.chen_lee_attractor.step(self.x.clone());
        x
    }
}

impl<S: Field + Float> Default for ChenLeeAttractorGenerator<S> {
    fn default() -> Self {
        let one = S::one();
        Self::new(ChenLeeAttractor::default(), Point3::new(one, one, one))
    }
}

impl<S: Field + Float> Iterator for ChenLeeAttractorGenerator<S> {
    type Item = Point3<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建 Chen-Lee 吸引子。
/// Create a Chen-Lee attractor.
pub fn chen_lee_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    h: S,
) -> ChenLeeAttractor<S> {
    ChenLeeAttractor::new(alpha, beta, delta, h)
}

/// 使用系统参数和初始值创建 Chen-Lee 吸引子生成器。
/// Create a Chen-Lee attractor generator from system parameters and an initial value.
pub fn chen_lee_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    h: S,
    x: Point3<S>,
) -> ChenLeeAttractorGenerator<S> {
    ChenLeeAttractorGenerator::from_parts(alpha, beta, delta, h, x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chen_lee_step_matches_kotlin_formula() {
        let attractor = ChenLeeAttractor::new(5.0_f64, -10.0, 0.38, 0.01);
        let next = attractor.step(Point3::new(1.0, 1.0, 1.0));

        assert_eq!(next, Point3::new(1.04, 0.91, 1.0071333333333334));
    }

    #[test]
    fn chen_lee_generator_returns_current_value_before_advancing() {
        let attractor = ChenLeeAttractor::new(5.0_f64, -10.0, 0.38, 0.01);
        let mut generator = ChenLeeAttractorGenerator::new(attractor, Point3::new(1.0, 1.0, 1.0));

        assert_eq!(generator.next_point(), Point3::new(1.0, 1.0, 1.0));
        assert_eq!(generator.x(), &Point3::new(1.04, 0.91, 1.0071333333333334));
        assert_eq!(
            generator.next(),
            Some(Point3::new(1.04, 0.91, 1.0071333333333334))
        );
    }

    #[test]
    fn chen_lee_default_parameters_match_kotlin_defaults() {
        let attractor = ChenLeeAttractor::<f64>::default();

        assert_eq!(attractor.alpha(), 5.0);
        assert_eq!(attractor.beta(), -10.0);
        assert_eq!(attractor.delta(), 0.38);
        assert_eq!(attractor.h(), 0.01);
    }
}
