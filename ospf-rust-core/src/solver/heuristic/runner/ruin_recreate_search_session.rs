//! 可注册的排列破坏修复会话，完整目标统一计费 / Registrable permutation ruin/recreate session with metered complete objectives

use serde::{Deserialize, Serialize};
use crate::model::callback::Solution;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::neighborhood::ruin_recreate::RuinRecreateOperator;
use crate::solver::heuristic::result::{HeuristicError, Result};
use crate::solver::heuristic::session::json::{JsonValue, SearchCodec};
use crate::solver::heuristic::session::{SearchEncoding, SearchProblem, SearchRandom, SearchSession, SearchSessionSnapshot, SearchSlice, SearchStepResult, SearchTermination};

/// 业务增量估计闭包的显式状态协议，不保存闭包本身 / Explicit state protocol for domain incremental estimates, never serializing closures
pub trait EstimateState {
    /// 保存非缓存估计状态 / Save non-cache estimate state
    fn save(&self) -> Result<JsonValue>;
    /// 恢复估计状态 / Restore estimate state
    fn load(&mut self, state: &JsonValue) -> Result<()>;
}

/// 破坏修复的会话配置 / Ruin/recreate session configuration
pub struct RuinRecreateSessionConfig {
    /// 每步移除数量 / Removal count per step
    pub amount: usize,
    /// 独立可恢复随机流 / Independent restorable random stream
    pub random: SearchRandom,
    /// 显式声明业务估计无状态，否则快照要求estimate_state / Explicitly stateless domain estimates; otherwise snapshots require estimate_state
    pub stateless_estimates: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct State {
    current: JsonValue,
    callback_state: JsonValue,
    random: i64,
}

/// 持有独占问题边界的排列会话；问题回调状态沿现有外层协议保存 / Permutation session owning its problem boundary; problem callback state uses the existing outer protocol
pub struct RuinRecreateSearchSession<P, O: Clone, V: HeuristicValue> {
    problem: P,
    operator: RuinRecreateOperator<V>,
    config: RuinRecreateSessionConfig,
    estimate_state: Option<Box<dyn EstimateState>>,
    codec: Box<dyn SearchCodec<Individual<O, V>>>,
    current: Option<Individual<O, V>>,
    iteration: Iteration,
}

fn invalid(message: &str) -> HeuristicError { HeuristicError::InvalidArgument(message.into()) }

fn boundary<T>(callback: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(callback))
        .unwrap_or_else(|_| Err(HeuristicError::Model("破坏修复会话回调异常 / Ruin/recreate session callback panicked".into())))
}

impl<P: SearchProblem<O, V>, O: Clone, V: HeuristicValue> RuinRecreateSearchSession<P, O, V> {
    /// 创建会话；codec保存原业务目标，构造不调用目标 / Create a session; the codec retains original objectives, and construction never evaluates them
    pub fn new(
        problem: P,
        operator: RuinRecreateOperator<V>,
        codec: Box<dyn SearchCodec<Individual<O, V>>>,
        config: RuinRecreateSessionConfig,
    ) -> Result<Self> {
        if problem.encoding() != SearchEncoding::Permutation || config.amount > problem.variable_order().len() {
            return Err(invalid("ruin/recreate requires permutation encoding and a valid removal count"));
        }
        if problem.has_constraints() { return Err(invalid("ruin/recreate factory does not declare native constraint transactions")); }
        Ok(Self { problem, operator, config, codec, estimate_state: None, current: None, iteration: Iteration::with_counts(0, 0) })
    }

    /// 声明有状态估计的恢复实现 / Declare a recovery implementation for stateful estimates
    pub fn with_estimate_state(mut self, state: Box<dyn EstimateState>) -> Self { self.estimate_state = Some(state); self }

    /// 原问题引用，允许调用方按问题状态协议独立保存 / Reference to the original problem for independent persistence through the problem-state protocol
    pub fn problem(&self) -> &P { &self.problem }

    /// 原问题可变引用，用于按问题状态协议恢复而不重新评估 / Mutable original problem for restoration through its state protocol without reevaluation
    pub fn problem_mut(&mut self) -> &mut P { &mut self.problem }

    /// 当前完整已评估个体 / Current complete evaluated individual
    pub fn current(&self) -> Option<&Individual<O, V>> { self.current.as_ref() }

    fn offer(&mut self, candidate: &Individual<O, V>) {
        if self.current.as_ref().is_none_or(|previous| self.problem.compare_individuals(previous, candidate) != std::cmp::Ordering::Less) {
            self.current = Some(candidate.clone());
        }
    }
}

impl<P: SearchProblem<O, V>, O: Clone, V: HeuristicValue> SearchSession<O, V> for RuinRecreateSearchSession<P, O, V> {
    fn supports_import(&self) -> bool { true }

