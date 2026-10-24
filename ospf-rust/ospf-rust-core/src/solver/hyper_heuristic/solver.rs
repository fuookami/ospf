//! 超启发式运行生命周期 / Hyper-heuristic run lifecycle

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};
use crate::model::callback::Solution;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::session::{ArchiveCodec, SearchBudget, SearchProblem, SearchRandom, SearchSlice, SearchStepResult, SearchTermination};
use crate::solver::heuristic::session::archive::ObjectiveComponents;
use crate::solver::hyper_heuristic::config::*;
use crate::solver::hyper_heuristic::components::{RewardContext, RuntimeComponents, SearchProgress, SliceAllocation};
use crate::solver::hyper_heuristic::memory::SearchMemory;
use crate::solver::hyper_heuristic::operators::PolicySession;
use crate::solver::hyper_heuristic::problem::{ProblemHandle, RandomHandle};
use crate::solver::hyper_heuristic::persistence::{PolicyArchive, RunCheckpoint, WorkerCheckpoint, FileSearchStore, SearchStore, fingerprint};
use crate::solver::hyper_heuristic::registry::{AlgorithmContext, SearchRegistry};
use crate::solver::hyper_heuristic::selection::*;
use crate::solver::hyper_heuristic::schedule::{ScheduleControl, ScheduleEvent, ScheduleLog};
use crate::solver::hyper_heuristic::{boundary, invalid};
use crate::variable::VariableId;

/// 搜索结果，多目标不伪造唯一最优 / Search result without fabricating a unique multi-objective optimum
#[derive(Debug, Clone)]
pub struct SearchResult<O, V> {
    /// 有效解档案 / Valid solution archive
    pub archive: Vec<Individual<O, V>>,
    /// 是否多目标 / Whether multi-objective
    pub multi_objective: bool,
    /// 终止原因 / Termination reason
    pub termination: SearchTermination,
    /// 评估请求次数 / Evaluation requests
    pub evaluations: u64,
    /// 实际目标调用次数 / Actual objective calls
    pub objective_calls: u64,
    /// 完成高层决策数 / Completed high-level decisions
    pub decisions: u64,
    /// 完整低层步数 / Complete low-level steps
    pub steps: u64,
    /// 有效运行时间，纳秒 / Active runtime in nanoseconds
    pub active_nanos: u64,
    /// 失败诊断 / Failure diagnostics
    pub diagnostics: Vec<String>,
}

impl<O, V> SearchResult<O, V> {
    /// 单目标最优；多目标为空 / Scalar optimum; absent for multi-objective search
    pub fn best(&self) -> Option<&Individual<O, V>> {
        if self.multi_objective { None } else { self.archive.first() }
    }
}

/// 可复用求解定义，每次 open 创建独立运行 / Reusable solver definition creating an independent run on each open
pub struct HyperHeuristicSolver<O, V> {
    config: HyperHeuristicConfig,
    registry: SearchRegistry<O, V>,
}

/// 可选恢复输入，策略与检查点互斥 / Optional restore inputs; policy and checkpoint are mutually exclusive
#[derive(Default)]
pub struct RunOptions<'a> {
    /// 完整实例恢复 / Complete instance recovery
    pub checkpoint: Option<&'a RunCheckpoint>,
    /// 仅迁移学习策略 / Transfer only learned policy
    pub policy: Option<&'a PolicyArchive>,
}

