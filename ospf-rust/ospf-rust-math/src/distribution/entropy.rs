//! 熵源契约 / Entropy source contract

/// 均匀熵源 / Uniform entropy source
///
/// 分布与随机过程都通过该接口取随机数，随机性因此可替换：伪随机、混沌序列或固定种子可复现流。
/// Distributions and stochastic processes draw through this interface, so randomness is replaceable:
/// pseudo-random, chaotic sequences, or a reproducible seeded stream.
pub trait Entropy {
    /// 采样 `[0, 1)` 上的均匀随机数 / Sample a uniform random number in `[0, 1)`
    fn next_unit(&mut self) -> f64;
}

impl<T> Entropy for &mut T
where
    T: Entropy + ?Sized,
{
    fn next_unit(&mut self) -> f64 {
        (**self).next_unit()
    }
}
