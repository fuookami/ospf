//! 共享父预算的持久化组合会话 / Persistent composite sessions sharing parent budgets

use std::collections::BTreeMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::model::callback::Solution;
use crate::solver::heuristic::{Individual, Iteration};
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::session::*;
use crate::solver::heuristic::session::archive::ObjectiveComponents;
use crate::solver::hyper_heuristic::memory::{NotWorseAcceptance, SearchMemory};
use crate::solver::hyper_heuristic::problem::{ProblemHandle, RandomHandle};
use crate::solver::hyper_heuristic::operators::PolicySession;
use crate::solver::hyper_heuristic::selection::{PolicySnapshot, SelectionTransition};
use crate::solver::hyper_heuristic::invalid;

/// 轮流推进岛屿或按阶段推进 memetic 子会话 / Advance islands in rotation or memetic child phases
pub struct CompositeSearchSession<O: ObjectiveComponents, V: HeuristicValue> {
    children: Vec<(String, Box<dyn PolicySession<O, V>>, RandomHandle)>,
    initialized: Vec<bool>,
    cursor: usize,
    completed: u64,
    sequential: bool,
    period: u64,
    problem: ProblemHandle<O, V>,
    memory: SearchMemory<O, V>,
    codec: Option<Box<dyn SearchCodec<Vec<Individual<O, V>>>>>,
    iteration: Iteration,
    closed: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompositeState {
    children: BTreeMap<String, SearchSessionSnapshot>,
    random: BTreeMap<String, i64>,
    initialized: Vec<bool>,
    cursor: usize,
    steps: u64,
    archive: JsonValue,
    working: JsonValue,
}

impl<O: ObjectiveComponents, V: HeuristicValue> CompositeSearchSession<O, V> {
    pub(crate) fn new(
        children: Vec<(String, Box<dyn PolicySession<O, V>>, RandomHandle)>,
        problem: ProblemHandle<O, V>,
        sequential: bool,
        period: u64,
        codec: Option<Box<dyn SearchCodec<Vec<Individual<O, V>>>>>,
    ) -> Result<Self> {
        if children.is_empty() || period == 0 || children.iter().any(|(_, session, _)| !session.supports_import()) {
            return Err(invalid("composites require import-capable children and a positive phase/migration period"));
        }
        let initialized = vec![false; children.len()];
        Ok(Self { children, initialized, cursor: 0, completed: 0, sequential, period, problem, memory: SearchMemory::local(100, 20),
            codec, iteration: Iteration::new(), closed: false })
    }
}

impl<O: ObjectiveComponents, V: HeuristicValue> SearchSession<O, V> for CompositeSearchSession<O, V> {
    fn supports_import(&self) -> bool { true }

    fn step(&mut self, slice: &mut SearchSlice) -> Result<SearchStepResult<O, V>> {
        if self.closed { return Err(invalid("composite session is closed")); }
        let start = slice.used();
        let parent = Arc::new(slice.clone());
        let mut report = SearchStepResult::empty(SearchTermination::SliceComplete);
        while report.steps < slice.steps() && slice.remaining() > 0 && !slice.expired() {
            let index = self.cursor;
            let migration = !self.sequential && self.completed > 0
                && self.completed % (self.period.saturating_mul(self.children.len() as u64)) == 0;
            let mut local = (*SearchSlice::child(&parent, slice.remaining(), 1)).clone();
            if self.initialized[index] && migration {
                let seeds = self.memory.working().iter().take(1).map(|value| value.solution.clone()).collect();
                let imported = self.children[index].1.import_solutions(seeds, &mut local)?;
                report.candidates.extend(imported.candidates);
                if imported.termination == SearchTermination::Failed {
                    report.termination = imported.termination; report.diagnostics.extend(imported.diagnostics); break;
                }
            }
            let child = self.children[index].1.step(&mut local)?;
            report.candidates.extend(child.candidates.iter().cloned());
            report.diagnostics.extend(child.diagnostics);
            report.steps += child.steps;
            if !self.initialized[index] && child.steps > 0 {
                self.initialized[index] = true;
                let seeds: Vec<_> = self.memory.working().iter().map(|value| value.solution.clone()).collect();
                if !seeds.is_empty() {
                    let imported = self.children[index].1.import_solutions(seeds, &mut local)?;
                    report.candidates.extend(imported.candidates);
                    report.diagnostics.extend(imported.diagnostics);
                }
            }
            self.memory.offer(&self.problem, child.candidates, &mut NotWorseAcceptance);
            if child.steps > 0 {
                self.completed += child.steps;
                self.iteration.next(true);
                if !self.sequential { self.cursor = (index + 1) % self.children.len(); }
                else if self.completed % self.period == 0 && index + 1 < self.children.len() { self.cursor = index + 1; }
            }
            if child.termination != SearchTermination::SliceComplete {
                report.termination = child.termination; break;
            }
            if child.steps == 0 { break; }
        }
        report.evaluations = slice.used().saturating_sub(start);
        if slice.expired() { report.termination = SearchTermination::TimeLimit; }
        else if slice.global().remaining() == 0 { report.termination = SearchTermination::EvaluationLimit; }
        Ok(report)
    }