impl<O: ObjectiveComponents + 'static, V: HeuristicValue> HyperHeuristicSolver<O, V> {
    /// 绑定配置与显式组件注册表 / Bind configuration and explicit component registry
    pub fn new(config: HyperHeuristicConfig, registry: SearchRegistry<O, V>) -> Result<Self> {
        config.validate()?;
        Ok(Self { config, registry })
    }

    /// 创建独立运行，不共享问题回调或缓存 / Create an independent run without sharing callbacks or caches
    pub fn open(&self) -> Result<HyperHeuristicRun<O, V>> {
        self.open_with(RunOptions::default())
    }

    /// 创建独立运行并显式恢复策略或检查点 / Create an independent run and explicitly restore policy or checkpoint
    pub fn open_with(&self, options: RunOptions<'_>) -> Result<HyperHeuristicRun<O, V>> {
        if options.checkpoint.is_some() && options.policy.is_some() { return Err(invalid("checkpoint and policy are mutually exclusive")); }
        let mut run = self.open_fresh()?;
        boundary(|| {
            if let Some(policy) = options.policy { run.restore_policy(policy)?; }
            if let Some(checkpoint) = options.checkpoint { run.restore_checkpoint(checkpoint)?; }
            Ok(())
        })?;
        Ok(run)
    }

    fn open_fresh(&self) -> Result<HyperHeuristicRun<O, V>> {
        let config = self.config.clone();
        config.validate()?;
        let threaded = config.execution.mode != ExecutionMode::Serial && config.execution.parallelism > 1;
        let components = boundary(|| self.registry.components.instantiate(&config, self.registry.objective_codec()))?;
        if config.persistence.enabled { boundary(|| components.save())?; }
        let random = SearchRandom::new(config.execution.seed as u64);
        let problem = self.registry.problem(&config.problem, random.clone())?;
        boundary(|| components.reward.validate_mode(problem.multi_objective(), problem.has_objective_coordinates()))?;
        boundary(|| components.acceptance.validate_problem(&problem))?;
        let archive_codec = self.registry.archive_codec(problem.variable_order());
        if config.persistence.enabled {
            if archive_codec.is_none() { return Err(invalid("complete recovery requires a lossless archive codec")); }
            boundary(|| problem.save_state())?;
        }
        let mut workers = Vec::new();
        let (completed, completions) = mpsc::channel();
        let mut algorithms: Vec<_> = config.algorithms.iter().collect();
        algorithms.sort_by(|lhs, rhs| lhs.id.cmp(&rhs.id));
        for algorithm in algorithms {
            let stream = random.split(&format!("algorithm:{}", algorithm.id));
            let (worker_problem, session, stream) = if threaded {
                super::parallel::spawn(self.registry.clone(), config.problem.clone(), algorithm.clone(), stream,
                    config.learning.mode != LearningMode::Frozen, config.learning.reward.clone(), completed.clone())?
            } else {
                let worker_problem = self.registry.problem(&config.problem, stream.clone())?;
                let stream = RandomHandle::new(stream);
                let session = self.registry.session_configured(AlgorithmContext {
                    config: algorithm.clone(), problem: worker_problem.clone(), random: stream.clone(),
                }, config.learning.mode != LearningMode::Frozen, &config.learning.reward)?;
                (worker_problem, session, stream)
            };
            if worker_problem.fingerprint() != problem.fingerprint() || worker_problem.variable_order().len() != problem.variable_order().len()
                || worker_problem.multi_objective() != problem.multi_objective() || worker_problem.encoding() != problem.encoding()
                || worker_problem.has_objective_coordinates() != problem.has_objective_coordinates()
                || worker_problem.has_constraint_violation() != problem.has_constraint_violation() {
                return Err(invalid("independent problem factories produced incompatible definitions"));
            }
            if config.persistence.enabled {
                boundary(|| worker_problem.save_state())?;
                boundary(|| session.snapshot())?;
            }
            if (components.exchange.requires_import() || algorithm.initialization == InitializationMode::WorkingSet) && !session.supports_import() {
                return Err(invalid("working-set initialization or exchange requires native import support"));
            }
            workers.push(Worker { id: algorithm.id.clone(), problem: worker_problem, random: stream,
                session, initialized: false, disabled: false, closed: false, initialization: algorithm.initialization });
        }
        workers.sort_by(|lhs, rhs| lhs.id.cmp(&rhs.id));
        let ids = config.algorithms.iter().map(|algorithm| algorithm.id.clone()).collect();
        if config.learning.features.r#type == "progress" && config.selection.integer("states", 64)? < 64 {
            return Err(invalid("progress features require at least 64 states"));
        }
        let policy = boundary(|| self.registry.components.selector(&config.selection, ids, config.learning.mode != LearningMode::Frozen))?;
        Ok(HyperHeuristicRun {
            memory: SearchMemory::new(config.memory.archive_capacity, config.memory.working_capacity),
            budget: Arc::new(SearchBudget::new(config.budget.evaluations.unwrap_or(u64::MAX))),
            config, problem, workers, policy, random, stopped: Arc::new(AtomicBool::new(false)), observed_stopped: false,
            decisions: 0, dispatched: 0, accounted: 0, steps: 0, stagnation: 0, successes: 0, last_cost: 0,
            active: Duration::ZERO, active_start: None, diagnostics: Vec::new(), transitions: Vec::new(), closed: false,
            components,
            archive_codec,
            completions: if threaded { Some(completions) } else { None },
            pending: BTreeMap::new(),
            ready_completions: BTreeSet::new(), schedule: ScheduleControl::default(),
        })
    }

    /// 创建、运行并清理独立会话 / Create, run, and clean up an independent session
    pub fn solve(&self) -> Result<SearchResult<O, V>> {
        let mut run = self.open()?;
        let result = run.run();
        let closed = run.close();
        match result { Err(error) => Err(error), Ok(result) => { closed?; Ok(result) } }
    }
}

struct Worker<O: Clone, V: HeuristicValue> {
    id: String,
    problem: ProblemHandle<O, V>,
    random: RandomHandle,
    session: Box<dyn PolicySession<O, V>>,
    initialized: bool,
    disabled: bool,
    closed: bool,
    initialization: InitializationMode,
}

struct Dispatch<O, V> {
    index: usize,
    action: String,
    before: SelectionContext,
    available: Vec<String>,
    revision: u64,
    reference: Vec<Individual<O, V>>,
    credit: RewardContext,
    elapsed_nanos: u64,
    local_order: Vec<VariableId>,
    order: Vec<VariableId>,
    steps: u64,
    slice: SearchSlice,
    seeds: Vec<Solution<V>>,
    decision: u64,
    previous_action: Option<String>,
}

impl<O: Clone, V: HeuristicValue> Drop for Worker<O, V> {
    fn drop(&mut self) {
        if !self.closed { let _ = boundary(|| self.session.close()); }
    }
}

/// 独立的运行时状态，可逐次推进 / Independent runtime state supporting incremental advancement
pub struct HyperHeuristicRun<O: ObjectiveComponents, V: HeuristicValue> {
    config: HyperHeuristicConfig,
    problem: ProblemHandle<O, V>,
    workers: Vec<Worker<O, V>>,
    policy: Box<dyn SelectionPolicy>,
    random: SearchRandom,
    memory: SearchMemory<O, V>,
    budget: Arc<SearchBudget>,
    stopped: Arc<AtomicBool>,
    observed_stopped: bool,
    decisions: u64,
    dispatched: u64,
    accounted: u64,
    steps: u64,
    stagnation: u64,
    successes: u64,
    last_cost: u64,
    active: Duration,
    active_start: Option<Instant>,
    diagnostics: Vec<String>,
    transitions: Vec<SelectionTransition>,
    closed: bool,
    components: RuntimeComponents<O, V>,
    archive_codec: Option<ArchiveCodec<O, V>>,
    completions: Option<Receiver<String>>,
    pending: BTreeMap<String, Dispatch<O, V>>,
    ready_completions: BTreeSet<String>,
    schedule: ScheduleControl,
}

