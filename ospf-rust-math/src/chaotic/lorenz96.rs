//! Lorenz 96 模型。
//! Lorenz 96 model.
//!
//! N 维混沌系统，状态为变长 `Vec<S>`。
//! N-dimensional chaotic system with variable-length `Vec<S>` state.

use num_traits::Float;
use crate::algebra::Field;
use super::helpers::default_float;

/// Lorenz 96 模型。
/// Lorenz 96 model.
///
/// 公式: dx_i/dt = (x_{i+1} - x_{i-2}) * x_{i-1} - x_i + a
/// （周期性边界条件）
#[derive(Clone, Debug, PartialEq)]
pub struct Lorenz96Model<S: Field + Float = f64> {
    a: S,
    h: S,
}

impl<S: Field + Float> Lorenz96Model<S> {
    pub fn new(a: S, h: S) -> Self {
        Self { a, h }
    }

    pub fn a(&self) -> S { self.a }
    pub fn h(&self) -> S { self.h }

    /// 执行一次 Lorenz 96 步进。
    /// Execute one Lorenz 96 step.
    pub fn step(&self, state: &[S]) -> Vec<S> {
        let n = state.len();
        (0..n)
            .map(|i| {
                let xip1 = state[(i + 1) % n];
                let xi = state[i];
                let xim1 = state[(i + n - 1) % n];
                let xim2 = state[(i + n - 2) % n];
                let dx = (xip1 - xim2) * xim1 - xi + self.a;
                xi + self.h * dx
            })
            .collect()
    }

    pub fn generator(self, initial: Vec<S>) -> Lorenz96ModelGenerator<S> {
        Lorenz96ModelGenerator::new(self, initial)
    }
}

impl<S: Field + Float> Default for Lorenz96Model<S> {
    fn default() -> Self {
        Self::new(
            default_float(8.0, "8.0 must be representable"),
            default_float(0.01, "0.01 must be representable"),
        )
    }
}

/// Lorenz 96 模型序列生成器。
/// Lorenz 96 model sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct Lorenz96ModelGenerator<S: Field + Float = f64> {
    model: Lorenz96Model<S>,
    state: Vec<S>,
}

impl<S: Field + Float> Lorenz96ModelGenerator<S> {
    pub fn new(model: Lorenz96Model<S>, state: Vec<S>) -> Self {
        Self { model, state }
    }

    pub fn model(&self) -> &Lorenz96Model<S> { &self.model }
    pub fn state(&self) -> &[S] { &self.state }

    pub fn next_state(&mut self) -> Vec<S> {
        let current = self.state.clone();
        self.state = self.model.step(&current);
        current
    }
}

impl<S: Field + Float> Iterator for Lorenz96ModelGenerator<S> {
    type Item = Vec<S>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_state())
    }
}

/// 创建 Lorenz 96 模型。
/// Create a Lorenz 96 model.
pub fn lorenz96_model<S: Field + Float>(a: S, h: S) -> Lorenz96Model<S> {
    Lorenz96Model::new(a, h)
}

/// 创建 Lorenz 96 模型生成器。
/// Create a Lorenz 96 model generator.
pub fn lorenz96_model_generator<S: Field + Float>(a: S, h: S, state: Vec<S>) -> Lorenz96ModelGenerator<S> {
    Lorenz96ModelGenerator::new(Lorenz96Model::new(a, h), state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lorenz96_step_formula() {
        let model = Lorenz96Model::new(8.0_f64, 0.01);
        let state = vec![1.0, 1.0, 1.0, 1.0];
        let next = model.step(&state);
        assert_eq!(next.len(), 4);
        // For all-ones: dx = (1 - 1) * 1 - 1 + 8 = 7
        for val in &next {
            assert!((val - 1.07).abs() < 1e-12);
        }
    }
}
