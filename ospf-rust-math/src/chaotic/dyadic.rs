//! 二进制变换。
//! Dyadic transformation.

use num_traits::Float;
use crate::algebra::Field;

/// 二进制变换。
/// Dyadic transformation.
///
/// 公式: x_{n+1} = 2*x mod 1
#[derive(Clone, Debug, PartialEq)]
pub struct DyadicTransformation<S: Field + Float = f64> {
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Field + Float> DyadicTransformation<S> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }

    pub fn step(&self, x: S) -> S {
        let one = S::one();
        let two = one + one;
        (two * x) - (two * x).floor()
    }

    pub fn generator(self, initial: S) -> DyadicTransformationGenerator<S> {
        DyadicTransformationGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for DyadicTransformation<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// 二进制变换序列生成器。
/// Dyadic transformation sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct DyadicTransformationGenerator<S: Field + Float = f64> {
    map: DyadicTransformation<S>,
    x: S,
}

impl<S: Field + Float> DyadicTransformationGenerator<S> {
    pub fn new(map: DyadicTransformation<S>, x: S) -> Self {
        Self { map, x }
    }

    pub fn map(&self) -> &DyadicTransformation<S> { &self.map }
    pub fn x(&self) -> S { self.x }

    pub fn next_value(&mut self) -> S {
        let x = self.x;
        self.x = self.map.step(self.x);
        x
    }
}

impl<S: Field + Float> Default for DyadicTransformationGenerator<S> {
    fn default() -> Self {
        Self::new(DyadicTransformation::default(), S::from(0.5).expect("0.5 must be representable"))
    }
}

impl<S: Field + Float> Iterator for DyadicTransformationGenerator<S> {
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_value())
    }
}

/// 创建二进制变换。
/// Create a dyadic transformation.
pub fn dyadic_transformation<S: Field + Float>() -> DyadicTransformation<S> {
    DyadicTransformation::new()
}

/// 创建二进制变换生成器。
/// Create a dyadic transformation generator.
pub fn dyadic_transformation_generator<S: Field + Float>(x: S) -> DyadicTransformationGenerator<S> {
    DyadicTransformationGenerator::new(DyadicTransformation::new(), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyadic_step_formula() {
        let system = DyadicTransformation::<f64>::default();
        assert!((system.step(0.3) - 0.6).abs() < 1e-12);
        assert!((system.step(0.7) - 0.4).abs() < 1e-12);
    }
}
