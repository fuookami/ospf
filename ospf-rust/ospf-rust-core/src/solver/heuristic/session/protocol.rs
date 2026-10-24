//! 搜索协议 / Search protocol
//!
//! 会话层的公共词汇：终止原因、解编码、问题边界、片段报告、快照与解导入。它们与具体算法无关，因此同一个协议既能
//! 承载种群搜索，也能承载轨迹搜索。
//! The shared vocabulary of the session layer: termination reasons, solution encodings, problem boundaries, slice reports,
//! snapshots, and solution imports. None of it is algorithm-specific, so one protocol carries both population and trajectory
//! search.
//!
//! 目标类型 `O` 与会话泛型：与 Kotlin `SearchProblem<ObjValue, V>` 一致，`O` 可以是标量（单目标）或目标向量
//! （多目标），由 [`SearchProblem::dominates`] 定义偏序。
//! Objective type `O` and session generics: matching Kotlin's `SearchProblem<ObjValue, V>`, `O` may be a scalar
//! (single-objective) or an objective vector (multi-objective), with the partial order defined by
//! [`SearchProblem::dominates`].

use std::fmt::{Display, Formatter};

use crate::model::callback::Solution;
use crate::variable::VariableId;

use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::result::HeuristicError;
use crate::solver::heuristic::session::budget::SearchSlice;
use crate::solver::heuristic::session::json::JsonValue;

/// 搜索终止原因 / Search termination reason
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SearchTermination {
    /// 片段额度用完 / The slice allowance is exhausted
    SliceComplete,
    /// 全局评估上限 / Global evaluation limit
    EvaluationLimit,
    /// 片段步数上限 / Slice step limit
    StepLimit,
    /// 时间上限 / Time limit
    TimeLimit,
    /// 外部请求停止 / Stopped externally
    Stopped,
    /// 没有可用候选 / No usable candidate
    NoCandidate,
    /// 执行失败 / Execution failed
    Failed,
}

impl SearchTermination {
    /// 是否为正常结束（而非失败）/ Whether the run ended normally rather than failing
    pub fn is_completed(self) -> bool {
        !matches!(self, SearchTermination::Failed | SearchTermination::NoCandidate)
    }
}

impl Display for SearchTermination {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            SearchTermination::SliceComplete => "slice-complete",
            SearchTermination::EvaluationLimit => "evaluation-limit",
            SearchTermination::StepLimit => "step-limit",
            SearchTermination::TimeLimit => "time-limit",
            SearchTermination::Stopped => "stopped",
            SearchTermination::NoCandidate => "no-candidate",
            SearchTermination::Failed => "failed",
        };
        write!(f, "{text}")
    }
}

/// 解编码能力 / Solution encoding capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SearchEncoding {
    /// 连续编码 / Continuous encoding
    Continuous,
    /// 排列编码 / Permutation encoding
    Permutation,
    /// 自定义编码 / Custom encoding
    Custom,
}

/// 搜索协议结果别名 / Result alias of the search protocol
pub type Result<T> = std::result::Result<T, HeuristicError>;

