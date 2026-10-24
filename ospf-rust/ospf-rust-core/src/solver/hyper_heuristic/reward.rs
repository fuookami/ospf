//! 自适应信用与有界超体积几何 / Adaptive credit and bounded hypervolume geometry

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::session::SearchRandom;
use crate::solver::heuristic::session::codecs::format_number;
use crate::solver::hyper_heuristic::components::{RecoverableComponent, RewardContext, RewardPolicy};
use crate::solver::hyper_heuristic::config::ComponentSpec;
use crate::solver::hyper_heuristic::invalid;

fn scalar_gain(context: &RewardContext) -> Result<f64> {
    if context.reference.is_empty() && context.candidates.is_empty() { return finite_gain(context.contribution); }
    if context.reference.iter().chain(&context.candidates).any(|point| point.len() != 1 || !point[0].is_finite()) {
        return Err(invalid("奖励需要有限标量坐标 / Reward requires finite scalar coordinates"));
    }
    if context.reference.is_empty() || context.candidates.is_empty() { return Ok(0.0); }
    let before = context.reference.iter().map(|point| point[0]).fold(f64::INFINITY, f64::min);
    let after = context.candidates.iter().map(|point| point[0]).fold(f64::INFINITY, f64::min);
    finite_gain((before - after).max(0.0))
}

fn finite_gain(value: f64) -> Result<f64> {
    if value.is_finite() && value >= 0.0 { Ok(value) }
    else { Err(invalid("奖励或几何数值溢出 / Reward or geometry overflow")) }
}

/// 单位秒标量收益，明确依赖真实时钟 / Scalar gain per second, explicitly dependent on real time
pub struct TimeNormalizedReward { epsilon_seconds: f64 }

impl TimeNormalizedReward {
    /// 配置正的最短计费时长 / Configure a positive minimum charged duration
    pub fn new(epsilon_seconds: f64) -> Result<Self> {
        if !epsilon_seconds.is_finite() || epsilon_seconds <= 0.0 { return Err(invalid("invalid minimum reward duration")); }
        Ok(Self { epsilon_seconds })
    }
}

