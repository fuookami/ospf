//! 参数演化算子 / Parameter evolution operators
//!
//! 覆盖「参数随迭代变化」的全部形态：确定性调度、混沌序列、分形投影、分布采样与组合子。
//! Covers every shape of "parameter varies with iteration": deterministic schedules, chaotic sequences, fractal
//! projections, distribution sampling, and combinators.

use ospf_rust_math::distribution::{Distribution, Entropy};
use ospf_rust_math::sequence::{ChannelProjection, ProjectedValue};

use super::super::iteration::Iteration;
use super::unit;

/// 参数演化源，把「参数取值」抽象为一个函数 / Parameter source abstracting parameter values as a function
pub trait ParameterSource {
    /// 计算当前迭代的参数值 / Compute the parameter value at the current iteration
    fn value(&mut self, iteration: &Iteration, entropy: &mut dyn Entropy) -> f64;
}

/// 迭代进度，取值位于 `[0, 1]` / Iteration progress within `[0, 1]`
pub fn progress(iteration: &Iteration, limit: u64) -> f64 {
    if limit == 0 {
        return 1.0;
    }
    (iteration.iteration() as f64 / limit as f64).clamp(0.0, 1.0)
}

/// 常数参数 / Constant parameter
#[derive(Debug, Clone, Copy)]
pub struct ConstantSource {
    /// 参数值 / Parameter value
    pub value: f64,
}

impl ConstantSource {
    /// 创建常数参数源 / Create a constant source
    pub fn new(value: f64) -> Self {
        Self { value }
    }
}

impl ParameterSource for ConstantSource {
    fn value(&mut self, _iteration: &Iteration, _entropy: &mut dyn Entropy) -> f64 {
        self.value
    }
}

/// 线性调度，从 `from` 线性过渡到 `to` / Linear schedule transitioning from `from` to `to`
#[derive(Debug, Clone, Copy)]
pub struct LinearSource {
    /// 起始值 / Start value
    pub from: f64,
    /// 结束值 / End value
    pub to: f64,
    /// 迭代上限 / Iteration limit
    pub limit: u64,
}

impl LinearSource {
    /// 创建线性调度 / Create a linear schedule
    pub fn new(from: f64, to: f64, limit: u64) -> Self {
        Self { from, to, limit }
    }
}

impl ParameterSource for LinearSource {
    fn value(&mut self, iteration: &Iteration, _entropy: &mut dyn Entropy) -> f64 {
        let progress = progress(iteration, self.limit);
        self.from + (self.to - self.from) * progress
    }
}

/// 几何衰减调度（等比数列）/ Geometric decay schedule
#[derive(Debug, Clone, Copy)]
pub struct GeometricSource {
    /// 初始值 / Initial value
    pub initial: f64,
    /// 公比 / Common ratio
    pub ratio: f64,
}

impl GeometricSource {
    /// 创建几何调度 / Create a geometric schedule
    pub fn new(initial: f64, ratio: f64) -> Self {
        Self { initial, ratio }
    }
}

impl ParameterSource for GeometricSource {
    fn value(&mut self, iteration: &Iteration, _entropy: &mut dyn Entropy) -> f64 {
        self.initial * self.ratio.powi(iteration.iteration() as i32)
    }
}

/// 指数调度，按时间常数衰减 / Exponential schedule decaying with a time constant
#[derive(Debug, Clone, Copy)]
pub struct ExponentialSource {
    /// 初始值 / Initial value
    pub initial: f64,
    /// 衰减率 / Decay rate
    pub rate: f64,
}

impl ExponentialSource {
    /// 创建指数调度 / Create an exponential schedule
    pub fn new(initial: f64, rate: f64) -> Self {
        Self { initial, rate }
    }
}

impl ParameterSource for ExponentialSource {
    fn value(&mut self, iteration: &Iteration, _entropy: &mut dyn Entropy) -> f64 {
        self.initial * (-self.rate * iteration.iteration() as f64).exp()
    }
}

/// 混沌参数源，把一维混沌序列线性映射到目标区间 / Chaotic source mapping a 1D chaotic sequence onto the target interval
pub struct ChaoticSource {
    generator: Box<dyn FnMut() -> f64>,
    /// 混沌序列下界 / Chaotic sequence lower bound
    pub domain_lower_bound: f64,
    /// 混沌序列上界 / Chaotic sequence upper bound
    pub domain_upper_bound: f64,
    /// 目标下界 / Target lower bound
    pub lower_bound: f64,
    /// 目标上界 / Target upper bound
    pub upper_bound: f64,
}

impl ChaoticSource {
    /// 创建混沌参数源 / Create a chaotic source
    pub fn new(generator: Box<dyn FnMut() -> f64>) -> Self {
        Self {
            generator,
            domain_lower_bound: 0.0,
            domain_upper_bound: 1.0,
            lower_bound: 0.0,
            upper_bound: 1.0,
        }
    }

    /// 设置定义域与目标区间 / Set the domain and the target interval
    pub fn with_domain(
        mut self,
        domain_lower_bound: f64,
        domain_upper_bound: f64,
        lower_bound: f64,
        upper_bound: f64,
    ) -> Self {
        self.domain_lower_bound = domain_lower_bound;
        self.domain_upper_bound = domain_upper_bound;
        self.lower_bound = lower_bound;
        self.upper_bound = upper_bound;
        self
    }
}

