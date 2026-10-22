//! 通用可恢复会话 / Generic resumable session
//!
//! 与 Kotlin `RunnerSearchSession<S, O, V>` 对应：会话只知道「状态类型 `S`、初始化、推进一步、把状态摊平为个体」四件事，
//! 其余（迭代、档案、预算、快照、导入）全部由会话负责。因此同一个实现可以承载种群、轨迹、粒子群、岛屿、CLPSO、SHADE
//! 与 CMA-ES。
//! Mirrors Kotlin's `RunnerSearchSession<S, O, V>`: the session knows only "state type `S`, initialise, advance one step, and
//! flatten the state into individuals"; iteration, archive, budget, snapshots, and imports are the session's job. One
//! implementation therefore carries population, trajectory, particle swarm, island, CLPSO, SHADE, and CMA-ES searches.

use crate::model::callback::Solution;
use crate::variable::VariableId;

use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, from_f64, to_f64};
use crate::solver::heuristic::operator::compare_strictly;
use crate::solver::heuristic::result::HeuristicError;
use crate::solver::heuristic::runner::iteration_observer::IterationObserver;
use crate::solver::heuristic::session::archive::{ObjectiveComponents, SessionArchive};
use crate::solver::heuristic::session::budget::SearchSlice;
use crate::solver::heuristic::session::json::{JsonValue, SearchCodec};
use crate::solver::heuristic::session::protocol::{
    Result, SearchProblem, SearchSession, SearchSessionSnapshot, SearchStepResult,
    SearchTermination,
};

/// 骨架钩子：会话只需要这四个能力 / Skeleton hooks: the four capabilities a session needs
///
/// `advance` 接收可变问题引用，因为骨架在推进一步时需要读取模型（目标、边界）并刷新缓存。
/// `advance` takes a mutable problem because the skeleton reads the model (objectives, bounds) and flushes caches while
/// advancing.
pub trait SessionHooks<P, O, V, S>
where
    P: SearchProblem<O, V>,
    O: Clone,
    V: HeuristicValue,
    S: Clone,
{
    /// 建立初始状态 / Build the initial state
    fn initialize(&mut self, problem: &P, slice: &SearchSlice) -> Result<S>;

    /// 推进一步 / Advance one step
    fn advance(&mut self, problem: &mut P, iteration: &Iteration, state: &S, slice: &SearchSlice)
        -> Result<S>;

    /// 把状态摊平为个体 / Flatten the state into individuals
    fn members(&self, state: &S) -> Vec<Individual<O, V>>;
}

/// 会话钩子包装的骨架 / A skeleton wrapped by the session hooks
///
/// 以闭包组装钩子，避免为每个骨架新增一个类型；Kotlin 用 `AbstractRunner` 的子类达到同样效果。
/// Hooks are assembled from closures so no per-skeleton type is needed; Kotlin achieves the same via `AbstractRunner`
/// subclasses.
pub struct HookedSkeleton<P, O, V, S>
where
    P: SearchProblem<O, V>,
    O: Clone,
    V: HeuristicValue,
    S: Clone,
{
    initializer: Box<dyn FnMut(&P, &SearchSlice) -> Result<S>>,
    advance: Box<dyn FnMut(&mut P, &Iteration, &S, &SearchSlice) -> Result<S>>,
    members: Box<dyn Fn(&S) -> Vec<Individual<O, V>>>,
}

impl<P, O, V, S> HookedSkeleton<P, O, V, S>
where
    P: SearchProblem<O, V>,
    O: Clone,
    V: HeuristicValue,
    S: Clone,
{
    /// 以三个闭包创建 / Create from three closures
    pub fn new(
        initializer: impl FnMut(&P, &SearchSlice) -> Result<S> + 'static,
        advance: impl FnMut(&mut P, &Iteration, &S, &SearchSlice) -> Result<S> + 'static,
        members: impl Fn(&S) -> Vec<Individual<O, V>> + 'static,
    ) -> Self {
        Self {
            initializer: Box::new(initializer),
            advance: Box::new(advance),
            members: Box::new(members),
        }
    }
}