/// 问题边界，目标比较与可行性由调用方定义 / Problem boundary with caller-defined ordering and feasibility
///
/// 会话只通过该边界访问模型，因此测试可以注入纯函数模型，而生产代码可以注入回调模型。偏序由
/// [`dominates`](SearchProblem::dominates) 定义：单目标时即「严格更优」，多目标时即 Pareto 支配。
/// Sessions access the model only through this boundary, so tests can inject pure-function models while production code injects
/// a callback model. The partial order is defined by [`dominates`](SearchProblem::dominates): strict improvement for
/// single-objective problems and Pareto dominance for multi-objective ones.
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 解值类型 / Solution value type
pub trait SearchProblem<O, V>
where
    O: Clone,
    V: HeuristicValue,
{
    /// 问题标识 / Problem identifier
    fn id(&self) -> &str;

    /// 输入与目标定义指纹 / Fingerprint of the input and objective definition
    fn fingerprint(&self) -> &str;

    /// 变量规范顺序 / Canonical variable order
    fn variable_order(&self) -> Vec<VariableId>;

    /// 变量下界（按规范顺序的下标）/ Variable lower bound by canonical index
    fn lower_bound(&self, _index: usize) -> Option<f64> {
        None
    }

    /// 变量上界（按规范顺序的下标）/ Variable upper bound by canonical index
    fn upper_bound(&self, _index: usize) -> Option<f64> {
        None
    }

    /// 生成初始解 / Generate initial solutions
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<V>>;

    /// 计算目标值 / Evaluate the objective
    fn objective(&self, solution: &Solution<V>) -> Option<O>;

    /// 左目标是否严格优于（或支配）右目标 / Whether the left objective strictly improves on or dominates the right one
    fn dominates(&self, lhs: &O, rhs: &O) -> bool;

    /// 目标是否等价 / Whether two objectives are equivalent
    fn equivalent(&self, lhs: &O, rhs: &O) -> bool {
        !self.multi_objective() && !self.dominates(lhs, rhs) && !self.dominates(rhs, lhs)
    }

    /// 目标是否有效 / Whether an objective value is valid
    fn valid_objective(&self, _objective: &O) -> bool {
        true
    }

    /// 判断可行性 / Decide feasibility
    fn feasible(&self, _solution: &Solution<V>) -> bool {
        true
    }

    /// 可搜索域，默认保持严格可行性过滤；显式覆盖不放宽有效档案 / Search domain, strictly feasible by default; overriding never relaxes valid archives
    fn admissible(&self, solution: &Solution<V>) -> bool {
        self.feasible(solution)
    }

    /// 装配约束策略时的显式搜索域；默认绝不放宽业务禁区 / Explicit domain under constraint handling; never relaxes business exclusions by default
    fn constraint_admissible(&self, solution: &Solution<V>) -> bool { self.admissible(solution) }

    /// 约束视图的计费入口；原搜索域保留自定义 evaluate，显式扩域可覆盖本方法保留专用缓存 / Metered constraint-view entry; preserve custom evaluate in the original domain, override for custom caches in explicitly relaxed domains
    fn evaluate_constraint_admissible(&self, solution: Solution<V>, slice: &SearchSlice) -> Result<SearchEvaluationResult<O, V>> {
        if self.admissible(&solution) { return self.evaluate(solution, slice); }
        if !slice.acquire_shared() { return Ok(SearchEvaluationResult { admitted: false, candidate: None }); }
        let candidate = if self.valid_solution(&solution) && self.constraint_admissible(&solution) {
            slice.global().objective_called();
            self.objective(&solution).filter(|objective| self.valid_objective(objective))
                .map(|objective| Individual::new(solution, objective))
        } else { None };
        Ok(SearchEvaluationResult { admitted: true, candidate })
    }

    /// 是否已装配原生约束策略 / Whether native constraint handling is attached
    fn has_constraints(&self) -> bool { false }

    /// 是否需要随机批量排序，成对比较仍确定 / Whether batch ordering is stochastic; pair comparisons remain deterministic
    fn stochastic_constraints(&self) -> bool { false }

    /// 读取并校验约束，必须先于目标调用 / Read and validate constraints before objective calls
    fn observe_constraints(&self, _solution: &Solution<V>) -> Result<()> { Ok(()) }

    /// 当前事务锁定的错误 / Latched error of the current transaction
    fn constraint_failure(&self) -> Option<String> { None }

    /// 原生约束层级，空值表示无策略 / Native constraint tier, absent without a strategy
    fn constraint_tier(&self, _solution: &Solution<V>) -> Option<(u8, f64)> { None }

    /// 开始约束事务 / Begin a constraint transaction
    fn begin_constraints(&self) {}

    /// 完整步提交后校准并推进约束状态 / Calibrate and advance constraints after a complete step
    fn commit_constraints(&self) -> Result<()> { Ok(()) }

    /// 保存独立约束状态 / Save independent constraint state
    fn save_constraints(&self) -> Result<JsonValue> { Ok(JsonValue::Null) }

    /// 恢复约束状态；无策略时拒绝非空状态 / Restore constraint state, rejecting non-null state without handling
    fn load_constraints(&self, state: &JsonValue) -> Result<()> {
        if state.is_null() { Ok(()) } else {
            Err(HeuristicError::InvalidArgument("约束状态能力不匹配 / Constraint state capability mismatch".into()))
        }
    }

    /// 完整个体的确定性比较，不覆盖业务目标 / Deterministic complete-individual comparison without replacing objectives
    fn compare_individuals(&self, lhs: &Individual<O, V>, rhs: &Individual<O, V>) -> std::cmp::Ordering {
        if let (Some(a), Some(b)) = (self.constraint_tier(&lhs.solution), self.constraint_tier(&rhs.solution)) {
            let compared = a.0.cmp(&b.0).then_with(|| a.1.total_cmp(&b.1));
            if compared != std::cmp::Ordering::Equal { return compared; }
        }
        match (self.dominates(&lhs.fitness, &rhs.fitness), self.dominates(&rhs.fitness, &lhs.fitness)) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    }

    /// 批量排序下标；随机扫描实现不得作为 Comparator / Batch rank indices; stochastic scans must never be a Comparator
    fn rank_indices(&self, entries: &[Individual<O, V>]) -> Vec<usize> {
        if self.stochastic_constraints() {
            let solutions: Vec<_> = entries.iter().map(|entry| entry.solution.clone()).collect();
            let objective = |a: usize, b: usize| {
                match (self.dominates(&entries[a].fitness, &entries[b].fitness), self.dominates(&entries[b].fitness, &entries[a].fitness)) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                }
            };
            if let Some(indices) = self.rank_constraints(&solutions, &objective) { return indices; }
        }
        let mut indices: Vec<usize> = (0..entries.len()).collect();
        indices.sort_by(|&a, &b| self.compare_individuals(&entries[a], &entries[b]));
        indices
    }

    /// 以当前目标视图驱动独立约束扫描，标量化不丢失违约信息 / Drive independent constraint scans with the current objective view, retaining violations under scalarization
    fn rank_constraints(
        &self,
        _solutions: &[Solution<V>],
        _objective: &dyn Fn(usize, usize) -> std::cmp::Ordering,
    ) -> Option<Vec<usize>> { None }

    /// 是否提供业务归一化最小化坐标，不从原目标隐式推导 / Whether normalized minimization coordinates are provided, never inferred from raw objectives
    fn has_objective_coordinates(&self) -> bool { false }

    /// 信用计算坐标；最大化须显式取反，不能替换原目标或调用目标评估 / Credit coordinates; explicitly negate maximization, without replacing or evaluating objectives
    fn objective_coordinates(&self, _objective: &O) -> Result<Vec<f64>> {
        Err(HeuristicError::InvalidArgument("缺少显式目标坐标 / Missing explicit objective coordinates".into()))
    }

    /// 校验坐标数值；跨候选的维度一致性由使用者校验 / Validate coordinate values; consumers validate dimensions across candidates
    fn checked_objective_coordinates(&self, objective: &O) -> Result<Vec<f64>> {
        if !self.has_objective_coordinates() || !self.valid_objective(objective) {
            return Err(HeuristicError::InvalidArgument("目标坐标不可用 / Objective coordinates unavailable".into()));
        }
        let coordinates = self.objective_coordinates(objective)?;
        if coordinates.is_empty() || coordinates.iter().any(|value| !value.is_finite())
            || !self.multi_objective() && coordinates.len() != 1 {
            return Err(HeuristicError::InvalidArgument("目标坐标维度或数值无效 / Invalid objective coordinate dimensions or values".into()));
        }
        Ok(coordinates)
    }

    /// 是否提供有限非负的归一化违约度 / Whether finite nonnegative normalized constraint violation is provided
    fn has_constraint_violation(&self) -> bool { false }

    /// 业务违约度，不调用完整目标，也不作为真实可行性证明 / Domain violation, neither evaluating the full objective nor proving feasibility
    fn constraint_violation(&self, _solution: &Solution<V>) -> Result<f64> {
        Err(HeuristicError::InvalidArgument("缺少违约度回调 / Missing constraint violation callback".into()))
    }

    /// 校验违约度，保留业务回调错误 / Validate violation while preserving domain callback errors
    fn checked_constraint_violation(&self, solution: &Solution<V>) -> Result<f64> {
        if !self.has_constraint_violation() || !self.valid_solution(solution) {
            return Err(HeuristicError::InvalidArgument("违约度回调不可用或解无效 / Violation unavailable or invalid solution".into()));
        }
        let violation = self.constraint_violation(solution)?;
        if !violation.is_finite() || violation < 0.0 {
            return Err(HeuristicError::InvalidArgument("违约度必须有限且非负 / Violation must be finite and nonnegative".into()));
        }
        Ok(violation)
    }

    /// 是否为多目标 / Whether the problem is multi-objective
    fn multi_objective(&self) -> bool {
        false
    }

    /// 刷新模型缓存 / Flush model caches
    fn flush(&mut self) {}

    /// 判断解维度是否与问题一致 / Check whether the solution dimension matches the problem
    fn valid_solution(&self, solution: &Solution<V>) -> bool {
        let order = self.variable_order();
        solution.len() == order.len() && order.iter().all(|id| solution.contains_key(id))
    }

    /// 解编码能力 / Solution encoding capability
    fn encoding(&self) -> SearchEncoding {
        SearchEncoding::Continuous
    }

    /// 回调是否无状态；有状态回调必须提供恢复协议 / Whether callbacks are stateless; stateful callbacks require recovery
    fn stateless(&self) -> bool {
        false
    }

    /// 保存回调状态，不包含可丢弃缓存 / Save callback state excluding disposable caches
    fn save_state(&self) -> Result<JsonValue> {
        if self.stateless() { Ok(JsonValue::Null) } else {
            Err(HeuristicError::InvalidArgument("problem state is not recoverable / 问题状态不可恢复".into()))
        }
    }

    /// 恢复回调状态 / Restore callback state
    fn load_state(&mut self, state: &JsonValue) -> Result<()> {
        if self.stateless() && state.is_null() { Ok(()) } else {
            Err(HeuristicError::InvalidArgument("problem state is not recoverable / 问题状态不可恢复".into()))
        }
    }

    /// 统一计费评估，预算拒绝与无效候选分开表达 / Meter evaluations, separating denial from invalid candidates
    fn evaluate(&self, solution: Solution<V>, slice: &SearchSlice) -> Result<SearchEvaluationResult<O, V>> {
        if !slice.acquire_shared() {
            return Ok(SearchEvaluationResult { admitted: false, candidate: None });
        }
        let candidate = if self.valid_solution(&solution) && self.admissible(&solution) {
            self.observe_constraints(&solution)?;
            slice.global().objective_called();
            self.objective(&solution).filter(|objective| self.valid_objective(objective))
                .map(|objective| Individual::new(solution, objective))
        } else { None };
        Ok(SearchEvaluationResult { admitted: true, candidate })
    }
}