impl RecoverableComponent for TimeNormalizedReward { fn stateless(&self) -> bool { true } }
impl RewardPolicy for TimeNormalizedReward {
    fn requires_coordinates(&self) -> bool { true }
    fn requires_scalar_coordinates(&self) -> bool { true }
    fn requires_elapsed_time(&self) -> bool { true }
    fn reward(&mut self, context: &RewardContext) -> Result<f64> {
        finite_gain(scalar_gain(context)? / (context.elapsed_nanos as f64 / 1e9 + self.epsilon_seconds))
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtremeObservation { first: String, second: f64 }

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtremeState { size: usize, observations: VecDeque<ExtremeObservation> }

/// 按全局动作窗口过期的各动作近期极值 / Recent per-action extremes expiring in a global action window
pub struct ExtremeValueReward { size: usize, observations: VecDeque<ExtremeObservation> }

impl ExtremeValueReward {
    /// 创建非空有界窗口 / Create a nonempty bounded window
    pub fn new(size: usize) -> Result<Self> {
        if size == 0 || size > i32::MAX as usize { return Err(invalid("invalid extreme-value window")); }
        Ok(Self { size, observations: VecDeque::new() })
    }
}

impl RecoverableComponent for ExtremeValueReward {
    fn save(&self) -> Result<Value> {
        serde_json::to_value(ExtremeState { size: self.size, observations: self.observations.clone() }).map_err(|error| invalid(error.to_string()))
    }
    fn load(&mut self, state: &Value) -> Result<()> {
        let saved: ExtremeState = serde_json::from_value(state.clone()).map_err(|error| invalid(error.to_string()))?;
        if saved.size != self.size || saved.observations.len() > self.size
            || saved.observations.iter().any(|value| value.first.trim().is_empty() || !value.second.is_finite() || value.second < 0.0) {
            return Err(invalid("invalid extreme-value reward state"));
        }
        self.observations = saved.observations;
        Ok(())
    }
}

impl RewardPolicy for ExtremeValueReward {
    fn requires_coordinates(&self) -> bool { true }
    fn requires_scalar_coordinates(&self) -> bool { true }
    fn reward(&mut self, context: &RewardContext) -> Result<f64> {
        let gain = scalar_gain(context)?;
        if context.action.trim().is_empty() { return Err(invalid("empty reward action")); }
        self.observations.push_back(ExtremeObservation { first: context.action.clone(), second: gain });
        while self.observations.len() > self.size { self.observations.pop_front(); }
        Ok(self.observations.iter().filter(|value| value.first == context.action).map(|value| value.second).fold(0.0, f64::max))
    }
}

/// 有效新非支配贡献，不以候选数量代替质量 / Effective novel non-dominated contribution, not offspring count
pub struct DominanceReward;

impl RecoverableComponent for DominanceReward { fn stateless(&self) -> bool { true } }
impl RewardPolicy for DominanceReward {
    fn reward(&mut self, context: &RewardContext) -> Result<f64> { finite_gain(context.contribution) }
}

/// 显式精确或采样模式，禁止隐式降级 / Explicit exact or sampled mode without implicit fallback
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HypervolumeMode {
    /// 有界递归切片 / Bounded recursive slicing
    Exact,
    /// 共同随机数积分 / Common-random-number integration
    Sampled,
}

/// 超体积配置与几何资源限制 / Hypervolume configuration and geometric resource limits
#[derive(Clone, Debug)]
pub struct HypervolumeConfig {
    /// 固定劣参考点；空则校准后冻结 / Fixed worse reference; empty calibrates and freezes
    pub reference: Vec<f64>,
    /// 显式计算模式 / Explicit computation mode
    pub mode: HypervolumeMode,
    /// 每次调用的几何操作额度 / Geometric operation allowance per call
    pub max_work: u64,
    /// 采样模式样本数 / Sample count in sampled mode
    pub samples: u32,
    /// 每次积分重建的共同随机种子 / Common seed reset for each integration
    pub seed: i64,
    /// 自动参考点所需有效候选数 / Valid candidates required for an automatic reference
    pub calibration_observations: u32,
}

impl Default for HypervolumeConfig {
    fn default() -> Self {
        Self { reference: Vec::new(), mode: HypervolumeMode::Exact, max_work: 1_000_000,
            samples: 16384, seed: 1, calibration_observations: 32 }
    }
}

/// 一次超体积增量及采样误差 / One hypervolume increment and its sampling uncertainty
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HypervolumeMeasurement {
    /// 非负增量 / Nonnegative increment
    pub delta: f64,
    /// 是否精确 / Whether exact
    pub exact: bool,
    /// 实际样本数 / Actual sample count
    #[serde(default)]
    pub samples: u32,
    /// 采样标准误差 / Sampling standard error
    #[serde(default)]
    pub standard_error: f64,
    /// 99% Hoeffding绝对误差界 / 99% Hoeffding absolute error bound
    #[serde(default)]
    pub error_bound: f64,
    /// 实际几何操作数 / Actual geometric operations
    #[serde(default)]
    pub work: u64,
    /// 参考点是否冻结 / Whether the reference is frozen
    #[serde(default = "yes")]
    pub calibrated: bool,
}

fn yes() -> bool { true }
fn version() -> u32 { 1 }

impl HypervolumeMeasurement {
    fn zero(mode: HypervolumeMode, calibrated: bool) -> Self {
        Self { delta: 0.0, exact: mode == HypervolumeMode::Exact, samples: 0,
            standard_error: 0.0, error_bound: 0.0, work: 0, calibrated }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HypervolumeState {
    #[serde(default = "version")]
    version: u32,
    definition: String,
    reference: Vec<f64>,
    minimum: Vec<f64>,
    maximum: Vec<f64>,
    observations: u32,
    measurement: Option<HypervolumeMeasurement>,
}

/// 具有可恢复参考点与误差诊断的多维超体积信用 / Multidimensional HV credit with restorable reference and uncertainty
pub struct HypervolumeDeltaReward { config: HypervolumeConfig, state: HypervolumeState }

impl HypervolumeDeltaReward {
    /// 校验定义，不执行任何业务目标 / Validate the definition without evaluating business objectives
    pub fn new(config: HypervolumeConfig) -> Result<Self> {
        if config.reference.iter().any(|value| !value.is_finite()) || config.max_work == 0 || config.max_work > i64::MAX as u64
            || config.samples == 0 || config.samples > i32::MAX as u32
            || config.calibration_observations == 0 || config.calibration_observations > i32::MAX as u32 {
            return Err(invalid("invalid hypervolume parameters"));
        }
        let definition = format!("{}|{}|{}|{}|{}|{}", config.reference.iter().map(|value| format_number(*value)).collect::<Vec<_>>().join(", "),
            if config.mode == HypervolumeMode::Exact { "Exact" } else { "Sampled" }, config.max_work, config.samples, config.seed, config.calibration_observations);
        let state = HypervolumeState { version: 1, definition, reference: config.reference.clone(), minimum: Vec::new(),
            maximum: Vec::new(), observations: 0, measurement: None };
        Ok(Self { config, state })
    }

    /// 最近成功的几何测量，失败不覆盖 / Last successful geometric measurement, unchanged on failure
    pub fn last_measurement(&self) -> Option<&HypervolumeMeasurement> { self.state.measurement.as_ref() }

    /// 测量并事务性提交校准与诊断；额度耗尽明确失败 / Measure and transactionally commit calibration and diagnostics; fail explicitly on exhaustion
    pub fn measure(&mut self, context: &RewardContext) -> Result<HypervolumeMeasurement> {
        let original = self.state.clone();
        match self.measure_inner(context) {
            Ok(value) if value.delta.is_finite() && value.standard_error.is_finite() && value.error_bound.is_finite() => {
                self.state.measurement = Some(value.clone()); Ok(value)
            }
            result => {
                self.state = original;
                result.and_then(|_| Err(invalid("hypervolume or uncertainty overflow")))
            }
        }
    }

    fn measure_inner(&mut self, context: &RewardContext) -> Result<HypervolumeMeasurement> {
        let points: Vec<_> = context.reference.iter().chain(&context.candidates).collect();
        let dimension = if !self.state.reference.is_empty() { self.state.reference.len() }
            else if !self.state.minimum.is_empty() { self.state.minimum.len() }
            else { points.first().map_or(0, |point| point.len()) };
        if points.iter().any(|point| point.is_empty() || point.len() != dimension || point.iter().any(|value| !value.is_finite())) {
            return Err(invalid("invalid hypervolume coordinate dimensions or values"));
        }
        if self.state.reference.is_empty() {
            for point in &points {
                if self.state.minimum.is_empty() { self.state.minimum = (*point).clone(); self.state.maximum = (*point).clone(); }
                else { for axis in 0..dimension {
                    self.state.minimum[axis] = self.state.minimum[axis].min(point[axis]);
                    self.state.maximum[axis] = self.state.maximum[axis].max(point[axis]);
                } }
            }
            self.state.observations = (self.state.observations as u64 + context.candidates.len() as u64)
                .min(self.config.calibration_observations as u64) as u32;
            if self.state.observations < self.config.calibration_observations || self.state.minimum.is_empty() {
                return Ok(HypervolumeMeasurement::zero(self.config.mode, false));
            }
            self.state.reference = (0..dimension).map(|axis| {
                let spread = self.state.maximum[axis] - self.state.minimum[axis];
                self.state.maximum[axis] + spread.max(self.state.maximum[axis].abs().max(1.0)) * 0.1
            }).collect();
            if self.state.reference.iter().enumerate().any(|(axis, value)| !value.is_finite() || *value <= self.state.maximum[axis]) {
                return Err(invalid("automatic reference overflow; configure an explicit reference"));
            }
            return Ok(HypervolumeMeasurement::zero(self.config.mode, true));
        }
        let mut work = GeometryWork { limit: self.config.max_work, used: 0 };
        if self.config.mode == HypervolumeMode::Exact {
            if dimension > 256 { return Err(invalid("exact HV recursion depth exceeded; explicitly select sampling")); }
            let before = exact_volume(&context.reference, &self.state.reference, &mut work)?;
            let all = points.into_iter().cloned().collect::<Vec<_>>();
            let after = exact_volume(&all, &self.state.reference, &mut work)?;
            let mut result = HypervolumeMeasurement::zero(self.config.mode, true);
            result.delta = finite_gain((after - before).max(0.0))?; result.work = work.used;
            Ok(result)
        } else { self.sampled(context, &mut work) }
    }

    fn sampled(&self, context: &RewardContext, work: &mut GeometryWork) -> Result<HypervolumeMeasurement> {
        let reference = &self.state.reference;
        let points: Vec<_> = context.reference.iter().chain(&context.candidates)
            .filter(|point| point.iter().zip(reference).all(|(a, b)| a < b)).collect();
        if points.is_empty() { return Ok(HypervolumeMeasurement::zero(HypervolumeMode::Sampled, true)); }
        let lower: Vec<_> = (0..reference.len()).map(|axis| points.iter().map(|point| point[axis]).fold(f64::INFINITY, f64::min)).collect();
        let widths: Vec<_> = reference.iter().zip(&lower).map(|(high, low)| high - low).collect();
        let volume = finite_gain(widths.iter().fold(1.0, |volume, width| volume * width))?;
        let mut random = SearchRandom::new(self.config.seed as u64);
        let mut hits = 0_u32;
        for _ in 0..self.config.samples {
            let sample: Vec<_> = lower.iter().zip(&widths).map(|(low, width)| low + random.next_unit() * width).collect();
            if !covered(&context.reference, &sample, work)? && covered(&context.candidates, &sample, work)? { hits += 1; }
        }
        let probability = hits as f64 / self.config.samples as f64;
        Ok(HypervolumeMeasurement { delta: volume * probability, exact: false, samples: self.config.samples,
            standard_error: volume * libm::sqrt(probability * (1.0 - probability) / self.config.samples as f64),
            error_bound: volume * libm::sqrt(crate::solver::heuristic::runner::fdlibm_log::log(200.0) / (2.0 * self.config.samples as f64)), work: work.used, calibrated: true })
    }
}

impl RecoverableComponent for HypervolumeDeltaReward {
    fn save(&self) -> Result<Value> { serde_json::to_value(&self.state).map_err(|error| invalid(error.to_string())) }
    fn load(&mut self, state: &Value) -> Result<()> {
        let saved: HypervolumeState = serde_json::from_value(state.clone()).map_err(|error| invalid(error.to_string()))?;
        let dimension = if saved.reference.is_empty() { saved.minimum.len() } else { saved.reference.len() };
        if saved.version != 1 || saved.definition != self.state.definition || saved.observations > self.config.calibration_observations
            || saved.reference.iter().chain(&saved.minimum).chain(&saved.maximum).any(|value| !value.is_finite())
            || saved.minimum.len() != saved.maximum.len()
            || saved.minimum.iter().zip(&saved.maximum).any(|(low, high)| low > high)
            || !saved.minimum.is_empty() && saved.minimum.len() != dimension
            || !self.config.reference.is_empty() && (saved.reference != self.config.reference || !saved.minimum.is_empty() || saved.observations != 0)
            || self.config.reference.is_empty() && !saved.reference.is_empty() && (saved.observations != self.config.calibration_observations
                || saved.minimum.is_empty() || saved.reference.iter().zip(&saved.maximum).any(|(value, high)| value <= high))
            || saved.observations > 0 && saved.minimum.is_empty()
            || saved.measurement.as_ref().is_some_and(|value| !value.delta.is_finite() || value.delta < 0.0 || value.work > self.config.max_work
                || value.exact != (self.config.mode == HypervolumeMode::Exact) || value.calibrated != !saved.reference.is_empty()
                || value.samples != 0 && (self.config.mode != HypervolumeMode::Sampled || value.samples != self.config.samples)
                || !value.standard_error.is_finite() || value.standard_error < 0.0 || !value.error_bound.is_finite() || value.error_bound < 0.0) {
            return Err(invalid("incompatible hypervolume state"));
        }
        self.state = saved;
        Ok(())
    }
}

impl RewardPolicy for HypervolumeDeltaReward {
    fn requires_coordinates(&self) -> bool { true }
    fn reward(&mut self, context: &RewardContext) -> Result<f64> { self.measure(context).map(|value| value.delta) }
}

struct GeometryWork { limit: u64, used: u64 }

impl GeometryWork {
    fn consume(&mut self) -> Result<()> {
        if self.used >= self.limit { return Err(invalid("hypervolume geometric work exhausted")); }
        self.used += 1;
        Ok(())
    }
}

fn covered(points: &[Vec<f64>], sample: &[f64], work: &mut GeometryWork) -> Result<bool> {
    for point in points {
        work.consume()?;
        if point.iter().zip(sample).all(|(point, sample)| point <= sample) { return Ok(true); }
    }
    Ok(false)
}

fn exact_volume(points: &[Vec<f64>], reference: &[f64], work: &mut GeometryWork) -> Result<f64> {
    let mut eligible: Vec<&Vec<f64>> = Vec::new();
    let mut distinct = Vec::new();
    for point in points {
        if distinct.contains(&point) { continue; }
        distinct.push(point);
        work.consume()?;
        if point.iter().zip(reference).all(|(a, b)| a < b) { eligible.push(point); }
    }
    if eligible.is_empty() { return Ok(0.0); }
    let mut front = Vec::new();
    for point in &eligible {
        let mut dominated = false;
        for other in &eligible {
            work.consume()?;
            if other != point && other.iter().zip(point.iter()).all(|(a, b)| a <= b) { dominated = true; break; }
        }
        if !dominated { front.push(*point); }
    }
    if front.len() == 1 {
        return finite_gain(reference.iter().zip(front[0]).fold(1.0, |volume, (high, low)| volume * (high - low)));
    }
    if reference.len() == 1 { return finite_gain(reference[0] - front.iter().map(|point| point[0]).fold(f64::INFINITY, f64::min)); }
    if reference.len() == 2 {
        front.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));
        let mut height = reference[1]; let mut area = 0.0;
        for point in front {
            work.consume()?;
            if point[1] < height { area += (reference[0] - point[0]) * (height - point[1]); height = point[1]; }
        }
        return finite_gain(area);
    }
    let axis = reference.len() - 1;
    let mut levels: Vec<_> = front.iter().map(|point| point[axis]).collect();
    levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)); levels.dedup();
    let mut volume = 0.0;
    for (index, &level) in levels.iter().enumerate() {
        work.consume()?;
        let active: Vec<_> = front.iter().filter(|point| point[axis] <= level).map(|point| point[..axis].to_vec()).collect();
        volume += exact_volume(&active, &reference[..axis], work)? * (levels.get(index + 1).copied().unwrap_or(reference[axis]) - level);
    }
    finite_gain(volume)
}

