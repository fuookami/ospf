//! 仅改变工作集的可恢复接受历史 / Restorable acceptance history affecting only working sets

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::session::{ListCodec, SearchCodec, SearchProblem};
use crate::solver::hyper_heuristic::components::RecoverableComponent;
use crate::solver::hyper_heuristic::config::ComponentSpec;
use crate::solver::hyper_heuristic::invalid;
use super::AcceptancePolicy;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LateState { length: usize, cursor: usize, history: Value }

/// 保存原始业务目标的延迟接受；拒绝也推进历史 / Late acceptance retaining original objectives and advancing history on rejection
pub struct LateAcceptance<O> { codec: ListCodec<Box<dyn SearchCodec<O>>>, length: usize, cursor: usize, history: Vec<O> }

impl<O: Clone> LateAcceptance<O> {
    /// 绑定无损业务目标编码及显式历史长度 / Bind a lossless objective codec and explicit history length
    pub fn new(codec: Box<dyn SearchCodec<O>>, length: usize) -> Result<Self> {
        if length == 0 || length > i32::MAX as usize { return Err(invalid("invalid late acceptance length")); }
        Ok(Self { codec: ListCodec::new(codec), length, cursor: 0, history: Vec::new() })
    }
}

impl<O: Clone> RecoverableComponent for LateAcceptance<O> {
    fn save(&self) -> Result<Value> {
        serde_json::to_value(LateState { length: self.length, cursor: self.cursor, history: self.codec.encode(&self.history)? })
            .map_err(|error| invalid(error.to_string()))
    }
    fn load(&mut self, state: &Value) -> Result<()> {
        let saved: LateState = serde_json::from_value(state.clone()).map_err(|error| invalid(error.to_string()))?;
        if saved.length != self.length || saved.cursor >= self.length { return Err(invalid("invalid late acceptance definition")); }
        let history = self.codec.decode(&saved.history)?;
        if !history.is_empty() && history.len() != self.length || history.is_empty() && saved.cursor != 0 {
            return Err(invalid("invalid late acceptance history"));
        }
        self.history = history; self.cursor = saved.cursor;
        Ok(())
    }
}