/// 单次请求结果 / Result of one evaluation request
pub struct SearchEvaluationResult<O, V> {
    /// 是否消耗请求额度 / Whether a request token was consumed
    pub admitted: bool,
    /// 可搜索候选；可能严格不可行，不能直接当作有效档案 / Search-admissible candidate, possibly strictly infeasible and not automatically a valid archive entry
    pub candidate: Option<Individual<O, V>>,
}

/// 一次片段的执行报告 / Execution report for one slice
#[derive(Debug, Clone)]
pub struct SearchStepResult<O, V>
where
    O: Clone,
    V: HeuristicValue,
{
    /// 已评估的有效候选 / Valid evaluated candidates
    pub candidates: Vec<Individual<O, V>>,
    /// 已消耗评估请求 / Consumed evaluation requests
    pub evaluations: u64,
    /// 完整提交的步数 / Fully committed steps
    pub steps: u64,
    /// 片段终止原因 / Slice termination
    pub termination: SearchTermination,
    /// 步进失败诊断 / Step failure diagnostics
    pub diagnostics: Vec<String>,
}

impl<O, V> SearchStepResult<O, V>
where
    O: Clone,
    V: HeuristicValue,
{
    /// 创建空报告 / Create an empty report
    pub fn empty(termination: SearchTermination) -> Self {
        Self {
            candidates: Vec::new(),
            evaluations: 0,
            steps: 0,
            termination,
            diagnostics: Vec::new(),
        }
    }

    /// 片段是否完整执行 / Whether the slice completed
    pub fn is_completed(&self) -> bool {
        self.termination.is_completed()
    }
}

