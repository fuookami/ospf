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
        let candidate = if self.valid_solution(&solution) && self.feasible(&solution) {
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
    /// 有效候选；不可行或无效时为空 / Valid candidate, absent for infeasible or invalid evaluations
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
    use super::*;

    #[test]
    fn termination_reports_failure_only_for_failed_and_empty() {
        assert!(SearchTermination::StepLimit.is_completed());
        assert!(!SearchTermination::Failed.is_completed());
        assert!(!SearchTermination::NoCandidate.is_completed());
        assert_eq!(SearchTermination::TimeLimit.to_string(), "time-limit");
    }
}
