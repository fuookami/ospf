//! 熵源 / Entropy source
//!
//! 统一使用 [`ospf_rust_math::distribution::Entropy`] 契约，保证分布、随机过程与启发式算子消费同一熵源。
//! The [`ospf_rust_math::distribution::Entropy`] contract is reused so distributions, stochastic processes, and
//! heuristic operators consume the same entropy source.

use std::cell::RefCell;
use std::rc::Rc;

pub use ospf_rust_math::distribution::Entropy;

/// 可复现的线性同余熵源 / Reproducible linear-congruential entropy source
///
/// 与 Kotlin 侧测试使用的 LCG 完全一致（乘子 `6364136223846793005`、增量 `1442695040888963407`、
/// 取高 53 位），因此同一 seed 在两侧产生同一序列。
/// Matches the LCG used by the Kotlin tests (multiplier `6364136223846793005`, increment `1442695040888963407`,
/// taking the high 53 bits), so the same seed produces the same stream on both sides.
#[derive(Debug, Clone)]
pub struct SeededEntropy {
    state: u64,
}

impl SeededEntropy {
    /// 以种子创建熵源 / Create an entropy source from a seed
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// 当前内部状态 / Current internal state
    pub fn state(&self) -> u64 {
        self.state
    }
}

impl Default for SeededEntropy {
    fn default() -> Self {
        Self::new(20_240_521)
    }
}

impl Entropy for SeededEntropy {
    fn next_unit(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.state >> 11) as f64) / ((1_u64 << 53) as f64)
    }
}

/// 由闭包驱动的熵源 / Entropy source driven by a closure
pub struct FnEntropy<F>
where
    F: FnMut() -> f64,
{
    generator: F,
}

impl<F> FnEntropy<F>
where
    F: FnMut() -> f64,
{
    /// 创建闭包熵源 / Create a closure-backed entropy source
    pub fn new(generator: F) -> Self {
        Self { generator }
    }
}

impl<F> Entropy for FnEntropy<F>
where
    F: FnMut() -> f64,
{
    fn next_unit(&mut self) -> f64 {
        (self.generator)()
    }
}

/// 共享熵源，让骨架与 Q 学习代理消费同一条随机流 / Shared entropy letting the skeleton and a Q-learning agent consume one random stream
#[derive(Clone)]
pub struct SharedEntropy {
    inner: Rc<RefCell<Box<dyn Entropy>>>,
}

impl SharedEntropy {
    /// 以独占熵源创建共享句柄 / Create a shared handle from an owned entropy source
    pub fn new(entropy: Box<dyn Entropy>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(entropy)),
        }
    }

    /// 复用已有共享句柄 / Reuse an existing shared handle
    pub fn from_shared(inner: Rc<RefCell<Box<dyn Entropy>>>) -> Self {
        Self { inner }
    }
}

impl Entropy for SharedEntropy {
    fn next_unit(&mut self) -> f64 {
        self.inner.borrow_mut().next_unit()
    }
}

/// 读取一个 `[0, 1]` 区间内的均匀值，NaN 归零 / Read a uniform value in `[0, 1]`, mapping NaN to zero
pub fn unit(entropy: &mut dyn Entropy) -> f64 {
    let value = entropy.next_unit();
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// 按概率判断是否触发 / Decide whether an event fires at the given probability
pub fn should_fire(entropy: &mut dyn Entropy, probability: f64) -> bool {
    probability >= 1.0 || unit(entropy) < probability
}