    fn import_solutions(&mut self, solutions: Vec<Solution<V>>, slice: &mut SearchSlice) -> Result<SearchStepResult<O, V>> {
        if !self.initialized[self.cursor] { return Ok(SearchStepResult::empty(SearchTermination::SliceComplete)); }
        self.children[self.cursor].1.import_solutions(solutions, slice)
    }

    fn snapshot(&self) -> Result<SearchSessionSnapshot> {
        let codec = self.codec.as_ref().ok_or_else(|| invalid("composite snapshot requires a lossless archive codec"))?;
        let archive = codec.encode(&self.memory.archive().to_vec())?;
        let state = CompositeState {
            children: self.children.iter().map(|(id, session, _)| Ok((id.clone(), session.snapshot()?))).collect::<Result<_>>()?,
            random: self.children.iter().map(|(id, _, random)| (id.clone(), random.state())).collect(),
            initialized: self.initialized.clone(), cursor: self.cursor, steps: self.completed,
            archive: archive.clone(), working: codec.encode(&self.memory.working().to_vec())?,
        };
        Ok(SearchSessionSnapshot { version: 1, initialized: self.initialized.iter().any(|initialized| *initialized),
            iteration: self.iteration.iteration(), stagnation: self.iteration.not_better_iteration(), active_nanos: 0,
            state: serde_json::to_value(state).map_err(|error| invalid(error.to_string()))?, archive, constraints: JsonValue::Null })
    }

    fn restore(&mut self, snapshot: &SearchSessionSnapshot) -> Result<()> {
        let state: CompositeState = serde_json::from_value(snapshot.state.clone()).map_err(|error| invalid(error.to_string()))?;
        if snapshot.version != 1 || state.cursor >= self.children.len() || state.initialized.len() != self.children.len()
            || state.children.len() != self.children.len() || state.random.len() != self.children.len()
            || self.children.iter().any(|(id, _, _)| !state.children.contains_key(id) || !state.random.contains_key(id)) {
            return Err(invalid("incompatible composite snapshot"));
        }
        let codec = self.codec.as_ref().ok_or_else(|| invalid("missing composite codec"))?;
        let mut memory = SearchMemory::local(100, 20);
        memory.restore(&self.problem, codec.decode(&state.archive)?, codec.decode(&state.working)?)?;
        for (id, session, random) in &mut self.children {
            session.restore(&state.children[id])?;
            random.restore(state.random[id]);
        }
        self.initialized = state.initialized; self.cursor = state.cursor; self.completed = state.steps;
        self.iteration = Iteration::with_counts(snapshot.iteration, snapshot.stagnation);
        self.memory = memory;
        Ok(())
    }

    fn iteration(&self) -> &Iteration { &self.iteration }

    fn close(&mut self) -> Result<()> {
        if self.closed { return Ok(()); }
        self.closed = true;
        let mut error = None;
        for (_, session, _) in &mut self.children {
            if let Err(failure) = super::boundary(|| session.close()) { error = Some(failure); }
        }
        error.map_or(Ok(()), Err)
    }
}

impl<O: ObjectiveComponents, V: HeuristicValue> PolicySession<O, V> for CompositeSearchSession<O, V> {
    fn policies(&self) -> Result<BTreeMap<String, PolicySnapshot>> {
        let mut result = BTreeMap::new();
        for (id, session, _) in &self.children {
            for (slot, policy) in session.policies()? { result.insert(format!("{id}/{slot}"), policy); }
        }
        Ok(result)
    }

    fn restore_policies(&mut self, policies: &BTreeMap<String, PolicySnapshot>) -> Result<()> {
        if self.policies()?.keys().ne(policies.keys()) { return Err(invalid("incompatible composite policy slots")); }
        for (id, session, _) in &mut self.children {
            let prefix = format!("{id}/");
            let child = policies.iter().filter_map(|(slot, policy)| slot.strip_prefix(&prefix).map(|slot| (slot.into(), policy.clone()))).collect();
            session.restore_policies(&child)?;
        }
        Ok(())
    }

    fn drain_transitions(&mut self) -> Vec<SelectionTransition> {
        let mut result = Vec::new();
        for (id, session, _) in &mut self.children {
            for mut event in session.drain_transitions() {
                event.scope = format!("{id}/{}", event.scope);
                result.push(event);
            }
        }
        result
    }
}