/// 会话快照，不保存调用栈 / Session snapshot without execution stacks
///
/// 字段名与 Kotlin `SearchSessionSnapshot` 逐字一致（含 `activeNanos` 的驼峰写法），且 `version`、`activeNanos`
/// 在取默认值时不输出，与 kotlinx 的 `encodeDefaults = false` 行为相同；因此两侧快照可以互相读取。
/// Field names match Kotlin's `SearchSessionSnapshot` verbatim (including the camel-case `activeNanos`), and `version` /
/// `activeNanos` are omitted when default, matching kotlinx's `encodeDefaults = false`, so snapshots are interchangeable.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSessionSnapshot {
    /// 协议版本 / Protocol version
    #[serde(default = "default_version", skip_serializing_if = "is_default_version")]
    pub version: u32,
    /// 是否已初始化 / Whether initialized
    pub initialized: bool,
    /// 迭代计数 / Iteration counters
    pub iteration: u64,
    /// 连续无改进计数 / Stagnation counters
    pub stagnation: u64,
    /// 算法原生状态 / Native algorithm state
    pub state: JsonValue,
    /// 局部档案 / Local archive
    pub archive: JsonValue,
    /// 独立原生约束状态，旧快照缺省为空 / Independent native constraint state, absent in old snapshots
    #[serde(default, skip_serializing_if = "JsonValue::is_null")]
    pub constraints: JsonValue,
    /// 有效运行纳秒数 / Active runtime in nanoseconds
    #[serde(default, skip_serializing_if = "is_zero")]
    pub active_nanos: u64,
}