pub(crate) fn builtin_reward(spec: &ComponentSpec) -> Result<Box<dyn RewardPolicy>> {
    match spec.r#type.as_str() {
        "time-normalized" => {
            spec.validate_parameters(&["epsilonSeconds"])?;
            Ok(Box::new(TimeNormalizedReward::new(spec.number("epsilonSeconds", 1e-9)?)?))
        }
        "extreme-value" => {
            spec.validate_parameters(&["window"])?;
            let size = usize::try_from(spec.integer("window", 50)?).map_err(|_| invalid("extreme window overflow"))?;
            Ok(Box::new(ExtremeValueReward::new(size)?))
        }
        "dominance" => { spec.validate_parameters(&[])?; Ok(Box::new(DominanceReward)) }
        "hypervolume-delta" => {
            spec.validate_parameters(&["reference", "referenceX", "referenceY", "mode", "maxWork", "samples", "seed", "calibrationObservations"])?;
            let values = &spec.parameters;
            if values.contains_key("reference") && (values.contains_key("referenceX") || values.contains_key("referenceY"))
                || values.contains_key("referenceX") != values.contains_key("referenceY") { return Err(invalid("ambiguous hypervolume reference")); }
            let reference = if let Some(value) = values.get("reference") {
                value.as_array().ok_or_else(|| invalid("hypervolume reference must be an array"))?.iter()
                    .map(|value| value.as_f64().filter(|value| value.is_finite()).ok_or_else(|| invalid("invalid hypervolume reference"))).collect::<Result<_>>()?
            } else if values.contains_key("referenceX") { vec![spec.number("referenceX", 0.0)?, spec.number("referenceY", 0.0)?] }
            else { Vec::new() };
            let mode = match values.get("mode").map(Value::as_str).unwrap_or(Some("exact")) {
                Some("exact") => HypervolumeMode::Exact, Some("sampled") => HypervolumeMode::Sampled,
                _ => return Err(invalid("unknown hypervolume mode")),
            };
            let seed = values.get("seed").map(|value| value.as_i64().ok_or_else(|| invalid("hypervolume seed must be an integer"))).transpose()?.unwrap_or(1);
            Ok(Box::new(HypervolumeDeltaReward::new(HypervolumeConfig { reference, mode, seed,
                max_work: spec.integer("maxWork", 1_000_000)?,
                samples: u32::try_from(spec.integer("samples", 16384)?).map_err(|_| invalid("hypervolume sample count overflow"))?,
                calibration_observations: u32::try_from(spec.integer("calibrationObservations", 32)?).map_err(|_| invalid("hypervolume calibration count overflow"))?,
            })?))
        }
        _ => Err(invalid("unknown reward policy")),
    }
}

#[cfg(test)]
mod tests;
