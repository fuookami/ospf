//! Singer 映射。
//! Singer map.

use crate::algebra::Field;
use num_traits::Float;
scalar_map!(
    /// Singer 映射。
    /// Singer map.
    SingerMap,
    /// Singer 映射序列生成器。
    /// Singer map sequence generator.
    SingerMapGenerator,
    [mu],
    |system, state| {
        let c786 = S::from(7.86).expect("7.86 must be representable");
        let c2323 = S::from(23.23).expect("23.23 must be representable");
        let c2875 = S::from(28.75).expect("28.75 must be representable");
        let c1330 = S::from(13.30).expect("13.30 must be representable");
        let x2 = state * state;
        let x3 = x2 * state;
        let x4 = x3 * state;
        system.mu * (c786 * state - c2323 * x2 + c2875 * x3 - c1330 * x4)
    }
);

impl<S: Field + Float> Default for SingerMap<S> {
    fn default() -> Self {
        Self::new(S::one())
    }
}

impl<S: Field + Float> Default for SingerMapGenerator<S> {
    fn default() -> Self {
        Self::new(
            SingerMap::default(),
            S::from(0.5).expect("0.5 must be representable"),
        )
    }
}

/// 创建 Singer 映射。
/// Create a Singer map.
pub fn singer_map<S: Field + Float>(mu: S) -> SingerMap<S> {
    SingerMap::new(mu)
}

/// 创建 Singer 映射生成器。
/// Create a Singer map generator.
pub fn singer_map_generator<S: Field + Float>(mu: S, x: S) -> SingerMapGenerator<S> {
    SingerMapGenerator::new(SingerMap::new(mu), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singer_step_formula() {
        let system = SingerMap::new(1.0_f64);
        let next = system.step(0.5);
        let expected = 7.86 * 0.5 - 23.23 * 0.25 + 28.75 * 0.125 - 13.30 * 0.0625;
        assert!((next - expected).abs() < 1e-12);
    }
}