impl<P, O, V, S> SessionHooks<P, O, V, S> for HookedSkeleton<P, O, V, S>
where
    P: SearchProblem<O, V>,
    O: Clone,
    V: HeuristicValue,
    S: Clone,
{
    fn initialize(&mut self, problem: &P, slice: &SearchSlice) -> Result<S> {
        (self.initializer)(problem, slice)
    }

    fn advance(
        &mut self,
        problem: &mut P,
        iteration: &Iteration,
        state: &S,
        slice: &SearchSlice,
    ) -> Result<S> {
        (self.advance)(problem, iteration, state, slice)
    }

    fn members(&self, state: &S) -> Vec<Individual<O, V>> {
        (self.members)(state)
    }
}

/// 状态编解码集合 / Bundle of state codecs
///
/// `state` 编码算法原生状态，`archive` 编码档案个体；两者都可为空，表示禁止快照/恢复（与 Kotlin 的 `codec: null` 一致）。
/// `state` encodes the native algorithm state and `archive` encodes archived individuals; either may be absent, disabling
/// snapshot/restore (matching Kotlin's `codec: null`).
pub struct SessionCodecs<S, O, V>
where
    O: Clone,
    V: HeuristicValue,
{
    /// 原生状态编解码 / Native state codec
    pub state: Option<Box<dyn SearchCodec<S>>>,
    /// 归档解编解码 / Archived-solution codec
    pub archive: Option<Box<dyn SearchCodec<Vec<Individual<O, V>>>>>,
}

impl<S, O, V> SessionCodecs<S, O, V>
where
    O: Clone,
    V: HeuristicValue,
{
    /// 创建编解码集合 / Create a codec bundle
    pub fn new(
        state: Option<Box<dyn SearchCodec<S>>>,
        archive: Option<Box<dyn SearchCodec<Vec<Individual<O, V>>>>>,
    ) -> Self {
        Self { state, archive }
    }

    /// 不支持快照的集合 / A bundle that disables snapshots
    pub fn unavailable() -> Self {
        Self {
            state: None,
            archive: None,
        }
    }
}

/// 通用可恢复会话 / Generic resumable session
pub struct RunnerSession<P, O, V, S>
where
    P: SearchProblem<O, V>,
    O: ObjectiveComponents,
    V: HeuristicValue,
    S: Clone,
{
    hooks: Box<dyn SessionHooks<P, O, V, S>>,
    problem: P,
    codecs: SessionCodecs<S, O, V>,
    importer: Option<Box<dyn Fn(&S, &[Individual<O, V>]) -> Result<S>>>,
    observers: Vec<Box<dyn IterationObserver<O, V>>>,
    iteration: Iteration,
    state: Option<S>,
    archive: SessionArchive<O, V>,
    active_nanos: u64,
}

