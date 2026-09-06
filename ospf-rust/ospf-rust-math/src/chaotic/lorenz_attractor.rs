//! Lorenz 吸引子（物理参数命名别名）。
//! Lorenz attractor (physics parameter naming alias).

use super::lorenz::LorenzSystem;
use crate::algebra::Field;
use crate::geometry::Point3;
use num_traits::Float;

/// Lorenz 吸引子，使用物理参数命名（sigma, rho, beta）。
/// Lorenz attractor with physics parameter naming (sigma, rho, beta).
///
/// 内部委托给 [`LorenzSystem`]，参数映射: sigma=a, beta=b, rho=c。
/// Delegates to [`LorenzSystem`] internally: sigma=a, beta=b, rho=c.
#[derive(Clone, Debug, PartialEq)]
pub struct LorenzAttractor<S: Field + Float = f64> {
    sigma: S,
    rho: S,
    beta: S,
    h: S,
    inner: LorenzSystem<S>,
}

impl<S: Field + Float> LorenzAttractor<S> {
    pub fn new(sigma: S, rho: S, beta: S, h: S) -> Self {
        Self {
            sigma,
            rho,
            beta,
            h,
            inner: LorenzSystem::new(sigma, beta, rho, h),
        }
    }

    pub fn sigma(&self) -> S {
        self.sigma
    }
    pub fn rho(&self) -> S {
        self.rho
    }
    pub fn beta(&self) -> S {
        self.beta
    }
    pub fn h(&self) -> S {
        self.h
    }

    pub fn step(&self, x: Point3<S>) -> Point3<S> {
        self.inner.step(x)
    }

    pub fn generator(self, initial: Point3<S>) -> LorenzAttractorGenerator<S> {
        LorenzAttractorGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for LorenzAttractor<S> {
    fn default() -> Self {
        Self::new(
            S::from(10.0).expect("10.0 must be representable"),
            S::from(28.0).expect("28.0 must be representable"),
            S::from(8.0 / 3.0).expect("8.0 / 3.0 must be representable"),
            S::from(0.01).expect("0.01 must be representable"),
        )
    }
}

/// Lorenz 吸引子序列生成器。
/// Lorenz attractor sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct LorenzAttractorGenerator<S: Field + Float = f64> {
    attractor: LorenzAttractor<S>,
    x: Point3<S>,
}

impl<S: Field + Float> LorenzAttractorGenerator<S> {
    pub fn new(attractor: LorenzAttractor<S>, x: Point3<S>) -> Self {
        Self { attractor, x }
    }

    pub fn attractor(&self) -> &LorenzAttractor<S> {
        &self.attractor
    }
    pub fn x(&self) -> &Point3<S> {
        &self.x
    }

    pub fn next_point(&mut self) -> Point3<S> {
        let x = self.x.clone();
        self.x = self.attractor.step(self.x.clone());
        x
    }
}

impl<S: Field + Float> Default for LorenzAttractorGenerator<S> {
    fn default() -> Self {
        let one = S::one();
        Self::new(LorenzAttractor::default(), Point3::new(one, one, one))
    }
}

impl<S: Field + Float> Iterator for LorenzAttractorGenerator<S> {
    type Item = Point3<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建 Lorenz 吸引子。
/// Create a Lorenz attractor.
pub fn lorenz_attractor<S: Field + Float>(sigma: S, rho: S, beta: S, h: S) -> LorenzAttractor<S> {
    LorenzAttractor::new(sigma, rho, beta, h)
}

/// 创建 Lorenz 吸引子生成器。
/// Create a Lorenz attractor generator.
pub fn lorenz_attractor_generator<S: Field + Float>(
    sigma: S,
    rho: S,
    beta: S,
    h: S,
    x: Point3<S>,
) -> LorenzAttractorGenerator<S> {
    LorenzAttractorGenerator::new(LorenzAttractor::new(sigma, rho, beta, h), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lorenz_attractor_matches_lorenz_system() {
        let attractor = LorenzAttractor::new(10.0_f64, 28.0, 8.0 / 3.0, 0.01);
        let system = LorenzSystem::new(10.0_f64, 8.0 / 3.0, 28.0, 0.01);
        let p = Point3::new(1.0, 1.0, 1.0);
        assert_eq!(attractor.step(p.clone()), system.step(p));
    }
}