fn default_version() -> u32 {
    SearchSessionSnapshot::VERSION
}

fn is_default_version(version: &u32) -> bool {
    *version == SearchSessionSnapshot::VERSION
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

impl SearchSessionSnapshot {
    /// 当前协议版本 / Current protocol version
    pub const VERSION: u32 = 1;
}

/// 可分片执行的搜索会话 / Sliceable search session
pub trait SearchSession<O, V>
where
    O: Clone,
    V: HeuristicValue,
{
    /// 是否能在步边界导入解 / Whether solutions can be imported at step boundaries
    fn supports_import(&self) -> bool {
        false
    }

    /// 执行预算片段 / Execute a budget slice
    fn step(&mut self, slice: &mut SearchSlice) -> Result<SearchStepResult<O, V>>;

    /// 在步进边界导入解 / Import solutions at a step boundary
    ///
    /// 默认实现直接拒绝，与 Kotlin 中「未提供 importer 即不支持导入」一致。
    /// The default implementation rejects the request, matching Kotlin's "no importer means no imports".
    fn import_solutions(
        &mut self,
        _solutions: Vec<Solution<V>>,
        _slice: &mut SearchSlice,
    ) -> Result<SearchStepResult<O, V>> {
        Err(HeuristicError::InvalidArgument(
            "session does not support imports / 会话不支持解导入".to_string(),
        ))
    }

    /// 获取完整状态 / Capture the complete state
    fn snapshot(&self) -> Result<SearchSessionSnapshot>;

    /// 恢复完整状态 / Restore the complete state
    fn restore(&mut self, snapshot: &SearchSessionSnapshot) -> Result<()>;

    /// 迭代计数 / Iteration counters
    fn iteration(&self) -> &Iteration;

    /// 释放会话资源 / Release session resources
    fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::sync::Arc;
    use crate::solver::heuristic::model::VariableModel;
    use crate::solver::heuristic::session::adapters::ProblemModel;
    use crate::solver::heuristic::session::SearchBudget;
    use crate::variable::new_standalone_id;
    use super::*;

    struct Domain {
        x: VariableId,
        feasible: bool,
        admission: Option<bool>,
        coordinates: Option<Vec<f64>>,
        violation: Option<f64>,
        violation_failed: bool,
        calls: Cell<u64>,
    }

    impl Default for Domain {
        fn default() -> Self {
            Self { x: new_standalone_id(), feasible: false, admission: None, coordinates: None,
                violation: None, violation_failed: false, calls: Cell::new(0) }
        }
    }

    impl SearchProblem<f64, f64> for Domain {
        fn id(&self) -> &str { "domain" }
        fn fingerprint(&self) -> &str { "domain:1" }
        fn variable_order(&self) -> Vec<VariableId> { vec![self.x] }
        fn initial_solutions(&self, _: usize) -> Vec<Solution<f64>> { Vec::new() }
        fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
            self.calls.set(self.calls.get() + 1);
            solution.get(&self.x).copied()
        }
        fn dominates(&self, lhs: &f64, rhs: &f64) -> bool { lhs > rhs }
        fn feasible(&self, _: &Solution<f64>) -> bool { self.feasible }
        fn admissible(&self, solution: &Solution<f64>) -> bool { self.admission.unwrap_or_else(|| self.feasible(solution)) }
        fn has_objective_coordinates(&self) -> bool { self.coordinates.is_some() }
        fn objective_coordinates(&self, _: &f64) -> Result<Vec<f64>> {
            self.coordinates.clone().ok_or_else(|| HeuristicError::InvalidArgument("no projection".into()))
        }
        fn has_constraint_violation(&self) -> bool { self.violation.is_some() || self.violation_failed }
        fn constraint_violation(&self, _: &Solution<f64>) -> Result<f64> {
            if self.violation_failed { return Err(HeuristicError::Model("domain violation failure".into())); }
            self.violation.ok_or_else(|| HeuristicError::InvalidArgument("no violation".into()))
        }
    }

    #[test]
    fn default_admission_stays_strict_and_denied_requests_do_not_call_objective() {
        let mut domain = Domain::default();
        let budget = Arc::new(SearchBudget::new(3));
        let slice = SearchSlice::new(budget.clone(), 3, 1);
        let solution = Solution::from([(domain.x, 4.0)]);
        assert!(domain.evaluate(solution.clone(), &slice).unwrap().candidate.is_none());
        domain.feasible = true;
        assert!(domain.evaluate(Solution::new(), &slice).unwrap().candidate.is_none());
        assert_eq!(domain.evaluate(solution.clone(), &slice).unwrap().candidate.unwrap().fitness, 4.0);
        assert!(!domain.evaluate(solution, &slice).unwrap().admitted);
        assert_eq!(budget.used(), 3);
        assert_eq!(budget.objective_calls(), 1);
        assert_eq!(domain.calls.get(), 1);
    }

    #[test]
    fn explicit_search_domain_preserves_objective_and_does_not_prove_feasibility() {
        let domain = Domain { admission: Some(true), violation: Some(0.0), ..Domain::default() };
        let solution = Solution::from([(domain.x, 7.0)]);
        let slice = SearchSlice::new(Arc::new(SearchBudget::new(1)), 1, 1);
        let candidate = domain.evaluate(solution.clone(), &slice).unwrap().candidate.unwrap();
        assert_eq!(candidate.fitness, 7.0);
        assert_eq!(domain.checked_constraint_violation(&solution).unwrap(), 0.0);
        assert!(!domain.feasible(&solution));
        assert_eq!(ProblemModel::new(&domain).constraint_satisfied(&solution), Some(true));
        assert_eq!(ProblemModel::new(&domain).constraint_satisfied(&Solution::new()), Some(false));
    }

    #[test]
    fn forbidden_domain_wins_even_when_strictly_feasible() {
        let domain = Domain { feasible: true, admission: Some(false), ..Domain::default() };
        let solution = Solution::from([(domain.x, 1.0)]);
        let budget = Arc::new(SearchBudget::new(1));
        let slice = SearchSlice::new(budget.clone(), 1, 1);
        assert!(domain.evaluate(solution.clone(), &slice).unwrap().candidate.is_none());
        assert_eq!(ProblemModel::new(&domain).constraint_satisfied(&solution), Some(false));
        assert_eq!(budget.used(), 1);
        assert_eq!(budget.objective_calls(), 0);
    }

    #[test]
    fn coordinates_are_explicit_and_checked_without_objective_evaluations() {
        let mut domain = Domain::default();
        assert!(!domain.has_objective_coordinates());
        assert!(domain.checked_objective_coordinates(&10.0).is_err());
        for coordinates in [vec![], vec![f64::NAN], vec![f64::INFINITY], vec![1.0, 2.0]] {
            domain.coordinates = Some(coordinates);
            assert!(domain.checked_objective_coordinates(&10.0).is_err());
        }
        domain.coordinates = Some(vec![-0.1]);
        assert_eq!(domain.checked_objective_coordinates(&10.0).unwrap(), vec![-0.1]);
        assert_eq!(domain.calls.get(), 0);
    }

    #[test]
    fn violation_rejects_missing_invalid_values_and_preserves_callback_errors() {
        let mut domain = Domain::default();
        let solution = Solution::from([(domain.x, 1.0)]);
        assert!(domain.checked_constraint_violation(&solution).is_err());
        for violation in [-0.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            domain.violation = Some(violation);
            assert!(domain.checked_constraint_violation(&solution).is_err());
        }
        domain.violation = Some(0.5);
        assert_eq!(domain.checked_constraint_violation(&solution).unwrap(), 0.5);
        assert!(domain.checked_constraint_violation(&Solution::new()).is_err());
        domain.violation_failed = true;
        assert!(domain.checked_constraint_violation(&solution).unwrap_err().to_string().contains("domain violation failure"));
        assert_eq!(domain.calls.get(), 0);
    }

    #[test]
    fn termination_reports_failure_only_for_failed_and_empty() {
        assert!(SearchTermination::StepLimit.is_completed());
        assert!(!SearchTermination::Failed.is_completed());
        assert!(!SearchTermination::NoCandidate.is_completed());
        assert_eq!(SearchTermination::TimeLimit.to_string(), "time-limit");
    }
}