impl<P, O, V, S> RunnerSession<P, O, V, S>
where
    P: SearchProblem<O, V>,
    O: ObjectiveComponents,
    V: HeuristicValue,
    S: Clone,
{
    /// 创建会话 / Create a session
    ///
    /// 默认容量与 Kotlin 的 `archiveAmount` 语义一致：单目标保留最优 1 个，多目标保留 100 个前沿成员。
    /// The default capacity follows Kotlin's `archiveAmount` semantics: the single best solution for single-objective search
    /// and 100 front members for multi-objective search.
    pub fn new(
        hooks: Box<dyn SessionHooks<P, O, V, S>>,
        problem: P,
        codecs: SessionCodecs<S, O, V>,
    ) -> Self {
        let capacity = if problem.multi_objective() { 100 } else { 1 };
        Self::with_archive_capacity(hooks, problem, codecs, capacity)
    }

    /// 以档案容量创建会话 / Create a session with an explicit archive capacity
    ///
    /// 多目标搜索应给出与 `archiveAmount` 一致的容量；容量为 0 会被夹到 1。
    /// Multi-objective searches should pass the `archiveAmount` capacity; a zero capacity is clamped to one.
    pub fn with_archive_capacity(
        hooks: Box<dyn SessionHooks<P, O, V, S>>,
        problem: P,
        codecs: SessionCodecs<S, O, V>,
        capacity: usize,
    ) -> Self {
        Self {
            hooks,
            problem,
            codecs,
            importer: None,
            observers: Vec::new(),
            iteration: Iteration::new(),
            state: None,
            archive: SessionArchive::new(capacity),
            active_nanos: 0,
        }
    }

    /// 设置导入策略 / Set the import policy
    pub fn with_importer(
        mut self,
        importer: impl Fn(&S, &[Individual<O, V>]) -> Result<S> + 'static,
    ) -> Self {
        self.importer = Some(Box::new(importer));
        self
    }

    /// 追加迭代观测者 / Append an iteration observer
    ///
    /// 与 Kotlin 一致：观测者在每次成功推进后广播，包括「是否改进」信号。
    /// Matches Kotlin: observers are broadcast after every successful advance, including the "improved" signal.
    pub fn push_observer(&mut self, observer: Box<dyn IterationObserver<O, V>>) {
        self.observers.push(observer);
    }

    /// 问题引用 / Problem reference
    pub fn problem(&self) -> &P {
        &self.problem
    }

    /// 变量规范顺序，便于调用方构造导入解 / Canonical variable order, so callers can build import solutions
    pub fn variable_order(&self) -> Vec<VariableId> {
        self.problem.variable_order()
    }

    /// 当前归档 / Current archive
    pub fn archive(&self) -> &[Individual<O, V>] {
        self.archive.items()
    }

    /// 档案容量 / Archive capacity
    pub fn archive_capacity(&self) -> usize {
        self.archive.capacity()
    }

    /// 把本轮成员并入归档 / Merge this round's members into the archive
    fn refresh_archive(&mut self, members: &[Individual<O, V>]) -> bool {
        self.archive
            .update(&self.problem, members.to_vec())
    }

    /// 评估单个解 / Evaluate one solution
    fn evaluate(&self, solution: Solution<V>) -> Option<Individual<O, V>> {
        if !self.problem.valid_solution(&solution) || !self.problem.feasible(&solution) {
            return None;
        }
        let objective = self.problem.objective(&solution)?;
        if !self.problem.valid_objective(&objective) {
            return None;
        }
        Some(Individual::new(solution, objective))
    }

    fn encode_archive(&self) -> Result<JsonValue> {
        match &self.codecs.archive {
            Some(codec) => codec.encode(&self.archive.items().to_vec()),
            None => Err(HeuristicError::InvalidArgument(
                "missing archive codec / 缺少归档编解码器".to_string(),
            )),
        }
    }

    fn decode_archive(&self, value: &JsonValue) -> Result<Vec<Individual<O, V>>> {
        match &self.codecs.archive {
            Some(codec) => codec.decode(value),
            None => Err(HeuristicError::InvalidArgument(
                "missing archive codec / 缺少归档编解码器".to_string(),
            )),
        }
    }
}

/// 个体目标比较 / Compare two individuals by objective
fn compare<P, O, V>(
    problem: &P,
    lhs: &Individual<O, V>,
    rhs: &Individual<O, V>,
) -> std::cmp::Ordering
where
    P: SearchProblem<O, V>,
    O: Clone,
    V: HeuristicValue,
{
    if problem.dominates(&lhs.fitness, &rhs.fitness) {
        std::cmp::Ordering::Less
    } else if problem.dominates(&rhs.fitness, &lhs.fitness) {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Equal
    }
}