impl<O: ObjectiveComponents + 'static, V: HeuristicValue> HyperHeuristicRun<O, V> {
    fn elapsed(&self) -> Duration {
        self.active + self.active_start.map(|start| start.elapsed()).unwrap_or_default()
    }

    fn pause_if_idle(&mut self) {
        if self.pending.is_empty() {
            self.active = self.elapsed();
            self.active_start = None;
        }
    }

    fn termination(&self) -> Option<SearchTermination> {
        // 同步归并不能观察下一 worker 的并发计费，否则 terminal 反馈随线程速度变化。
        // Ordered synchronous merging must not observe another worker's racing charges in terminal feedback.
        let committed_budget = self.schedule.enabled() || self.config.execution.mode == ExecutionMode::Synchronous;
        if !self.diagnostics.is_empty() { Some(SearchTermination::Failed) }
        else if (if self.schedule.enabled() { self.observed_stopped } else { self.stopped.load(Ordering::Relaxed) }) || self.closed { Some(SearchTermination::Stopped) }
        else if committed_budget && self.accounted >= self.budget.limit()
            || !committed_budget && self.budget.remaining() == 0 { Some(SearchTermination::EvaluationLimit) }
        else if self.steps >= self.config.budget.steps.unwrap_or(u64::MAX) || self.dispatched >= self.config.budget.decisions.unwrap_or(u64::MAX) { Some(SearchTermination::StepLimit) }
        else if self.config.budget.time_millis.is_some_and(|limit| self.elapsed() >= Duration::from_millis(limit)) { Some(SearchTermination::TimeLimit) }
        else if self.workers.iter().all(|worker| worker.disabled) { Some(SearchTermination::NoCandidate) }
        else { None }
    }

    fn context(&self) -> Result<SelectionContext> {
        self.context_at(self.dispatched)
    }

    fn context_at(&self, decision: u64) -> Result<SelectionContext> {
        let mut unique = Vec::new();
        for value in self.memory.working() {
            if !unique.contains(&&value.solution) { unique.push(&value.solution); }
        }
        let context = self.components.features.extract(&SearchProgress {
            decision, evaluations: self.accounted, evaluation_limit: self.config.budget.evaluations,
            stagnation: self.stagnation, diversity: unique.len() as f64 / self.memory.working().len().max(1) as f64,
            success_ratio: self.successes as f64 / self.decisions.max(1) as f64,
            cost_ratio: self.last_cost as f64 / self.config.budget.slice_evaluations as f64,
        })?;
        if context.features.iter().any(|value| !value.is_finite()) { return Err(invalid("non-finite search features")); }
        Ok(context)
    }

    /// 请求协作式停止，不中断阻塞的业务回调 / Request cooperative stopping without interrupting blocking callbacks
    pub fn request_stop(&self) { self.stopped.store(true, Ordering::Relaxed); }

    /// 可跨线程发送停止信号的句柄 / Handle for signaling stop from another thread
    pub fn stop_token(&self) -> Arc<AtomicBool> { self.stopped.clone() }

    /// 获取当前结果，不触发目标评估 / Inspect the current result without evaluating objectives
    pub fn result(&self) -> SearchResult<O, V> {
        SearchResult { archive: self.memory.archive().to_vec(), multi_objective: self.problem.multi_objective(),
            termination: self.termination().unwrap_or(SearchTermination::SliceComplete), evaluations: self.budget.used(),
            objective_calls: self.budget.objective_calls(), decisions: self.decisions, steps: self.steps,
            active_nanos: self.elapsed().as_nanos().min(u64::MAX as u128) as u64, diagnostics: self.diagnostics.clone() }
    }

    /// 已完成的学习轨迹 / Completed learning transitions
    pub fn transitions(&self) -> &[SelectionTransition] { &self.transitions }

    fn bind_schedule(&mut self, source: Option<&ScheduleLog>) -> Result<()> {
        if self.config.execution.mode != ExecutionMode::Asynchronous || self.completions.is_none()
            || self.config.budget.time_millis.is_some() || self.components.reward.requires_elapsed_time() || !self.pending.is_empty() || self.closed {
            return Err(invalid("schedule control requires quiescent asynchronous threaded execution without time budgets or elapsed-time rewards"));
        }
        if self.dispatched != 0 && !self.schedule.enabled() { return Err(invalid("enable schedule control before dispatching")); }
        self.schedule.bind(fingerprint(&checkpoint_configuration(&self.config)?), format!("{}:{}", self.problem.id(), self.problem.fingerprint()), source)
    }

    /// 开启或显式继续录制；只支持无时间预算的异步线程模式 / Start or explicitly resume recording; asynchronous threaded mode without time budgets only
    pub fn record_schedule(&mut self) -> Result<()> { self.bind_schedule(None) }

    /// 绑定完整源日志，重新执行算法并校验已消费前缀 / Bind a complete source log, re-execute algorithms and validate the consumed prefix
    pub fn replay_schedule(&mut self, source: &ScheduleLog) -> Result<()> { self.bind_schedule(Some(source)) }

    /// 导出已终止运行的完整调度日志 / Export the complete schedule of a terminated run
    pub fn export_schedule(&mut self) -> Result<ScheduleLog> {
        self.quiesce()?; self.finish_schedule()?; self.schedule.export()
    }

    fn finish_schedule(&mut self) -> Result<()> {
        self.observe_stop();
        if self.schedule.enabled() && !self.schedule.ended() && self.pending.is_empty() && self.termination().is_some() {
            self.schedule.event(ScheduleEvent { kind: "end".into(), evaluations: self.accounted, steps: self.steps,
                stopped: self.observed_stopped, failed: !self.diagnostics.is_empty(), ..Default::default() })?;
        }
        Ok(())
    }

    fn observe_stop(&mut self) {
        // 只在录制事件边界采样外部停止，避免归并过程中发生未记录的竞态。
        // Sample external stopping only at recorded event boundaries, avoiding unlogged races during merging.
        self.observed_stopped |= self.stopped.load(Ordering::Relaxed)
            || self.schedule.replaying() && self.schedule.peek().is_some_and(|event| event.stopped && matches!(event.kind.as_str(), "complete" | "drain" | "end"));
    }

    /// 排空在途结果后导出可迁移策略 / Drain in-flight outcomes before exporting transferable policy
    pub fn export_policy(&mut self) -> Result<PolicyArchive> {
        self.quiesce()?;
        self.policy_archive()
    }

    fn policy_archive(&self) -> Result<PolicyArchive> {
        let definitions = self.policy_definitions()?;
        let operators = self.workers.iter().map(|worker| Ok((worker.id.clone(), worker.session.policies()?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        Ok(PolicyArchive { version: 1, definition: fingerprint(&definitions), feature_version: self.components.features.version().into(),
            root: self.policy.snapshot()?, operators, metadata: BTreeMap::new(), definitions })
    }

    /// 在尚未推进的运行上导入策略 / Import a policy before advancing a run
    pub fn restore_policy(&mut self, policy: &PolicyArchive) -> Result<()> {
        if self.dispatched > 0 { return Err(invalid("load policy before advancing")); }
        if policy.version != 1 || policy.feature_version != self.components.features.version() || policy.definition != fingerprint(&self.policy_definitions()?)
            || policy.definition != fingerprint(&policy.definitions) {
            return Err(invalid("incompatible learned policy definition"));
        }
        let expected = self.policy_archive()?.operators;
        if expected.keys().ne(policy.operators.keys()) { return Err(invalid("incompatible operator policy algorithms")); }
        self.policy.restore(&policy.root)?;
        for worker in &mut self.workers {
            if let Some(policies) = policy.operators.get(&worker.id) { worker.session.restore_policies(policies)?; }
        }
        Ok(())
    }

    fn policy_definitions(&self) -> Result<serde_json::Value> {
        let mut algorithms = self.config.algorithms.clone();
        algorithms.sort_by(|lhs, rhs| lhs.id.cmp(&rhs.id));
        let mut selection = serde_json::to_value(&self.config.selection).map_err(|error| invalid(error.to_string()))?;
        let mut features = serde_json::to_value(&self.config.learning.features).map_err(|error| invalid(error.to_string()))?;
        let mut reward = serde_json::to_value(&self.config.learning.reward).map_err(|error| invalid(error.to_string()))?;
        compact_component(&mut selection); compact_component(&mut features); compact_component(&mut reward);
        Ok(serde_json::json!({ "algorithms": algorithms.iter().map(compact_algorithm).collect::<Result<Vec<_>>>()?,
            "selection": selection, "features": features, "reward": reward }))
    }

    /// 排空在途结果并保存全部实例状态 / Drain in-flight outcomes and capture all instance state
    pub fn checkpoint(&mut self) -> Result<RunCheckpoint> {
        if !self.config.persistence.enabled || self.closed { return Err(invalid("complete recovery is not enabled or run is closed")); }
        self.quiesce()?;
        self.finish_schedule()?;
        let codec = self.archive_codec.as_ref().ok_or_else(|| invalid("missing lossless archive codec"))?;
        boundary(|| {
            let configuration_data = checkpoint_configuration(&self.config)?;
            let workers = self.workers.iter().map(|worker| Ok((worker.id.clone(), WorkerCheckpoint {
                session: worker.session.snapshot()?, random: worker.random.state(), initialized: worker.initialized,
                disabled: worker.disabled, problem_state: worker.problem.save_state()?,
            }))).collect::<Result<_>>()?;
            Ok(RunCheckpoint { version: 1, schedule: self.schedule.snapshot()?, configuration: fingerprint(&configuration_data), problem: format!("{}:{}", self.problem.id(), self.problem.fingerprint()),
                policy: self.policy_archive()?, workers, archive: codec.encode(&self.memory.archive().to_vec())?,
                working: codec.encode(&self.memory.working().to_vec())?, evaluations: self.budget.used(),
                dispatched: self.dispatched, completed: self.decisions, steps: self.steps, stagnation: self.stagnation,
                active_nanos: self.active.as_nanos().min(u64::MAX as u128) as u64, random: self.random.state() as i64,
                diagnostics: self.diagnostics.clone(), problem_state: self.problem.save_state()?, objective_calls: self.budget.objective_calls(),
                transitions: self.transitions.clone(), successes: self.successes, last_cost: self.last_cost,
                configuration_data, components: self.components.save()? })
        })
    }

    fn restore_checkpoint(&mut self, saved: &RunCheckpoint) -> Result<()> {
        if let Some(schedule) = &saved.schedule {
            let completions: Vec<_> = schedule.log.events.iter().filter(|event| event.kind == "complete").collect();
            if self.config.execution.mode != ExecutionMode::Asynchronous || self.completions.is_none() || self.config.budget.time_millis.is_some()
                || schedule.log.configuration != saved.configuration || schedule.log.problem != saved.problem
                || schedule.log.events.iter().filter(|event| event.kind == "dispatch").count() as u64 != saved.dispatched
                || completions.len() as u64 != saved.completed
                || completions.iter().try_fold(0_u64, |sum, event| sum.checked_add(event.evaluations)) != Some(saved.evaluations)
                || completions.iter().try_fold(0_u64, |sum, event| sum.checked_add(event.steps)) != Some(saved.steps) {
                return Err(invalid("inconsistent checkpoint schedule identity or counters"));
            }
        }
        self.schedule.restore(saved.schedule.as_ref())?;
        if saved.schedule.as_ref().and_then(|saved| saved.log.events.last()).is_some_and(|event| event.kind == "end" && event.stopped) {
            self.stopped.store(true, Ordering::Relaxed);
            self.observed_stopped = true;
        }
        let configuration = checkpoint_configuration(&self.config)?;
        if !self.config.persistence.enabled || saved.version != 1 || saved.configuration != fingerprint(&configuration)
            || saved.configuration != fingerprint(&saved.configuration_data) || saved.problem != format!("{}:{}", self.problem.id(), self.problem.fingerprint())
            || saved.dispatched != saved.completed || saved.evaluations > self.budget.limit() || saved.objective_calls > saved.evaluations
            || saved.steps > self.config.budget.steps.unwrap_or(u64::MAX) || saved.completed > self.config.budget.decisions.unwrap_or(u64::MAX)
            || saved.successes > saved.completed || saved.last_cost > saved.evaluations
            || saved.workers.len() != self.workers.len() || self.workers.iter().any(|worker| !saved.workers.contains_key(&worker.id)) {
            return Err(invalid("incompatible or invalid checkpoint"));
        }
        let completed: Vec<_> = saved.transitions.iter().filter(|event| event.scope == "algorithm").collect();
        let mut sequence: Vec<_> = completed.iter().map(|event| event.decision).collect();
        sequence.sort_unstable();
        let evaluations = completed.iter().try_fold(0_u64, |sum, event| sum.checked_add(event.evaluations));
        let steps = completed.iter().try_fold(0_u64, |sum, event| sum.checked_add(event.steps));
        if completed.len() as u64 != saved.completed || sequence.iter().enumerate().any(|(index, value)| *value != index as u64)
            || evaluations != Some(saved.evaluations) || steps != Some(saved.steps) || saved.stagnation > saved.completed
            || completed.iter().any(|event| !event.available.contains(&event.action) || !saved.workers.contains_key(&event.action)
                || !event.reward.is_finite() || event.before.features.iter().chain(&event.after.features).any(|value| !value.is_finite())) {
            return Err(invalid("inconsistent checkpoint transitions or counters"));
        }
        let codec = self.archive_codec.as_ref().ok_or_else(|| invalid("missing archive codec"))?;
        let archive = codec.decode(&saved.archive)?;
        let working = codec.decode(&saved.working)?;
        self.problem.load_state(&saved.problem_state)?;
        let mut memory = SearchMemory::new(self.config.memory.archive_capacity, self.config.memory.working_capacity);
        memory.restore(&self.problem, archive, working)?;
        self.restore_policy(&saved.policy)?;
        // 校验回调可能有状态；重置校验造成的副作用 / Validation callbacks may be stateful; reset their validation side effects
        self.problem.load_state(&saved.problem_state)?;
        for worker in &mut self.workers {
            let state = &saved.workers[&worker.id];
            if state.initialized != state.session.initialized { return Err(invalid("inconsistent worker initialization state")); }
            worker.problem.load_state(&state.problem_state)?;
            worker.session.restore(&state.session)?;
            // 原生状态校验也可能访问有状态回调，恢复后消除校验副作用。
            // Native-state validation may call stateful callbacks; reset its validation side effects afterward.
            worker.problem.load_state(&state.problem_state)?;
            worker.random.restore(state.random);
            worker.initialized = state.initialized;
            worker.disabled = state.disabled;
        }
        self.components.load(&saved.components)?;
        self.memory = memory;
        self.budget = Arc::new(SearchBudget::with_counts(self.budget.limit(), saved.evaluations, saved.objective_calls));
        self.random = SearchRandom::new(saved.random as u64);
        self.decisions = saved.completed; self.dispatched = saved.dispatched; self.accounted = saved.evaluations;
        self.steps = saved.steps; self.stagnation = saved.stagnation;
        self.successes = saved.successes; self.last_cost = saved.last_cost;
        self.active = Duration::from_nanos(saved.active_nanos);
        self.transitions = saved.transitions.clone(); self.diagnostics = saved.diagnostics.clone();
        Ok(())
    }

    /// 执行同步批次或归并一个异步完成事件 / Execute a synchronous batch or merge one asynchronous completion
    pub fn advance(&mut self) -> Result<bool> {
        if self.closed { return Err(invalid("run is closed")); }
        self.schedule.ready()?;
        self.observe_stop();
        while self.schedule.replaying() && self.schedule.peek().is_some_and(|event| event.kind == "drain") {
            if self.schedule.peek().is_some_and(|event| event.stopped) { self.request_stop(); }
            self.quiesce()?;
        }
        if self.schedule.replaying() && self.schedule.peek().is_some_and(|event| event.kind == "end" && event.stopped) { self.request_stop(); }
        if self.termination().is_some() {
            self.quiesce()?;
            self.finish_schedule()?;
            return Ok(false);
        }
        self.schedule.event(ScheduleEvent { kind: "begin".into(), ..Default::default() })?;
        self.active_start.get_or_insert_with(Instant::now);
        let result = boundary(|| self.advance_inner());
        if let Err(error) = &result && self.diagnostics.is_empty() {
            self.diagnostics.push(error.to_string());
        }
        // 失败和终止仍需恰好一次归并所有已派发片段 / Failure and termination still merge every dispatched slice exactly once
        let drained = if result.is_err() || self.termination().is_some() { self.drain_pending() } else { Ok(()) };
        self.pause_if_idle();
        self.schedule.event(ScheduleEvent { kind: "yield".into(), ..Default::default() })?;
        self.finish_schedule()?;
        result?;
        drained?;
        if self.config.persistence.every_decisions > 0 && self.decisions.is_multiple_of(self.config.persistence.every_decisions)
            && let Some(destination) = self.config.persistence.checkpoint_path.clone()
        {
            let path = Path::new(&destination);
            let key = path.file_name().and_then(|name| name.to_str()).ok_or_else(|| invalid("invalid checkpoint destination"))?;
            let parent = path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
            FileSearchStore::new(parent)?.save_checkpoint(key, &self.checkpoint()?)?;
        }
        Ok(self.termination().is_none() || !self.pending.is_empty())
    }

    fn prepare(
        &mut self,
        available: Vec<String>,
        allowance: u64,
        steps: u64,
        decision: u64,
    ) -> Result<Dispatch<O, V>> {
        let before = self.context_at(decision)?;
        let action = match boundary(|| self.policy.select(&before, &available, &mut self.random)) {
            Ok(action) => action,
            Err(error) => { self.policy.discard(decision); return Err(error); }
        };
        let prepared = boundary(|| {
            let index = self.workers.iter().position(|worker| worker.id == action && !worker.disabled)
                .ok_or_else(|| invalid("selector returned a masked action"))?;
            if !available.contains(&action) { return Err(invalid("selector returned an unavailable action")); }
            let worker = &self.workers[index];
            let local_order = worker.problem.variable_order();
            let order = self.problem.variable_order();
            let mut slice = SearchSlice::new(self.budget.clone(), allowance, steps);
            if let Some(limit) = self.config.budget.time_millis {
                slice = slice.with_duration(Duration::from_millis(limit).saturating_sub(self.elapsed()));
            }
            let amount = self.components.exchange.amount(self.decisions)?;
            let seeds = if (!worker.initialized && worker.initialization == InitializationMode::WorkingSet) || amount > 0 {
                self.memory.working().iter().take(if worker.initialized { amount } else { usize::MAX })
                    .map(|value| remap(&value.solution, &order, &local_order)).collect::<Result<_>>()?
            } else { Vec::new() };
            let credit = RewardContext::default().with_reference(&self.problem, self.memory.archive())?;
            let previous_action = self.policy.predecessor(decision);
            Ok(Dispatch { index, action, before, available, revision: self.policy.revision(), reference: self.memory.archive().to_vec(), credit, elapsed_nanos: 0,
                local_order, order, steps, slice, seeds, decision, previous_action })
        });
        if prepared.is_err() { self.policy.discard(decision); }
        prepared
    }

    fn merge(
        &mut self,
        dispatch: Dispatch<O, V>,
        outcome: Result<SearchStepResult<O, V>>,
        batch_active: &[String],
    ) -> Result<()> {
        let decision = dispatch.decision;
        let result = boundary(|| self.merge_inner(dispatch, outcome, batch_active));
        self.policy.discard(decision);
        result
    }

    fn merge_inner(
        &mut self,
        dispatch: Dispatch<O, V>,
        outcome: Result<SearchStepResult<O, V>>,
        batch_active: &[String],
    ) -> Result<()> {
        let Dispatch { index, action, before, available, revision, reference, credit, elapsed_nanos, local_order, order, steps, slice, decision, previous_action, .. } = dispatch;
        let mut report = match outcome {
            Ok(report) => report,
            Err(error) => { let mut report = SearchStepResult::empty(SearchTermination::Failed); report.diagnostics.push(error.to_string()); report },
        };
        let worker = &mut self.workers[index];
        if report.steps > steps {
            report.steps = 0;
            report.termination = SearchTermination::Failed;
            report.diagnostics.push("session exceeded its step allowance".into());
        }
        worker.initialized |= report.steps > 0;
        report.candidates.retain_mut(|candidate| {
            match remap(&candidate.solution, &local_order, &order) {
                Ok(solution) => { candidate.solution = solution; true }
                Err(error) => { report.diagnostics.push(error.to_string()); false }
            }
        });
        let mut failed = report.termination == SearchTermination::Failed || !report.diagnostics.is_empty();
        let consumed = slice.used();
        let operator_transitions = worker.session.drain_transitions();
        self.decisions += 1;
        self.accounted += consumed;
        self.steps += report.steps;
        self.last_cost = consumed;
        if failed {
            if report.diagnostics.is_empty() { report.diagnostics.push(format!("worker {action} failed")); }
            self.diagnostics.extend(report.diagnostics.iter().cloned());
        }
        if failed || report.termination == SearchTermination::NoCandidate || (report.steps == 0 && consumed == 0) { worker.disabled = true; }
        // 仅步数限额不能终止永远无法提交一代的片段 / A steps-only budget cannot terminate slices that never commit a generation
        if report.steps == 0 && report.termination == SearchTermination::EvaluationLimit
            && self.config.budget.evaluations.is_none() && self.config.budget.decisions.is_none() && self.config.budget.time_millis.is_none() {
            worker.disabled = true;
            self.diagnostics.push(format!("slice allowance cannot commit a complete step: {action}"));
        }
        let contribution = boundary(|| Ok(self.memory.contribution(&self.problem, &reference, &report.candidates)));
        let (improved, contribution) = match contribution {
            Ok(value) => value,
            Err(error) => { failed = true; self.diagnostics.push(error.to_string()); (false, 0.0) }
        };
        let credit = boundary(|| credit.with_candidates(&self.problem, &report.candidates));
        let merged = match boundary(|| Ok(self.memory.offer(&self.problem, report.candidates, self.components.acceptance.as_mut()))) {
            Ok(value) => value,
            Err(error) => { failed = true; self.diagnostics.push(error.to_string()); false }
        };
        if merged { self.stagnation = 0; self.successes += 1; } else { self.stagnation += 1; }
        let reward = match boundary(|| {
            let mut context = credit?;
            context.improved = improved; context.contribution = contribution; context.evaluations = consumed;
            context.action = action.clone(); context.elapsed_nanos = elapsed_nanos;
            let reward = self.components.reward.reward(&context)?;
            if !reward.is_finite() { return Err(invalid("non-finite search reward")); }
            Ok(reward)
        }) {
            Ok(value) => value,
            Err(error) => { failed = true; self.diagnostics.push(error.to_string()); 0.0 }
        };
        let after = match boundary(|| self.context()) {
            Ok(value) => value,
            Err(error) => { failed = true; self.diagnostics.push(error.to_string()); before.clone() }
        };
        if failed { self.workers[index].disabled = true; }
        let transition = SelectionTransition {
            episode: format!("{}:{}:{}", self.problem.id(), self.problem.fingerprint(), self.config.execution.seed),
            decision, action, before, after, available,
            next_available: self.workers.iter().filter(|worker| !worker.disabled && !self.pending.contains_key(&worker.id)
                && !batch_active.contains(&worker.id)).map(|worker| worker.id.clone()).collect(),
            reward, evaluations: consumed, terminal: failed || self.termination().is_some(), policy_version: revision,
            feature_version: self.components.features.version().into(), scope: "algorithm".into(), steps: report.steps, failed, merged_improvement: merged,
            elapsed_nanos: if self.components.reward.requires_elapsed_time() { elapsed_nanos } else { 0 }, previous_action,
        };
        self.transitions.extend(operator_transitions.into_iter().map(|mut event| {
            event.episode = transition.episode.clone();
            event.scope = format!("{}/{}", transition.action, event.scope);
            event
        }));
        self.transitions.push(transition.clone());
        let observed = boundary(|| self.policy.observe(&transition));
        observed?;
        if failed { return Err(crate::solver::heuristic::result::HeuristicError::Model(self.diagnostics.join("; "))); }
        Ok(())
    }

    fn receive_pending(&mut self) -> Result<()> {
        let expected = if self.schedule.replaying() {
            Some(self.schedule.peek().filter(|event| event.kind == "complete").ok_or_else(|| invalid("schedule expected a completion"))?.action.clone())
        } else { None };
        let id = loop {
            if let Some(id) = &expected {
                if self.ready_completions.remove(id) { break id.clone(); }
            } else if let Some(id) = self.ready_completions.pop_first() { break id; }
            let id = self.completions.as_ref().ok_or_else(|| invalid("missing completion channel"))?
                .recv().map_err(|_| invalid("completion channel disconnected"))?;
            if expected.as_ref().is_none_or(|wanted| *wanted == id) { break id; }
            if !self.pending.contains_key(&id) || !self.ready_completions.insert(id) { return Err(invalid("unexpected or duplicate completion notification")); }
        };
        let mut dispatch = self.pending.remove(&id).ok_or_else(|| invalid("unexpected or duplicate worker completion"))?;
        let report = self.workers[dispatch.index].session.receive();
        dispatch.elapsed_nanos = self.workers[dispatch.index].session.execution_nanos();
        self.observe_stop();
        let event = ScheduleEvent { kind: "complete".into(), decision: dispatch.decision, action: id,
            evaluations: dispatch.slice.used(), steps: report.as_ref().map_or(0, |report| report.steps),
            stopped: self.observed_stopped,
            failed: report.as_ref().map_or(true, |report| report.termination == SearchTermination::Failed || !report.diagnostics.is_empty()), ..Default::default() };
        let checked = self.schedule.event(event);
        let result = boundary(|| self.merge(dispatch, report, &[]));
        if let Err(error) = &result && self.diagnostics.is_empty() {
            self.diagnostics.push(error.to_string());
        }
        checked.and(result)
    }

    fn drain_pending(&mut self) -> Result<()> {
        self.observe_stop();
        let mut error = None;
        if !self.pending.is_empty()
            && let Err(failure) = self.schedule.event(ScheduleEvent { kind: "drain".into(), stopped: self.observed_stopped, ..Default::default() })
        {
            self.schedule.diverged = true; error = Some(failure);
        }
        while !self.pending.is_empty() {
            if let Err(failure) = self.receive_pending() { self.schedule.diverged = true; error = Some(failure); }
        }
        error.map_or(Ok(()), Err)
    }

    fn quiesce(&mut self) -> Result<()> {
        let result = self.drain_pending();
        self.pause_if_idle();
        result
    }

    fn advance_asynchronous(&mut self) -> Result<()> {
        let mut available: Vec<_> = self.workers.iter()
            .filter(|worker| !worker.disabled && !self.pending.contains_key(&worker.id))
            .map(|worker| worker.id.clone()).collect();
        // 使用已归并消耗加完整在途配额，避免读取并发计数时重复释放预留。
        // Use accounted consumption plus full in-flight allowances, never racing live counters to release reservations.
        let reserved: u64 = self.pending.values().map(|dispatch| dispatch.slice.evaluations()).sum();
        let reserved_steps: u64 = self.pending.values().map(|dispatch| dispatch.steps).sum();
        let mut evaluations = self.budget.limit().saturating_sub(self.accounted).saturating_sub(reserved);
        let mut steps = self.config.budget.steps.unwrap_or(u64::MAX).saturating_sub(self.steps).saturating_sub(reserved_steps);
        while self.pending.len() < self.config.execution.parallelism && !available.is_empty()
            && evaluations > 0 && steps > 0 && self.termination().is_none()
            && (!self.schedule.replaying() || self.schedule.peek().is_some_and(|event| event.kind == "dispatch")) {
            let ceiling = SliceAllocation { evaluations: self.config.budget.slice_evaluations.min(evaluations),
                steps: self.config.budget.slice_steps.min(steps) };
            let progress = self.context()?;
            let allocation = self.components.allocator.allocate(ceiling, &progress)?;
            if allocation.evaluations == 0 || allocation.steps == 0 || allocation.evaluations > ceiling.evaluations || allocation.steps > ceiling.steps {
                return Err(invalid("allocator exceeded its ceiling or returned a zero allowance"));
            }
            let mut dispatch = self.prepare(available.clone(), allocation.evaluations, allocation.steps, self.dispatched)?;
            let scheduled = self.schedule.event(ScheduleEvent { kind: "dispatch".into(), action: dispatch.action.clone(), decision: dispatch.decision,
                available: dispatch.available.clone(), evaluations: allocation.evaluations, steps: allocation.steps,
                random: self.random.state() as i64, ..Default::default() });
            if let Err(error) = scheduled { self.policy.discard(dispatch.decision); return Err(error); }
            available.retain(|id| *id != dispatch.action);
            evaluations -= allocation.evaluations;
            steps -= allocation.steps;
            self.dispatched += 1;
            let worker = &mut self.workers[dispatch.index];
            let sent = boundary(|| worker.session.dispatch(dispatch.slice.clone(), std::mem::take(&mut dispatch.seeds), worker.initialized));
            match sent {
                Ok(()) => { self.pending.insert(dispatch.action.clone(), dispatch); }
                Err(error) => { return self.merge(dispatch, Err(error), &[]); }
            }
        }
        if !self.pending.is_empty() {
            if self.termination().is_some() || (self.schedule.replaying() && self.schedule.peek().is_some_and(|event| event.kind == "drain")) { self.drain_pending() } else { self.receive_pending() }
        } else { Ok(()) }
    }

    fn advance_inner(&mut self) -> Result<()> {
        if self.completions.is_some() && self.config.execution.mode == ExecutionMode::Asynchronous {
            return self.advance_asynchronous();
        }
        let mut available: Vec<_> = self.workers.iter().filter(|worker| !worker.disabled).map(|worker| worker.id.clone()).collect();
        let mut evaluations = self.budget.remaining();
        let mut steps = self.config.budget.steps.unwrap_or(u64::MAX).saturating_sub(self.steps);
        let capacity = if self.completions.is_some() { self.config.execution.parallelism } else { 1 };
        let capacity = capacity.min(self.config.budget.decisions.unwrap_or(u64::MAX).saturating_sub(self.decisions).min(usize::MAX as u64) as usize);
        // 派发前预留不重叠额度，避免完成顺序改变同步批次的请求分配。
        // Reserve disjoint allowances before dispatch so completion order cannot change synchronous allocation.
        let mut batch = BTreeMap::new();
        let prepared = boundary(|| {
            for offset in 0..capacity {
                if available.is_empty() || evaluations == 0 || steps == 0 || self.stopped.load(Ordering::Relaxed) { break; }
                let ceiling = SliceAllocation { evaluations: self.config.budget.slice_evaluations.min(evaluations),
                    steps: self.config.budget.slice_steps.min(steps) };
                let progress = self.context_at(self.dispatched + offset as u64)?;
                let allocation = self.components.allocator.allocate(ceiling, &progress)?;
                if allocation.evaluations == 0 || allocation.steps == 0 || allocation.evaluations > ceiling.evaluations || allocation.steps > ceiling.steps {
                    return Err(invalid("allocator exceeded its ceiling or returned a zero allowance"));
                }
                let allowance = allocation.evaluations;
                let allowance_steps = allocation.steps;
                let dispatch = self.prepare(available.clone(), allowance, allowance_steps, self.dispatched + offset as u64)?;
                available.retain(|id| *id != dispatch.action);
                evaluations -= allowance;
                steps -= allowance_steps;
                batch.insert(dispatch.action.clone(), dispatch);
            }
            Ok(())
        });
        if let Err(error) = prepared {
            for dispatch in batch.values() { self.policy.discard(dispatch.decision); }
            return Err(error);
        }
        self.dispatched += batch.len() as u64;
        if self.completions.is_none() {
            if let Some((_, mut dispatch)) = batch.pop_first() {
                let worker = &mut self.workers[dispatch.index];
                let started = Instant::now();
                let report = super::parallel::execute(worker.session.as_mut(), &mut dispatch.slice, std::mem::take(&mut dispatch.seeds), worker.initialized);
                dispatch.elapsed_nanos = started.elapsed().as_nanos().min(u64::MAX as u128) as u64;
                return self.merge(dispatch, Ok(report), &[]);
            }
            return Ok(());
        }
        let mut pending = Vec::new();
        let mut completed = BTreeMap::new();
        for (id, dispatch) in &mut batch {
            let worker = &mut self.workers[dispatch.index];
            match boundary(|| worker.session.dispatch(dispatch.slice.clone(), std::mem::take(&mut dispatch.seeds), worker.initialized)) {
                Ok(()) => pending.push(id.clone()),
                Err(error) => { completed.insert(id.clone(), Err(error)); }
            }
        }
        let mut error = None;
        // 每次公开 advance 返回前排空批次，包括失败路径，检查点永远处于静止边界。
        // Drain each batch before public advance returns, including failures, keeping checkpoints quiescent.
        for _ in 0..pending.len() {
            let id = self.completions.as_ref().ok_or_else(|| invalid("missing completion channel"))?
                .recv().map_err(|_| invalid("completion channel disconnected"))?;
            let dispatch = batch.get_mut(&id).ok_or_else(|| invalid("unexpected worker completion"))?;
            let report = self.workers[dispatch.index].session.receive();
            dispatch.elapsed_nanos = self.workers[dispatch.index].session.execution_nanos();
            if self.config.execution.mode == ExecutionMode::Asynchronous {
                let dispatch = batch.remove(&id).ok_or_else(|| invalid("duplicate worker completion"))?;
                if let Err(failure) = boundary(|| self.merge(dispatch, report, &[])) { error = Some(failure); }
            } else { completed.insert(id, report); }
        }
        let mut batch_active: Vec<_> = batch.keys().cloned().collect();
        for (id, dispatch) in batch {
            batch_active.retain(|active| active != &id);
            let report = completed.remove(&id).ok_or_else(|| invalid("missing worker result"))?;
            if let Err(failure) = boundary(|| self.merge(dispatch, report, &batch_active)) { error = Some(failure); }
        }
        error.map_or(Ok(()), Err)
    }

    /// 推进直至全局预算、停止或失败 / Advance until a global limit, stop request, or failure
    pub fn run(&mut self) -> Result<SearchResult<O, V>> {
        let execution = (|| { while self.advance()? {} Ok(self.result()) })();
        let saved = if let Some(path) = &self.config.learning.trajectory_path {
            super::learning::TrajectoryFiles::write(path, &self.transitions)
        } else { Ok(()) };
        match execution { Err(error) => Err(error), Ok(result) => { saved?; Ok(result) } }
    }

    /// 关闭所有会话；重复关闭安全 / Close every session; repeated closure is safe
    pub fn close(&mut self) -> Result<()> {
        if self.closed { return Ok(()); }
        let drained = self.quiesce();
        self.closed = true;
        let mut errors = Vec::new();
        if let Err(error) = drained { errors.push(error.to_string()); }
        for worker in &mut self.workers {
            worker.closed = true;
            if let Err(error) = boundary(|| worker.session.close()) { errors.push(error.to_string()); }
        }
        if errors.is_empty() { Ok(()) } else { Err(crate::solver::heuristic::result::HeuristicError::Model(errors.join("; "))) }
    }
}

fn remap<V: Clone>(
    solution: &Solution<V>,
    source: &[VariableId],
    target: &[VariableId],
) -> Result<Solution<V>> {
    if source.len() != target.len() || solution.len() != source.len() { return Err(invalid("incompatible solution dimensions")); }
    source.iter().zip(target).map(|(source, target)| solution.get(source).cloned().map(|value| (*target, value))
        .ok_or_else(|| invalid("solution contains noncanonical variable identities"))).collect()
}
