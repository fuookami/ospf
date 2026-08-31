//! Chen-Celikovsky 吸引子的一阶欧拉步进模型。
//! First-order Euler step model for the Chen-Celikovsky attractor.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;

/// Chen-Celikovsky 吸引子的一阶欧拉步进模型。
/// First-order Euler step model for the Chen-Celikovsky attractor.
#[derive(Clone, Debug, PartialEq)]
pub struct ChenCelikovskyAttractor<S: Field + Float = f64> {
    alpha: S,
    beta: S,
    delta: S,
    h: S,
}

impl<S: Field + Float> ChenCelikovskyAttractor<S> {
    /// 创建 Chen-Celikovsky 吸引子。
    /// Create a Chen-Celikovsky attractor.
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

    /// 执行一次 Chen-Celikovsky 吸引子步进。
    /// Execute one Chen-Celikovsky attractor step.
    pub fn step(&self, x: Point3<S>) -> Point3<S> {
        let dx = self.alpha * (x.y() - x.x());
        let dy = -x.x() * x.z() + self.delta * x.y();
        let dz = x.x() * x.y() - self.beta * x.z();
        Point3::new(
            x.x() + self.h * dx,
            x.y() + self.h * dy,
            x.z() + self.h * dz,
        )
    }

    /// 从指定初始值创建无限序列生成器。
    /// Create an infinite sequence generator from the given initial value.
    pub fn generator(self, initial: Point3<S>) -> ChenCelikovskyAttractorGenerator<S> {
        ChenCelikovskyAttractorGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for ChenCelikovskyAttractor<S> {
    fn default() -> Self {
        Self::new(
            S::from(36.0).expect("36.0 must be representable"),
            S::from(3.0).expect("3.0 must be representable"),
            S::from(20.0).expect("20.0 must be representable"),
            S::from(0.01).expect("0.01 must be representable"),
        )
    }
}

/// Chen-Celikovsky 吸引子序列生成器。
/// Chen-Celikovsky attractor sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct ChenCelikovskyAttractorGenerator<S: Field + Float = f64> {
    chen_celikovsky_attractor: ChenCelikovskyAttractor<S>,
    x: Point3<S>,
}

impl<S: Field + Float> ChenCelikovskyAttractorGenerator<S> {
    /// 使用吸引子和初始值创建生成器。
    /// Create a generator from an attractor and an initial value.
    pub fn new(chen_celikovsky_attractor: ChenCelikovskyAttractor<S>, x: Point3<S>) -> Self {
        Self {
            chen_celikovsky_attractor,
            x,
        }
    }

    /// 使用系统参数和初始值创建生成器。
    /// Create a generator from system parameters and an initial value.
    pub fn from_parts(alpha: S, beta: S, delta: S, h: S, x: Point3<S>) -> Self {
        Self::new(ChenCelikovskyAttractor::new(alpha, beta, delta, h), x)
    }

    /// 返回当前 Chen-Celikovsky 吸引子。
    /// Return the current Chen-Celikovsky attractor.
    pub fn chen_celikovsky_attractor(&self) -> &ChenCelikovskyAttractor<S> {
        &self.chen_celikovsky_attractor
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
        self.x = self.chen_celikovsky_attractor.step(self.x.clone());
        x
    }
}

impl<S: Field + Float> Default for ChenCelikovskyAttractorGenerator<S> {
    fn default() -> Self {
        let one = S::one();
        Self::new(
            ChenCelikovskyAttractor::default(),
            Point3::new(one, one, one),
        )
    }
}

impl<S: Field + Float> Iterator for ChenCelikovskyAttractorGenerator<S> {
    type Item = Point3<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_point())
    }
}

/// 创建 Chen-Celikovsky 吸引子。
/// Create a Chen-Celikovsky attractor.
pub fn chen_celikovsky_attractor<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    h: S,
) -> ChenCelikovskyAttractor<S> {
    ChenCelikovskyAttractor::new(alpha, beta, delta, h)
}

/// 使用系统参数和初始值创建 Chen-Celikovsky 吸引子生成器。
/// Create a Chen-Celikovsky attractor generator from system parameters and an initial value.
pub fn chen_celikovsky_attractor_generator<S: Field + Float>(
    alpha: S,
    beta: S,
    delta: S,
    h: S,
    x: Point3<S>,
) -> ChenCelikovskyAttractorGenerator<S> {
    ChenCelikovskyAttractorGenerator::from_parts(alpha, beta, delta, h, x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chen_celikovsky_step_matches_kotlin_formula() {
        let attractor = ChenCelikovskyAttractor::new(36.0_f64, 3.0, 20.0, 0.01);
        let next = attractor.step(Point3::new(1.0, 1.0, 1.0));

        assert_eq!(next, Point3::new(1.0, 1.19, 0.98));
    }

    #[test]
    fn chen_celikovsky_generator_returns_current_value_before_advancing() {
        let attractor = ChenCelikovskyAttractor::new(36.0_f64, 3.0, 20.0, 0.01);
        let mut generator =
            ChenCelikovskyAttractorGenerator::new(attractor, Point3::new(1.0, 1.0, 1.0));

        assert_eq!(generator.next_point(), Point3::new(1.0, 1.0, 1.0));
        assert_eq!(generator.x(), &Point3::new(1.0, 1.19, 0.98));
        assert_eq!(generator.next(), Some(Point3::new(1.0, 1.19, 0.98)));
    }

    #[test]
    fn chen_celikovsky_default_parameters_match_kotlin_defaults() {
        let attractor = ChenCelikovskyAttractor::<f64>::default();

        assert_eq!(attractor.alpha(), 36.0);
        assert_eq!(attractor.beta(), 3.0);
        assert_eq!(attractor.delta(), 20.0);
        assert_eq!(attractor.h(), 0.01);
    }
}