    fn step(&mut self, slice: &mut SearchSlice) -> Result<SearchStepResult<O, V>> {
        let start = slice.used();
        let mut report = SearchStepResult::empty(SearchTermination::SliceComplete);
        let outcome = boundary(|| -> Result<()> {
            while report.steps < slice.steps() && !slice.expired() {
                let solution = if let Some(previous) = &self.current {
                    let order = self.problem.variable_order();
                    let values = order.iter().map(|id| previous.solution.get(id).copied()).collect::<Option<Vec<_>>>()
                        .ok_or_else(|| invalid("incomplete current permutation"))?;
                    let values = self.operator.recreate(&values, self.config.amount, &mut self.config.random)?;
                    order.into_iter().zip(values).collect()
                } else {
                    let Some(solution) = boundary(|| Ok(self.problem.initial_solutions(1).into_iter().next()))? else {
                        report.termination = SearchTermination::NoCandidate; break;
                    };
                    solution
                };
                let evaluated = boundary(|| self.problem.evaluate(solution, slice))?;
                if !evaluated.admitted {
                    report.termination = SearchTermination::EvaluationLimit; break;
                }
                if let Some(candidate) = evaluated.candidate {
                    report.candidates.push(candidate.clone()); self.offer(&candidate);
                }
                self.iteration.next(true);
                report.steps += 1;
                if let Err(error) = boundary(|| { self.problem.flush(); Ok(()) }) {
                    report.termination = SearchTermination::Failed;
                    report.diagnostics.push(format!("模型刷新失败 / Model flush failed: {error}")); break;
                }
                if self.current.is_none() { report.termination = SearchTermination::NoCandidate; break; }
            }
            Ok(())
        });
        if let Err(error) = outcome {
            report.termination = SearchTermination::Failed; report.diagnostics.push(error.to_string());
        }
        report.evaluations = slice.used() - start;
        if slice.expired() && report.termination != SearchTermination::Failed { report.termination = SearchTermination::TimeLimit; }
        Ok(report)
    }

    fn import_solutions(&mut self, solutions: Vec<Solution<V>>, slice: &mut SearchSlice) -> Result<SearchStepResult<O, V>> {
        let start = slice.used();
        let mut report = SearchStepResult::empty(SearchTermination::SliceComplete);
        let outcome = boundary(|| -> Result<()> {
            for solution in solutions {
                let evaluated = boundary(|| self.problem.evaluate(solution, slice))?;
                if !evaluated.admitted { report.termination = SearchTermination::EvaluationLimit; break; }
                if let Some(candidate) = evaluated.candidate { report.candidates.push(candidate.clone()); self.offer(&candidate); }
            }
            Ok(())
        });
        if let Err(error) = outcome {
            report.termination = SearchTermination::Failed; report.diagnostics.push(error.to_string());
        }
        report.evaluations = slice.used() - start;
        if slice.expired() && report.termination != SearchTermination::Failed { report.termination = SearchTermination::TimeLimit; }
        Ok(report)
    }

    fn snapshot(&self) -> Result<SearchSessionSnapshot> {
        if !self.config.stateless_estimates && self.estimate_state.is_none() { return Err(invalid("domain estimates must declare statelessness or provide recovery")); }
        let current = self.current.as_ref().map(|individual| self.codec.encode(individual)).transpose()?.unwrap_or(JsonValue::Null);
        let callback_state = if let Some(state) = &self.estimate_state { boundary(|| state.save())? } else { JsonValue::Null };
        let state = serde_json::to_value(State { current, callback_state, random: self.config.random.state() as i64 })
            .map_err(|_| invalid("invalid ruin session state"))?;
        Ok(SearchSessionSnapshot { version: 1, initialized: self.current.is_some(), iteration: self.iteration.iteration(), stagnation: 0,
            state, archive: JsonValue::Array(Vec::new()), constraints: JsonValue::Null, active_nanos: 0 })
    }

    fn restore(&mut self, snapshot: &SearchSessionSnapshot) -> Result<()> {
        if snapshot.version != 1 || snapshot.stagnation != 0 || !snapshot.constraints.is_null()
            || snapshot.archive != JsonValue::Array(Vec::new()) || !self.config.stateless_estimates && self.estimate_state.is_none() {
            return Err(invalid("incompatible ruin/recreate snapshot"));
        }
        let state: State = serde_json::from_value(snapshot.state.clone()).map_err(|_| invalid("invalid ruin/recreate state"))?;
        let current = if state.current.is_null() { None } else { Some(self.codec.decode(&state.current)?) };
        if snapshot.initialized != current.is_some() || current.as_ref().is_some_and(|entry| {
            !self.problem.valid_solution(&entry.solution) || !self.problem.admissible(&entry.solution) || !self.problem.valid_objective(&entry.fitness)
        }) { return Err(invalid("invalid restored ruin/recreate candidate or initialization")); }
        if let Some(callbacks) = &mut self.estimate_state {
            let before = boundary(|| callbacks.save())?;
            if let Err(error) = boundary(|| callbacks.load(&state.callback_state)) {
                boundary(|| callbacks.load(&before))?; return Err(error);
            }
        } else if !state.callback_state.is_null() { return Err(invalid("unexpected estimate state")); }
        self.current = current;
        self.iteration = Iteration::with_counts(snapshot.iteration, 0);
        self.config.random = SearchRandom::new(state.random as u64);
        Ok(())
    }

    fn iteration(&self) -> &Iteration { &self.iteration }
}

#[cfg(test)]
mod tests;