impl ParameterSource for ChaoticSource {
    fn value(&mut self, _iteration: &Iteration, _entropy: &mut dyn Entropy) -> f64 {
        let raw = (self.generator)();
        normalize(
            raw,
            self.domain_lower_bound,
            self.domain_upper_bound,
            self.lower_bound,
            self.upper_bound,
        )
    }
}

/// 分形参数源，把多维分形序列按通道投影后映射到目标区间 / Fractal source projecting a multi-dimensional fractal sequence onto the target interval
pub struct FractalSource {
    generator: Box<dyn FnMut() -> ProjectedValue>,
    /// 通道投影 / Channel projection
    pub projection: ChannelProjection,
    /// 投影值下界 / Projection lower bound
    pub domain_lower_bound: f64,
    /// 投影值上界 / Projection upper bound
    pub domain_upper_bound: f64,
    /// 目标下界 / Target lower bound
    pub lower_bound: f64,
    /// 目标上界 / Target upper bound
    pub upper_bound: f64,
}

impl FractalSource {
    /// 创建分形参数源 / Create a fractal source
    pub fn new(generator: Box<dyn FnMut() -> ProjectedValue>) -> Self {
        Self {
            generator,
            projection: ChannelProjection::Magnitude,
            domain_lower_bound: 0.0,
            domain_upper_bound: 1.0,
            lower_bound: 0.0,
            upper_bound: 1.0,
        }
    }

    /// 设置投影与区间 / Set the projection and intervals
    pub fn with_projection(
        mut self,
        projection: ChannelProjection,
        domain_lower_bound: f64,
        domain_upper_bound: f64,
        lower_bound: f64,
        upper_bound: f64,
    ) -> Self {
        self.projection = projection;
        self.domain_lower_bound = domain_lower_bound;
        self.domain_upper_bound = domain_upper_bound;
        self.lower_bound = lower_bound;
        self.upper_bound = upper_bound;
        self
    }
}

impl ParameterSource for FractalSource {
    fn value(&mut self, _iteration: &Iteration, _entropy: &mut dyn Entropy) -> f64 {
        let point = (self.generator)();
        let raw = self
            .projection
            .project(&point)
            .unwrap_or(self.domain_lower_bound);
        normalize(
            raw,
            self.domain_lower_bound,
            self.domain_upper_bound,
            self.lower_bound,
            self.upper_bound,
        )
    }
}

/// 分布参数源，按概率分布采样并可裁剪到区间 / Distribution source sampling from a distribution with optional clamping
pub struct DistributionSource {
    distribution: Box<dyn Distribution>,
    /// 下界（可选）/ Lower bound (optional)
    pub lower_bound: Option<f64>,
    /// 上界（可选）/ Upper bound (optional)
    pub upper_bound: Option<f64>,
}

impl DistributionSource {
    /// 创建分布参数源 / Create a distribution source
    pub fn new(distribution: Box<dyn Distribution>) -> Self {
        Self {
            distribution,
            lower_bound: None,
            upper_bound: None,
        }
    }

    /// 设置裁剪区间 / Set the clamping interval
    pub fn with_bounds(mut self, lower_bound: f64, upper_bound: f64) -> Self {
        self.lower_bound = Some(lower_bound);
        self.upper_bound = Some(upper_bound);
        self
    }
}

impl ParameterSource for DistributionSource {
    fn value(&mut self, _iteration: &Iteration, entropy: &mut dyn Entropy) -> f64 {
        let sampled = self.distribution.sample_checked(entropy).unwrap_or_else(|_| {
            self.lower_bound.unwrap_or(0.0)
        });
        let mut value = sampled;
        if let Some(lower) = self.lower_bound {
            value = value.max(lower);
        }
        if let Some(upper) = self.upper_bound {
            value = value.min(upper);
        }
        value
    }
}

/// 抖动参数源，在基础参数上叠加对称随机抖动 / Jittered source adding symmetric random jitter on top of a base source
pub struct JitteredSource {
    base: Box<dyn ParameterSource>,
    /// 抖动幅度 / Jitter amplitude
    pub amplitude: f64,
}

impl JitteredSource {
    /// 创建抖动参数源 / Create a jittered source
    pub fn new(base: Box<dyn ParameterSource>, amplitude: f64) -> Self {
        Self { base, amplitude }
    }
}

impl ParameterSource for JitteredSource {
    fn value(&mut self, iteration: &Iteration, entropy: &mut dyn Entropy) -> f64 {
        let base = self.base.value(iteration, entropy);
        base + self.amplitude * (2.0 * unit(entropy) - 1.0)
    }
}

/// 把原始值从定义域线性映射到目标区间 / Linearly map a raw value from its domain onto the target interval
pub fn normalize(
    raw: f64,
    domain_lower: f64,
    domain_upper: f64,
    lower: f64,
    upper: f64,
) -> f64 {
    let span = domain_upper - domain_lower;
    let unit = if span <= 0.0 || !span.is_finite() {
        0.5
    } else {
        ((raw - domain_lower) / span).clamp(0.0, 1.0)
    };
    lower + unit * (upper - lower)
}