impl<P, O, V, S> SearchSession<O, V> for RunnerSession<P, O, V, S>
where
    P: SearchProblem<O, V>,
    O: ObjectiveComponents,
    V: HeuristicValue,
    S: Clone,
{
    fn supports_import(&self) -> bool {
        self.importer.is_some()
    }

    fn step(&mut self, slice: &mut SearchSlice) -> Result<SearchStepResult<O, V>> {
        let mut completed = 0_u64;
        let mut reason = SearchTermination::SliceComplete;
        let mut candidates: Vec<Individual<O, V>> = Vec::new();

        // 首次推进负责建立初始状态，与 Kotlin 会话的「previous == null 时 initialize」一致。
        // The first advance builds the initial state, matching Kotlin's "initialize when previous == null".
        if self.state.is_none() {
            let state = match self.hooks.initialize(&self.problem, slice) {
                Ok(state) => state,
                Err(error) => {
                    return Ok(SearchStepResult {
                        candidates: Vec::new(),
                        evaluations: slice.used(),
                        steps: 0,
                        termination: SearchTermination::Failed,
                        diagnostics: vec![error.to_string()],
                    })
                }
            };
            let members = self.hooks.members(&state);
            if members.is_empty() {
                return Ok(SearchStepResult {
                    candidates: Vec::new(),
                    evaluations: slice.used(),
                    steps: 0,
                    termination: SearchTermination::NoCandidate,
                    diagnostics: Vec::new(),
                });
            }
            self.refresh_archive(&members);
            self.state = Some(state);
            self.iteration.next(true);
        }

        while completed < slice.steps() && !slice.expired() {
            if !slice.acquire() {
                reason = if slice.global().remaining() == 0 {
                    SearchTermination::EvaluationLimit
                } else {
                    SearchTermination::SliceComplete
                };
                break;
            }

            let current = match &self.state {
                Some(state) => state.clone(),
                None => break,
            };
            let advanced = match self.hooks.advance(
                &mut self.problem,
                &self.iteration,
                &current,
                slice,
            ) {
                Ok(advanced) => advanced,
                Err(error) => {
                    return Ok(SearchStepResult {
                        candidates,
                        evaluations: slice.used(),
                        steps: completed,
                        termination: SearchTermination::Failed,
                        diagnostics: vec![error.to_string()],
                    })
                }
            };

            let members = self.hooks.members(&advanced);
            if members.is_empty() {
                self.state = Some(advanced);
                reason = SearchTermination::NoCandidate;
                break;
            }

            candidates.extend(members.iter().cloned());
            let improved = self.refresh_archive(&members);
            for observer in self.observers.iter_mut() {
                observer.observe(&self.iteration, &members, improved);
            }
            self.problem.flush();
            self.state = Some(advanced);
            self.iteration.next(improved);
            completed += 1;
            self.active_nanos = self.iteration.time().as_nanos() as u64;
        }

        if slice.expired() {
            reason = SearchTermination::TimeLimit;
        }

        Ok(SearchStepResult {
            candidates,
            evaluations: slice.used(),
            steps: completed,
            termination: reason,
            diagnostics: Vec::new(),
        })
    }

    fn import_solutions(
        &mut self,
        solutions: Vec<Solution<V>>,
        slice: &mut SearchSlice,
    ) -> Result<SearchStepResult<O, V>> {
        let state = self.state.clone().ok_or_else(|| {
            HeuristicError::InvalidArgument(
                "initialize before importing / 导入前必须先初始化".to_string(),
            )
        })?;
        let importer = self.importer.as_ref().ok_or_else(|| {
            HeuristicError::InvalidArgument(
                "session does not support imports / 会话不支持解导入".to_string(),
            )
        })?;

        let mut candidates = Vec::new();
        let mut exhausted = false;
        for solution in solutions {
            if !slice.acquire() {
                exhausted = true;
                break;
            }
            if let Some(individual) = self.evaluate(solution) {
                candidates.push(individual);
            }
        }

        let imported = importer(&state, &candidates)?;
        let members = self.hooks.members(&imported);
        self.refresh_archive(&members);
        self.refresh_archive(&candidates);
        self.state = Some(imported);

        Ok(SearchStepResult {
            candidates,
            evaluations: slice.used(),
            steps: 0,
            termination: if exhausted {
                if slice.global().remaining() == 0 {
                    SearchTermination::EvaluationLimit
                } else {
                    SearchTermination::SliceComplete
                }
            } else {
                SearchTermination::SliceComplete
            },
            diagnostics: Vec::new(),
        })
    }

    fn snapshot(&self) -> Result<SearchSessionSnapshot> {
        let state = match (&self.codecs.state, &self.state) {
            (Some(codec), Some(state)) => codec.encode(state)?,
            (Some(_), None) => JsonValue::Null,
            (None, _) => {
                return Err(HeuristicError::InvalidArgument(
                    "missing native state codec / 缺少原生状态编解码器".to_string(),
                ))
            }
        };

        Ok(SearchSessionSnapshot {
            version: SearchSessionSnapshot::VERSION,
            initialized: self.state.is_some(),
            iteration: self.iteration.iteration(),
            stagnation: self.iteration.not_better_iteration(),
            state,
            archive: self.encode_archive()?,
            active_nanos: self.active_nanos,
        })
    }

    fn restore(&mut self, snapshot: &SearchSessionSnapshot) -> Result<()> {
        if snapshot.version != SearchSessionSnapshot::VERSION {
            return Err(HeuristicError::InvalidArgument(format!(
                "unsupported snapshot version: {} / 不支持的快照版本：{}",
                snapshot.version, snapshot.version
            )));
        }

        let archive = self.decode_archive(&snapshot.archive)?;
        let state = if snapshot.initialized {
            match &self.codecs.state {
                Some(codec) => Some(codec.decode(&snapshot.state)?),
                None => {
                    return Err(HeuristicError::InvalidArgument(
                        "missing native state codec / 缺少原生状态编解码器".to_string(),
                    ))
                }
            }
        } else {
            None
        };

        self.state = state;
        self.archive.replace(archive);
        self.iteration =
            Iteration::with_counts(snapshot.iteration, snapshot.stagnation);
        self.active_nanos = snapshot.active_nanos;
        Ok(())
    }

    fn iteration(&self) -> &Iteration {
        &self.iteration
    }
}