impl<O: Clone, V: HeuristicValue> AcceptancePolicy<O, V> for LateAcceptance<O> {
    fn validate_problem(&self, problem: &dyn SearchProblem<O, V>) -> Result<()> {
        if problem.multi_objective() { Err(invalid("late acceptance requires a scalar objective")) } else { Ok(()) }
    }
    fn accept(
        &mut self,
        problem: &dyn SearchProblem<O, V>,
        working: &[Individual<O, V>],
        candidate: &Individual<O, V>,
    ) -> bool {
        if problem.multi_objective() { return false; }
        let current = &working.first().unwrap_or(candidate).fitness;
        if self.history.is_empty() { self.history = vec![current.clone(); self.length]; }
        let accepted = !problem.dominates(current, &candidate.fitness) || !problem.dominates(&self.history[self.cursor], &candidate.fitness);
        self.history[self.cursor] = if accepted { candidate.fitness.clone() } else { current.clone() };
        self.cursor = (self.cursor + 1) % self.length;
        accepted
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordState { ratio: f64, horizon: u64, clock: u64, best: Option<f64>, scale: f64 }

/// 使用显式最小化标量坐标的动态记录阈值 / Dynamic record threshold using explicit scalar minimization coordinates
pub struct RecordToRecordAcceptance { state: RecordState }

impl RecordToRecordAcceptance {
    /// 配置初始容差比例及收紧候选数 / Configure initial tolerance ratio and tightening candidate count
    pub fn new(initial_ratio: f64, horizon: u64) -> Result<Self> {
        if !initial_ratio.is_finite() || !(0.0..=1.0).contains(&initial_ratio) || horizon == 0 || horizon > i64::MAX as u64 {
            return Err(invalid("invalid record-to-record definition"));
        }
        Ok(Self { state: RecordState { ratio: initial_ratio, horizon, clock: 0, best: None, scale: 0.0 } })
    }
}

impl RecoverableComponent for RecordToRecordAcceptance {
    fn save(&self) -> Result<Value> { serde_json::to_value(&self.state).map_err(|error| invalid(error.to_string())) }
    fn load(&mut self, state: &Value) -> Result<()> {
        let saved: RecordState = serde_json::from_value(state.clone()).map_err(|error| invalid(error.to_string()))?;
        if saved.ratio != self.state.ratio || saved.horizon != self.state.horizon || saved.clock > i64::MAX as u64
            || !saved.scale.is_finite() || saved.scale < 0.0 || saved.best.is_some_and(|best| !best.is_finite() || best.abs() > saved.scale)
            || (saved.clock == 0) != saved.best.is_none() {
            return Err(invalid("invalid record-to-record state"));
        }
        self.state = saved;
        Ok(())
    }
}

impl<O: Clone, V: HeuristicValue> AcceptancePolicy<O, V> for RecordToRecordAcceptance {
    fn validate_problem(&self, problem: &dyn SearchProblem<O, V>) -> Result<()> {
        if problem.multi_objective() || !problem.has_objective_coordinates() { Err(invalid("record acceptance requires scalar coordinates")) }
        else { Ok(()) }
    }
    fn accept(
        &mut self,
        problem: &dyn SearchProblem<O, V>,
        working: &[Individual<O, V>],
        candidate: &Individual<O, V>,
    ) -> bool {
        if problem.multi_objective() || self.state.clock == i64::MAX as u64 { return false; }
        let points = working.iter().chain(std::iter::once(candidate)).map(|value| problem.checked_objective_coordinates(&value.fitness))
            .collect::<Result<Vec<_>>>();
        let Ok(points) = points else { return false; };
        if points.iter().any(|point| point.len() != 1) { return false; }
        let value = points.last().map(|point| point[0]).unwrap_or(0.0);
        let record = points.iter().map(|point| point[0]).fold(self.state.best.unwrap_or(f64::INFINITY), f64::min);
        self.state.best = Some(record);
        self.state.scale = points.iter().map(|point| point[0].abs()).fold(self.state.scale, f64::max);
        let tolerance = self.state.ratio * (1.0 - self.state.clock as f64 / self.state.horizon as f64).clamp(0.0, 1.0) * self.state.scale;
        self.state.clock += 1;
        value - record <= tolerance
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AutoState {
    #[serde(default = "version")]
    version: u32,
    horizon: u64,
    capacity: usize,
    span: usize,
    decisions: u64,
    improvements: usize,
    history: Value,
}

fn version() -> u32 { 1 }

/// 按预算规模与严格改善率调节跨度的无温度接受 / Temperature-free acceptance adapting span to budget scale and strict improvement rate
pub struct AutoAcceptance<O> {
    codec: ListCodec<Box<dyn SearchCodec<O>>>,
    horizon: u64,
    capacity: usize,
    span: usize,
    decisions: u64,
    improvements: usize,
    history: VecDeque<O>,
}

impl<O: Clone> AutoAcceptance<O> {
    /// 绑定原业务目标编码及预算规模提示；提示不终止运行 / Bind the original objective codec and a budget-scale hint, not a stopping limit
    pub fn new(codec: Box<dyn SearchCodec<O>>, horizon: u64) -> Result<Self> {
        if horizon > i64::MAX as u64 { return Err(invalid("automatic acceptance horizon overflow")); }
        let capacity = (libm::sqrt(horizon.max(1) as f64) as usize).clamp(8, 256);
        Ok(Self { codec: ListCodec::new(codec), horizon, capacity, span: (capacity / 2).max(2), decisions: 0,
            improvements: 0, history: VecDeque::new() })
    }

    /// 当前自适应延迟跨度，仅供诊断 / Current adaptive history span, for diagnostics only
    pub fn history_span(&self) -> usize { self.span }
}

impl<O: Clone> RecoverableComponent for AutoAcceptance<O> {
    fn save(&self) -> Result<Value> {
        serde_json::to_value(AutoState { version: 1, horizon: self.horizon, capacity: self.capacity, span: self.span,
            decisions: self.decisions, improvements: self.improvements, history: self.codec.encode(&self.history.iter().cloned().collect())? })
            .map_err(|error| invalid(error.to_string()))
    }
    fn load(&mut self, state: &Value) -> Result<()> {
        let saved: AutoState = serde_json::from_value(state.clone()).map_err(|error| invalid(error.to_string()))?;
        if saved.version != 1 || saved.horizon != self.horizon || saved.capacity != self.capacity
            || !(2..=self.capacity).contains(&saved.span) || saved.decisions > i64::MAX as u64
            || saved.improvements > (saved.decisions % self.capacity as u64) as usize {
            return Err(invalid("incompatible automatic acceptance state"));
        }
        let history = self.codec.decode(&saved.history)?;
        if history.len() as u64 != saved.decisions.min(self.capacity as u64) { return Err(invalid("incomplete automatic acceptance history")); }
        self.history = history.into(); self.span = saved.span; self.decisions = saved.decisions; self.improvements = saved.improvements;
        Ok(())
    }
}

impl<O: Clone, V: HeuristicValue> AcceptancePolicy<O, V> for AutoAcceptance<O> {
    fn accept(
        &mut self,
        problem: &dyn SearchProblem<O, V>,
        working: &[Individual<O, V>],
        candidate: &Individual<O, V>,
    ) -> bool {
        if self.decisions == i64::MAX as u64 { return false; }
        let current = &working.first().unwrap_or(candidate).fitness;
        let delayed = self.history.get(self.history.len().saturating_sub(self.span)).unwrap_or(current);
        let improved = problem.dominates(&candidate.fitness, current);
        let accepted = !problem.dominates(current, &candidate.fitness) || !problem.dominates(delayed, &candidate.fitness);
        if improved { self.improvements += 1; }
        self.history.push_back(if accepted { candidate.fitness.clone() } else { current.clone() });
        while self.history.len() > self.capacity { self.history.pop_front(); }
        self.decisions += 1;
        if self.decisions % self.capacity as u64 == 0 {
            self.span = if self.improvements == 0 { (self.span * 2).min(self.capacity) }
                else if self.improvements * 4 >= self.capacity { (self.span / 2).max(2) } else { self.span };
            self.improvements = 0;
        }
        accepted
    }
}

pub(crate) fn builtin_acceptance<O: Clone + 'static, V: HeuristicValue>(
    spec: &ComponentSpec,
    codec: Option<Box<dyn SearchCodec<O>>>,
    horizon: u64,
) -> Result<Box<dyn AcceptancePolicy<O, V>>> {
    match spec.r#type.as_str() {
        "late" => {
            spec.validate_parameters(&["length"])?;
            let length = usize::try_from(spec.integer("length", 50)?).map_err(|_| invalid("late acceptance length overflow"))?;
            Ok(Box::new(LateAcceptance::new(codec.ok_or_else(|| invalid("late acceptance requires a lossless objective codec"))?, length)?))
        }
        "auto" => {
            spec.validate_parameters(&[])?;
            Ok(Box::new(AutoAcceptance::new(codec.ok_or_else(|| invalid("automatic acceptance requires a lossless objective codec"))?, horizon)?))
        }
        "record-to-record" => {
            spec.validate_parameters(&["initialRatio", "horizon"])?;
            Ok(Box::new(RecordToRecordAcceptance::new(spec.number("initialRatio", 0.05)?, spec.integer("horizon", 1000)?)?))
        }
        _ => Err(invalid("unknown acceptance policy")),
    }
}

#[cfg(test)]
mod tests;
