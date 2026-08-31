//! Coullet 吸引子的一阶欧拉步进模型。
//! First-order Euler step model for the Coullet attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;

/// Coullet 吸引子的一阶欧拉步进模型。
/// First-order Euler step model for the Coullet attractor.
#[derive(Clone, Debug, PartialEq)]
pub struct CoulletAttractor<S: Field + Float = f64> {
    alpha: S,
    beta: S,
    delta: S,
    zeta: S,
    h: S,
}

impl<S: Field + Float> CoulletAttractor<S> {
    /// 创建 Coullet 吸引子。
    /// Create a Coullet attractor.
    pub fn new(alpha: S, beta: S, delta: S, zeta: S, h: S) -> Self {
        Self {
            alpha,
            beta,
            delta,
            zeta,
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

    /// 返回系统参数 `zeta`。
    /// Return system parameter `zeta`.
    pub fn zeta(&self) -> S {
        self.zeta
    }

    /// 返回时间步长 `h`。
    /// Return time step `h`.
    pub fn h(&self) -> S {
        self.h
    }

    /// 执行一次 Coullet 吸引子步进。
    /// Execute one Coullet attractor step.
    pub fn step(&self, x: Point3<S>) -> Point3<S> {
        let dx = x.y();
        let dy = x.z();
        let dz = self.alpha * x.x()
            + self.beta * x.y()
            + self.delta * x.z()
            + self.delta * x.x().powi(3);
        Point3::new(
            x.x() + self.h * dx,
            x.y() + self.h * dy,
            x.z() + self.h * dz,
        )
    }

    /// 从指定初始值创建无限序列生成器。
    /// Create an infinite sequence generator from the given initial value.
    pub fn generator(self, initial: Point3<S>) -> CoulletAttractorGenerator<S> {
        CoulletAttractorGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for CoulletAttractor<S> {
    fn default() -> Self {
        Self::new(
            S::from(0.8).expect("0.8 must be representable"),
            S::from(-1.1).expect("-1.1 must be representable"),
            S::from(-1.0).expect("-1.0 must be representable"),
            S::from(-0.45).expect("-0.45 must be representable"),
            S::from(0.01).expect("0.01 must be representable"),
        )
    }
}

/// Coullet 吸引子序列生成器。
/// Coullet attractor sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct CoulletAttractorGenerator<S: Field + Float = f64> {
    coullet_attractor: CoulletAttractor<S>,
    x: Point3<S>,
}

impl<S: Field + Float> CoulletAttractorGenerator<S> {
    /// 使用吸引子和初始值创建生成器。
    /// Create a generator from an attractor and an initial value.
    pub fn new(coullet_attractor: CoulletAttractor<S>, x: Point3<S>) -> Self {
        Self {
            coullet_attractor,
            x,
        }
    }

    /// 使用系统参数和初始值创建生成器。
    /// Create a generator from system parameters and an initial value.
    pub fn from_parts(alpha: S, beta: S, delta: S, zeta: S, h: S, x: Point3<S>) -> Self {
        Self::new(CoulletAttractor::new(alpha, beta, delta, zeta, h), x)
    }

    /// 返回当前 Coullet 吸引子。
    /// Return the current Coullet attractor.
    pub fn coullet_attractor(&self) -> &CoulletAttractor<S> {
        &self.coullet_attractor
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
        self.x = self.coullet_attractor.step(self.x.clone());
        x
    }
}

impl<S: Field + Float> Default for CoulletAttractorGenerator<S> {
    fn default() -> Self {
        let one = S::one();
        Self::new(CoulletAttractor::default(), Point3::new(one, one, one))
    }
}

impl<S: Field + Float> Iterator for CoulletAttractorGenerator<S> {
    type Item = Point3<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建 Coullet 吸引子。
/// Create a Coullet attractor.
pub fn coullet_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    zeta: S,
    h: S,
) -> CoulletAttractor<S> {
    CoulletAttractor::new(alpha, beta, delta, zeta, h)
}

/// 使用系统参数和初始值创建 Coullet 吸引子生成器。
/// Create a Coullet attractor generator from system parameters and an initial value.
pub fn coullet_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    zeta: S,
    h: S,
    x: Point3<S>,
) -> CoulletAttractorGenerator<S> {
    CoulletAttractorGenerator::from_parts(alpha, beta, delta, zeta, h, x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coullet_step_matches_kotlin_formula() {
        let attractor = CoulletAttractor::new(0.8_f64, -1.1, -1.0, -0.45, 0.01);
        let next = attractor.step(Point3::new(1.0, 1.0, 1.0));

        assert_eq!(next, Point3::new(1.01, 1.01, 0.977));
    }

    #[test]
    fn coullet_generator_returns_current_value_before_advancing() {
        let attractor = CoulletAttractor::new(0.8_f64, -1.1, -1.0, -0.45, 0.01);
        let mut generator = CoulletAttractorGenerator::new(attractor, Point3::new(1.0, 1.0, 1.0));

        assert_eq!(generator.next_point(), Point3::new(1.0, 1.0, 1.0));
        assert_eq!(generator.x(), &Point3::new(1.01, 1.01, 0.977));
        assert_eq!(generator.next(), Some(Point3::new(1.01, 1.01, 0.977)));
    }

    #[test]
    fn coullet_default_parameters_match_kotlin_defaults() {
        let attractor = CoulletAttractor::<f64>::default();

        assert_eq!(attractor.alpha(), 0.8);
        assert_eq!(attractor.beta(), -1.1);
        assert_eq!(attractor.delta(), -1.0);
        assert_eq!(attractor.zeta(), -0.45);
        assert_eq!(attractor.h(), 0.01);
    }
}