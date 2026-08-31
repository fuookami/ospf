//! 帐篷映射。
//! Tent map.

use num_traits::Float;
use crate::algebra::Field;
use super::helpers::default_float;

scalar_map!(
    /// 帐篷映射。
    /// Tent map.
    TentMap,
    /// 帐篷映射序列生成器。
    /// Tent map sequence generator.
    TentMapGenerator,
    [mu],
    |system, state| {
        let one = S::one();
        let half = S::from(0.5).expect("0.5 must be representable");
        if state < half {
            system.mu * state
        } else {
            system.mu * (one - state)
        }
    }
);

impl<S: Field + Float> Default for TentMap<S> {
    fn default() -> Self {
        Self::new(default_float(1.5, "1.5 must be representable"))
    }
}

impl<S: Field + Float> Default for TentMapGenerator<S> {
    fn default() -> Self {
        Self::new(TentMap::default(), S::from(0.5).expect("0.5 must be representable"))
    }
}

/// 创建帐篷映射。
/// Create a tent map.
pub fn tent_map<S: Field + Float>(mu: S) -> TentMap<S> {
    TentMap::new(mu)
}

/// 创建帐篷映射生成器。
/// Create a tent map generator.
pub fn tent_map_generator<S: Field + Float>(mu: S, x: S) -> TentMapGenerator<S> {
    TentMapGenerator::new(TentMap::new(mu), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tent_step_formula() {
        let system = TentMap::new(1.5_f64);
        assert!((system.step(0.3) - 0.45).abs() < 1e-12);
        assert!((system.step(0.7) - 0.45).abs() < 1e-12);
    }
}
