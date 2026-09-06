//! 逻辑斯蒂映射。
//! Logistic map.

use super::helpers::default_float;
use crate::algebra::Field;
use num_traits::Float;

scalar_map!(
    /// 逻辑斯蒂映射。
    /// Logistic map.
    LogisticMap,
    /// 逻辑斯蒂映射序列生成器。
    /// Logistic map sequence generator.
    LogisticMapGenerator,
    [a],
    |system, state| {
        let one = S::one();
        system.a * state * (one - state)
    }
);

impl<S: Field + Float> Default for LogisticMap<S> {
    fn default() -> Self {
        Self::new(default_float(3.9, "3.9 must be representable"))
    }
}

impl<S: Field + Float> Default for LogisticMapGenerator<S> {
    fn default() -> Self {
        Self::new(
            LogisticMap::default(),
            S::from(0.5).expect("0.5 must be representable"),
        )
    }
}

/// 创建逻辑斯蒂映射。
/// Create a logistic map.
pub fn logistic_map<S: Field + Float>(a: S) -> LogisticMap<S> {
    LogisticMap::new(a)
}

/// 创建逻辑斯蒂映射生成器。
/// Create a logistic map generator.
pub fn logistic_map_generator<S: Field + Float>(a: S, x: S) -> LogisticMapGenerator<S> {
    LogisticMapGenerator::new(LogisticMap::new(a), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logistic_step_formula() {
        let system = LogisticMap::new(3.9_f64);
        let next = system.step(0.5);
        assert!((next - 3.9 * 0.5 * 0.5).abs() < 1e-12);
    }
}