/// 会话观测者广播辅助 / Helper broadcasting to observers
pub fn broadcast_observers<O, V>(
    observers: &mut [Box<dyn IterationObserver<O, V>>],
    iteration: &Iteration,
    members: &[Individual<O, V>],
    improved: bool,
) where
    O: Clone,
    V: HeuristicValue,
{
    for observer in observers.iter_mut() {
        observer.observe(iteration, members, improved);
    }
}

/// 变量顺序辅助：把解转成规范顺序的数值向量 / Convert a solution into a canonical-order value vector
pub fn values_of<V>(order: &[VariableId], solution: &Solution<V>) -> Vec<f64>
where
    V: HeuristicValue,
{
    order
        .iter()
        .map(|id| solution.get(id).map(to_f64).unwrap_or(0.0))
        .collect()
}

/// 变量顺序辅助：把数值向量还原为解 / Rebuild a solution from a value vector
pub fn solution_of<V>(order: &[VariableId], values: &[f64]) -> Solution<V>
where
    V: HeuristicValue,
{
    let mut solution = Solution::<V>::new();
    for (index, id) in order.iter().enumerate() {
        if let Some(value) = values.get(index) {
            solution.insert(*id, from_f64(*value));
        }
    }
    solution
}

/// 目标比较辅助，供外部复用 / Objective comparison helper for external reuse
pub fn compare_objective<P, O, V>(problem: &P, lhs: &O, rhs: &O) -> std::cmp::Ordering
where
    P: SearchProblem<O, V>,
    O: Clone,
    V: HeuristicValue,
{
    if problem.dominates(lhs, rhs) {
        std::cmp::Ordering::Less
    } else if problem.dominates(rhs, lhs) {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Equal
    }
}

/// 便于算子使用的严格比较适配 / Strict-comparison adapter for operators
pub fn strict_compare<P, O, V>(
    problem: &P,
    lhs: &Individual<O, V>,
    rhs: &Individual<O, V>,
) -> std::cmp::Ordering
where
    P: SearchProblem<O, V>,
    O: Clone,
    V: HeuristicValue,
{
    compare(problem, lhs, rhs)
}

/// 让 `compare_strictly` 在会话中被复用 / Reuse `compare_strictly` inside sessions
pub(crate) fn compare_with_model<M, V>(
    model: &M,
    lhs: &V,
    rhs: &V,
) -> std::cmp::Ordering
where
    M: crate::solver::heuristic::model::ObjectiveModel<V, V>,
    V: HeuristicValue,
{
    compare_strictly(model, lhs, rhs)
}
