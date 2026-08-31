//! 区间交换变换。
//! Interval exchange transformation.

use num_traits::Float;
use crate::algebra::Field;
/// 区间交换变换。
/// Interval exchange transformation.
#[derive(Clone, Debug, PartialEq)]
pub struct IntervalExchangeTransformation<S: Field + Float = f64> {
    lambda: Vec<S>,
    pi: Vec<usize>,
}

impl<S: Field + Float> IntervalExchangeTransformation<S> {
    pub fn new(lambda: Vec<S>, pi: Vec<usize>) -> Self {
        assert_eq!(lambda.len(), pi.len(), "lambda and pi must have the same size");
        Self { lambda, pi }
    }

    pub fn lambda(&self) -> &[S] { &self.lambda }
    pub fn pi(&self) -> &[usize] { &self.pi }

    /// 执行一次区间交换变换。
    /// Execute one interval exchange transformation.
    pub fn step(&self, x: S) -> S {
        let n = self.lambda.len();
        let zero = S::zero();
        let mut sum = zero;
        let mut interval_index = 0;
        for i in 0..n {
            if x < sum + self.lambda[i] {
                interval_index = i;
                break;
            }
            sum = sum + self.lambda[i];
            if i == n - 1 {
                interval_index = i;
            }
        }
        let relative_pos = (x - sum) / self.lambda[interval_index];
        let target_interval = self.pi[interval_index];
        let mut target_start = zero;
        for i in 0..target_interval {
            target_start = target_start + self.lambda[i];
        }
        target_start + relative_pos * self.lambda[target_interval]
    }

    pub fn generator(self, initial: S) -> IntervalExchangeTransformationGenerator<S> {
        IntervalExchangeTransformationGenerator::new(self, initial)
    }
}

/// 区间交换变换序列生成器。
/// Interval exchange transformation sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct IntervalExchangeTransformationGenerator<S: Field + Float = f64> {
    map: IntervalExchangeTransformation<S>,
    x: S,
}

impl<S: Field + Float> IntervalExchangeTransformationGenerator<S> {
    pub fn new(map: IntervalExchangeTransformation<S>, x: S) -> Self {
        Self { map, x }
    }

    pub fn map(&self) -> &IntervalExchangeTransformation<S> { &self.map }
    pub fn x(&self) -> S { self.x }

    pub fn next_value(&mut self) -> S {
        let x = self.x;
        self.x = self.map.step(self.x);
        x
    }
}

impl<S: Field + Float> Iterator for IntervalExchangeTransformationGenerator<S> {
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_value())
    }
}

/// 创建区间交换变换。
/// Create an interval exchange transformation.
pub fn interval_exchange_transformation<S: Field + Float>(
    lambda: Vec<S>, pi: Vec<usize>,
) -> IntervalExchangeTransformation<S> {
    IntervalExchangeTransformation::new(lambda, pi)
}

/// 创建区间交换变换生成器。
/// Create an interval exchange transformation generator.
pub fn interval_exchange_transformation_generator<S: Field + Float>(
    lambda: Vec<S>, pi: Vec<usize>, x: S,
) -> IntervalExchangeTransformationGenerator<S> {
    IntervalExchangeTransformationGenerator::new(IntervalExchangeTransformation::new(lambda, pi), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_exchange_identity() {
        // Two equal intervals, identity permutation
        let map = IntervalExchangeTransformation::new(vec![0.5_f64, 0.5], vec![0, 1]);
        // x=0.25 is in interval 0 (0.0..0.5), relative pos = 0.5, target = interval 0
        // result = 0.0 + 0.5 * 0.5 = 0.25
        assert!((map.step(0.25) - 0.25).abs() < 1e-12);
        // x=0.75 is in interval 1 (0.5..1.0), relative pos = 0.5, target = interval 1
        // result = 0.5 + 0.5 * 0.5 = 0.75
        assert!((map.step(0.75) - 0.75).abs() < 1e-12);
    }

    #[test]
    fn interval_exchange_swap() {
        // Two equal intervals, swap permutation
        let map = IntervalExchangeTransformation::new(vec![0.5_f64, 0.5], vec![1, 0]);
        assert!((map.step(0.25) - 0.75).abs() < 1e-12);
        assert!((map.step(0.75) - 0.25).abs() < 1e-12);
    }
}
