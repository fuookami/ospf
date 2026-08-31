//! 统一求解报告 / Unified solve report.
//!
//! 本模块是正常求解终态的唯一公共表示。旧 `SolverOutput` 仍保留为兼容
//! facade，但新的算法和应用层应读取本模块中的正交状态、解、证明和统计。
//! This module is the single public representation for normal solve terminals. The
//! legacy `SolverOutput` remains as a compatibility facade, while new algorithms and
//! applications should consume the orthogonal status, solution, proof, and statistics.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{CoreError, Result, SolverError};
use crate::model::intermediate::LinearTriadModel;
use crate::solver::audit::evaluate_linear_solution;
use crate::solver::cancellation::SolveHandle;
use crate::solver::fingerprint::linear_model_fingerprint;
use crate::solver::output::{SolverOutput, SolverStatus};
use crate::solver::value::{SolveValue, SolveValueConversionPolicy};

/// 当前统一报告 schema 版本 / Current unified-report schema version.
pub const CURRENT_SOLVE_REPORT_SCHEMA_VERSION: &str = "1.0";

/// 统一报告 schema 版本类型 / Unified-report schema version type.
pub type SolveReportSchemaVersion = String;

fn attempt_epoch_ms(elapsed: Duration) -> (Option<u64>, Option<u64>) {
    let completed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64);
    let started = completed.map(|completed| {
        completed.saturating_sub(elapsed.as_millis().min(u64::MAX as u128) as u64)
    });
    (started, completed)
}

/// 问题的数学结论 / Mathematical conclusion of the problem.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProblemStatus {
    /// 存在经过校验的 incumbent / A validated incumbent exists.
    Feasible,
    /// 已证明不可行 / Proven infeasible.
    Infeasible,
    /// 已证明无界 / Proven unbounded.
    Unbounded,
    /// 不可行或无界，尚未消歧 / Infeasible or unbounded, not disambiguated.
    InfeasibleOrUnbounded,
    /// 尚不能形成数学结论 / No mathematical conclusion is currently available.
    Unknown,
}

/// 本次执行的终止原因 / Reason why this execution terminated.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminationReason {
    /// 正常完成 / Completed normally.
    Completed,
    /// 达到时间上限 / Time limit reached.
    TimeLimit,
    /// 达到节点上限 / Node limit reached.
    NodeLimit,
    /// 达到总节点上限 / Total node limit reached.
    TotalNodeLimit,
    /// 达到停滞节点上限 / Stall-node limit reached.
    StallNodeLimit,
    /// 达到迭代上限 / Iteration limit reached.
    IterationLimit,
    /// 达到解数量上限 / Solution limit reached.
    SolutionLimit,
    /// 达到最优解改进上限 / Best-solution limit reached.
    BestSolutionLimit,
    /// 达到 gap 上限 / Gap limit reached.
    GapLimit,
    /// 达到内存上限 / Memory limit reached.
    MemoryLimit,
    /// 达到工作量上限 / Work limit reached.
    WorkLimit,
    /// 达到目标界限 / Objective limit reached.
    ObjectiveLimit,
    /// 达到 cutoff / Cutoff reached.
    Cutoff,
    /// 达到重启上限 / Restart limit reached.
    RestartLimit,
    /// 求解器返回次优解 / Solver returned a suboptimal solution.
    Suboptimal,
    /// 被取消 / Cancelled.
    Cancelled,
    /// 被外部中断 / Interrupted.
    Interrupted,
    /// 数值失败 / Numerical failure.
    NumericalFailure,
    /// backend 启动后失败 / Backend failure after startup.
    BackendFailure,
    /// 终止原因未知 / Unknown termination reason.
    Unknown,
}

/// 解存在性和证明层级 / Solution presence and proof level.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SolutionPresence {
    /// 没有 incumbent / No incumbent.
    None,
    /// 存在可行 incumbent，但未证明最优 / Feasible incumbent without optimality proof.
    Incumbent,
    /// incumbent 具有可靠最优性证明 / Incumbent with a reliable optimality proof.
    Optimal,
}

/// 证明状态 / Proof status.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProofStatus {
    /// 没有证明 / No proof.
    None,
    /// backend 或算法声明了证明 / Proof was claimed by a backend or algorithm.
    Claimed,
    /// 证明来源和报告已通过合同校验 / Proof source and report passed contract checks.
    Verified,
}

/// 证明类型 / Proof kind.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProofKind {
    /// 最优性证明 / Optimality proof.
    Optimality,
    /// 不可行证明 / Infeasibility proof.
    Infeasibility,
    /// 无界证明 / Unboundedness proof.
    Unboundedness,
    /// 不可行或无界证明 / Infeasible-or-unbounded proof.
    InfeasibleOrUnbounded,
}

/// 证明可靠性 / Proof reliability.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProofReliability {
    /// 精确可靠 / Exact and reliable.
    Exact,
    /// backend 声明可靠且通过结构校验 / Backend-declared reliable proof passing structural checks.
    Reliable,
    /// 启发式证据，不得用于 exact gate / Heuristic evidence, not valid for exact gates.
    Heuristic,
    /// 可靠性未知 / Reliability is unknown.
    Unknown,
}

/// 证明完整度 / Proof completeness.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProofCompleteness {
    /// 完整 / Complete.
    Complete,
    /// 部分完成 / Partial.
    Partial,
    /// 未提供 / Unavailable.
    Unavailable,
}

/// 报告中的结构化 warning / Structured warning carried by a report.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolveWarning {
    /// 稳定 warning code / Stable warning code.
    pub code: String,
    /// 脱敏说明 / Redacted explanation.
    pub message: String,
}

impl SolveWarning {
    /// 创建 warning / Create a warning.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// 结构化诊断 issue / Structured diagnostic issue.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolveIssue {
    /// 稳定 issue code / Stable issue code.
    pub code: String,
    /// 脱敏 issue message / Redacted issue message.
    pub message: String,
}

impl SolveIssue {
    /// 创建 issue / Create an issue.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// 报告级稳定变量身份 / Report-level stable variable identity.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableVariableId(pub String);

impl std::fmt::Display for StableVariableId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<String> for StableVariableId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for StableVariableId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// 报告级稳定约束身份 / Report-level stable constraint identity.
///
/// 约束 ID 与变量 ID 分开命名，避免把约束的生命周期错误地绑定到
/// solver row、插入顺序或进程内地址。/ Constraint IDs are separate from variable IDs so
/// that constraint lifetime is not coupled to solver rows, insertion order, or process addresses.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableConstraintId(pub String);

impl std::fmt::Display for StableConstraintId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<String> for StableConstraintId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for StableConstraintId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// 统一求解解 / Unified solve solution.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct SolveSolution<V> {
    /// 单个领域 incumbent payload；算法层可使用该字段承载业务解 / Domain incumbent payload.
    pub value: Option<V>,
    /// 按 solver 列顺序排列的解向量 / Solution vector in solver-column order.
    pub values: Vec<V>,
    /// 按稳定变量 ID 排列的解值 / Values keyed by stable variable ID.
    pub stable_values: BTreeMap<StableVariableId, V>,
    /// typed 目标值 / Typed objective value.
    pub objective: Option<V>,
    /// solver adapter 的浮点目标快照 / Floating-point objective snapshot from the solver adapter.
    pub objective_value: Option<f64>,
    /// 线性对偶 / Linear dual solution.
    pub dual_solution: Option<Vec<V>>,
    /// 二次约束对偶 / Quadratic-constraint dual solution.
    pub quadratic_dual_solution: Option<Vec<V>>,
    /// 可选解池 / Optional solution pool.
    pub pool: Vec<Vec<V>>,
}

impl<V> SolveSolution<V> {
    /// 创建 solver 向量解 / Create a solver-vector solution.
    pub fn vector(values: Vec<V>) -> Self {
        Self {
            value: None,
            values,
            stable_values: BTreeMap::new(),
            objective: None,
            objective_value: None,
            dual_solution: None,
            quadratic_dual_solution: None,
            pool: Vec::new(),
        }
    }

    /// 创建领域 incumbent 解 / Create a domain incumbent solution.
    pub fn incumbent(value: V) -> Self {
        Self {
            value: Some(value),
            values: Vec::new(),
            stable_values: BTreeMap::new(),
            objective: None,
            objective_value: None,
            dual_solution: None,
            quadratic_dual_solution: None,
            pool: Vec::new(),
        }
    }

    /// 判断是否有任一形式的解 / Check whether any solution representation is present.
    pub fn has_values(&self) -> bool {
        self.value.is_some() || !self.values.is_empty() || !self.stable_values.is_empty()
    }
}

/// 统一求解证明 / Unified solve proof.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct SolveProof<V> {
    /// 证明类型 / Proof kind.
    pub kind: ProofKind,
    /// 证明状态 / Proof status.
    pub status: ProofStatus,
    /// 证明可靠性 / Proof reliability.
    pub reliability: ProofReliability,
    /// 证明完整度 / Proof completeness.
    pub completeness: ProofCompleteness,
    /// 外部证据引用 / External evidence reference.
    pub reference: Option<String>,
    /// 可选的数值证据 / Optional numeric evidence.
    pub evidence: Option<Vec<V>>,
}

impl<V> SolveProof<V> {
    /// 创建最优性证明 / Create an optimality proof.
    pub fn optimality() -> Self {
        Self {
            kind: ProofKind::Optimality,
            status: ProofStatus::Verified,
            reliability: ProofReliability::Exact,
            completeness: ProofCompleteness::Complete,
            reference: None,
            evidence: None,
        }
    }

    /// 创建不可行证明 / Create an infeasibility proof.
    pub fn infeasibility() -> Self {
        Self {
            kind: ProofKind::Infeasibility,
            status: ProofStatus::Verified,
            reliability: ProofReliability::Exact,
            completeness: ProofCompleteness::Complete,
            reference: None,
            evidence: None,
        }
    }

    /// 创建无界证明 / Create an unboundedness proof.
    pub fn unboundedness() -> Self {
        Self {
            kind: ProofKind::Unboundedness,
            status: ProofStatus::Verified,
            reliability: ProofReliability::Exact,
            completeness: ProofCompleteness::Complete,
            reference: None,
            evidence: None,
        }
    }

    /// 创建不可行或无界证明 / Create an infeasible-or-unbounded proof.
    pub fn infeasible_or_unbounded() -> Self {
        Self {
            kind: ProofKind::InfeasibleOrUnbounded,
            status: ProofStatus::Verified,
            reliability: ProofReliability::Exact,
            completeness: ProofCompleteness::Complete,
            reference: None,
            evidence: None,
        }
    }
}

/// 求解统计 / Solve statistics.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct SolveStatistics<V> {
    /// 求解耗时 / Solve duration.
    pub solve_time: Duration,
    /// 迭代数 / Iteration count.
    pub iterations: Option<usize>,
    /// 节点数 / Node count.
    pub nodes: Option<usize>,
    /// typed 最优下界 / Typed best bound.
    pub best_bound: Option<V>,
    /// backend 浮点下界快照 / Floating-point best-bound snapshot.
    pub best_bound_value: Option<f64>,
    /// 绝对 gap / Absolute gap.
    pub absolute_gap: Option<f64>,
    /// 相对 gap / Relative gap.
    pub relative_gap: Option<f64>,
    /// 解数量 / Solution count.
    pub solution_count: Option<usize>,
    /// backend 扩展统计 / Backend extension statistics.
    pub extensions: BTreeMap<String, String>,
}

impl<V> Default for SolveStatistics<V> {
    fn default() -> Self {
        Self {
            solve_time: Duration::ZERO,
            iterations: None,
            nodes: None,
            best_bound: None,
            best_bound_value: None,
            absolute_gap: None,
            relative_gap: None,
            solution_count: None,
            extensions: BTreeMap::new(),
        }
    }
}

/// 统一诊断 / Unified diagnostics.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct SolveDiagnostics<V> {
    /// 约束求值记录 / Constraint evaluations.
    pub constraint_evaluations: Vec<ConstraintEvaluation<V>>,
    /// 变量边界求值记录 / Variable-bound evaluations.
    pub variable_bound_evaluations: Vec<VariableBoundEvaluation<V>>,
    /// 不可行证据摘要 / Infeasibility evidence summary.
    pub infeasibility_evidence: Option<InfeasibilityEvidence>,
    /// 诊断 warning / Diagnostic warnings.
    pub warnings: Vec<SolveWarning>,
    /// 诊断 issue / Diagnostic issues.
    pub issues: Vec<SolveIssue>,
    /// backend 或算法扩展 / Backend or algorithm extensions.
    pub extensions: BTreeMap<String, String>,
}

/// 线性模型的稳定 solver 映射 / Stable solver mapping for a linear model.
pub type SolveModelMapping = crate::solver::audit::LinearModelMapping;

impl<V> Default for SolveDiagnostics<V> {
    fn default() -> Self {
        Self {
            constraint_evaluations: Vec::new(),
            variable_bound_evaluations: Vec::new(),
            infeasibility_evidence: None,
            warnings: Vec::new(),
            issues: Vec::new(),
            extensions: BTreeMap::new(),
        }
    }
}

/// 约束求值记录 / Constraint evaluation record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintEvaluation<V> {
    /// 稳定约束 ID / Stable constraint ID.
    pub constraint_id: String,
    /// 左侧值 / Left-hand side value.
    pub lhs: V,
    /// 右侧值 / Right-hand side value.
    pub rhs: V,
    /// 有符号残差 / Signed residual.
    pub residual: V,
    /// 违反量 / Violation.
    pub violation: V,
    /// 使用的容差 / Tolerance used.
    pub tolerance: V,
    /// 是否满足 / Whether satisfied.
    pub satisfied: bool,
}

/// 变量边界求值记录 / Variable-bound evaluation record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct VariableBoundEvaluation<V> {
    /// 稳定变量 ID / Stable variable ID.
    pub variable_id: StableVariableId,
    /// 变量值 / Variable value.
    pub value: V,
    /// 下界 / Lower bound.
    pub lower: Option<V>,
    /// 上界 / Upper bound.
    pub upper: Option<V>,
    /// 是否满足 / Whether satisfied.
    pub satisfied: bool,
}

/// 不可行证据来源 / Infeasibility evidence source.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InfeasibilityEvidenceSource {
    /// 原生 IIS / Native IIS.
    NativeIis,
    /// Farkas 证书 / Farkas certificate.
    Farkas,
    /// 删除过滤 / Deletion filtering.
    DeletionFilter,
    /// 弹性过滤 / Elastic filtering.
    ElasticFilter,
    /// 基于 CP snapshot 重建和复验 / CP snapshot rebuild and verification.
    RebuildVerification,
    /// 不可用 / Unavailable.
    Unavailable,
}

/// 不可行证据成员 / Member of an infeasibility explanation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InfeasibilityEvidenceMember {
    /// 原始模型约束 / Original model constraint.
    Constraint(String),
    /// 变量下界 / Variable lower bound.
    LowerBound(String),
    /// 变量上界 / Variable upper bound.
    UpperBound(String),
    /// 稀疏域或连续域成员 / Sparse-domain or contiguous-domain member.
    Domain(String),
    /// 区间成员 / Interval member.
    Interval(String),
    /// CP assumption / CP assumption.
    Assumption(String),
}

/// 冲突集合最小性状态 / Minimality state of a conflict set.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InfeasibilityMinimality {
    /// 已逐项删除复验 / Every member was deletion-checked.
    Irreducible,
    /// 只完成了部分删除复验 / Only part of deletion checking completed.
    Partial,
    /// 未请求或尚未检查 / Not requested or not checked.
    #[default]
    NotChecked,
}

/// 不可行证据摘要 / Infeasibility evidence summary.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfeasibilityEvidence {
    /// 证据来源 / Evidence source.
    pub source: InfeasibilityEvidenceSource,
    /// 证据是否可靠 / Whether evidence is reliable.
    pub reliability: ProofReliability,
    /// 证据是否完整 / Whether evidence is complete.
    pub completeness: ProofCompleteness,
    /// 稳定约束 ID / Stable constraint IDs.
    pub constraint_ids: BTreeSet<String>,
    /// 带类型的模型、域、区间和 assumption 成员 / Typed model, domain, interval, and assumption members.
    #[cfg_attr(feature = "serde", serde(default))]
    pub members: BTreeSet<InfeasibilityEvidenceMember>,
    /// 冲突集合最小性 / Conflict-set minimality.
    #[cfg_attr(feature = "serde", serde(default))]
    pub minimality: InfeasibilityMinimality,
    /// 诊断计算耗时 / Time spent computing the evidence.
    pub computation_time: Duration,
    /// 不可用原因 / Reason when unavailable.
    pub unavailable_reason: Option<String>,
}

impl InfeasibilityEvidence {
    /// 判断证据是否应优先于重建/过滤 fallback 保留 / Check whether evidence outranks rebuild or filtering fallbacks.
    ///
    /// 原生 IIS、Farkas 以及完整且精确的 CP 重建证据不能被后续诊断覆盖。/ Native IIS,
    /// Farkas, and complete exact CP rebuild evidence must not be overwritten by a later
    /// diagnostic fallback.
    pub fn is_authoritative(&self) -> bool {
        matches!(
            self.source,
            InfeasibilityEvidenceSource::NativeIis | InfeasibilityEvidenceSource::Farkas
        ) || (self.source == InfeasibilityEvidenceSource::RebuildVerification
            && self.reliability == ProofReliability::Exact
            && self.completeness == ProofCompleteness::Complete)
    }
}

/// 求解器执行来源 / Solver execution provenance.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolverProvenance {
    /// 稳定 solver ID / Stable solver ID.
    pub solver_id: String,
    /// backend 名称 / Backend name.
    pub backend_name: String,
    /// backend 版本 / Backend version.
    pub backend_version: Option<String>,
    /// plugin 版本 / Plugin version.
    pub plugin_version: Option<String>,
    /// 请求配置摘要 / Requested configuration summary.
    pub requested_configuration: BTreeMap<String, String>,
    /// 实际生效配置 / Effective configuration.
    pub effective_configuration: BTreeMap<String, String>,
    /// 线程数 / Thread count.
    pub thread_count: Option<usize>,
    /// 随机种子 / Random seed.
    pub random_seed: Option<u64>,
    /// 是否确定性 / Whether deterministic.
    pub deterministic: Option<bool>,
    /// 脱敏环境摘要 / Redacted environment summary.
    pub environment_summary: BTreeMap<String, String>,
}

impl Default for SolverProvenance {
    fn default() -> Self {
        Self {
            solver_id: "unknown".to_owned(),
            backend_name: "unknown".to_owned(),
            backend_version: None,
            plugin_version: None,
            requested_configuration: BTreeMap::new(),
            effective_configuration: BTreeMap::new(),
            thread_count: None,
            random_seed: None,
            deterministic: None,
            environment_summary: BTreeMap::new(),
        }
    }
}

/// 审计指纹 / Audit fingerprint.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditFingerprint {
    /// 指纹 schema 版本 / Fingerprint schema version.
    pub schema_version: SolveReportSchemaVersion,
    /// 摘要算法 / Digest algorithm.
    pub algorithm: String,
    /// 十六进制摘要 / Hex digest.
    pub value: String,
}

/// 模型指纹别名 / Model-fingerprint alias.
///
/// 证书消费者使用独立的语义名称，底层仍复用统一的审计指纹编码。
/// Certificate consumers use a semantic name while retaining the common audit-fingerprint encoding.
pub type ModelFingerprint = AuditFingerprint;

/// 报告使用的模型、配置和 solver 指纹 / Model, configuration, and solver fingerprints.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SolveFingerprints {
    /// 模型指纹 / Model fingerprint.
    pub model: Option<AuditFingerprint>,
    /// 实际配置指纹 / Effective-configuration fingerprint.
    pub configuration: Option<AuditFingerprint>,
    /// solver 环境指纹 / Solver-environment fingerprint.
    pub solver: Option<AuditFingerprint>,
}

/// Branch-and-Price 等算法的通用 trace / Generic algorithm trace for Branch-and-Price and similar algorithms.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SolveTrace {
    /// 已探索节点数 / Explored node count.
    pub nodes_explored: usize,
    /// 已剪枝节点数 / Pruned node count.
    pub nodes_pruned: usize,
    /// 活动节点数 / Active node count.
    pub active_nodes: usize,
    /// 全局有效下界 / Global valid lower bound.
    pub global_lower_bound: Option<f64>,
    /// incumbent 上界 / Incumbent upper bound.
    pub upper_bound: Option<f64>,
    /// 相对 gap / Relative gap.
    pub relative_gap: Option<f64>,
    /// 总迭代数 / Total iterations.
    pub total_iterations: usize,
    /// 定价调用数 / Pricing calls.
    pub pricing_calls: usize,
    /// 生成列数 / Generated columns.
    pub generated_columns: usize,
    /// 已耗时 / Elapsed duration.
    pub elapsed: Duration,
    /// 逐轮算法快照 / Per-iteration algorithm snapshots.
    #[cfg_attr(feature = "serde", serde(default))]
    pub iteration_snapshots: Vec<SolveIterationSnapshot>,
}

/// 统一报告中的逐轮算法快照 / Backend-neutral per-iteration algorithm snapshot.
///
/// 该结构只保存可审计的数值和计数，不承载任何算法自己的终态枚举。
/// This structure stores auditable values and counters without introducing an algorithm-specific terminal enum.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "serde", serde(default))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SolveIterationSnapshot {
    /// 轮次，从 1 开始 / One-based iteration number.
    pub iteration: usize,
    /// 稳定阶段名称 / Stable stage name.
    pub stage: String,
    /// 该轮目标值 / Objective value for the iteration.
    pub objective_value: Option<f64>,
    /// 该轮有效下界 / Valid best bound for the iteration.
    pub best_bound: Option<f64>,
    /// 该轮相对间隙 / Relative gap for the iteration.
    pub relative_gap: Option<f64>,
    /// 本轮新增切割或列数 / Cuts or columns added in this iteration.
    pub items_added: usize,
    /// 截止本轮累计切割或列数 / Cumulative cuts or columns through this iteration.
    pub items_total: usize,
    /// 证书引用 / Certificate reference.
    pub proof_reference: Option<String>,
    /// master attempt 身份 / Master-attempt identity.
    pub master_attempt_id: Option<String>,
    /// subproblem attempt 身份 / Subproblem-attempt identity.
    pub subproblem_attempt_id: Option<String>,
    /// master 问题结论 / Master problem conclusion.
    pub master_problem_status: Option<ProblemStatus>,
    /// master 终止原因 / Master termination reason.
    pub master_termination_reason: Option<TerminationReason>,
    /// subproblem 问题结论 / Subproblem problem conclusion.
    pub subproblem_problem_status: Option<ProblemStatus>,
    /// subproblem 终止原因 / Subproblem termination reason.
    pub subproblem_termination_reason: Option<TerminationReason>,
    /// bound 是否通过外层算法合同验证 / Whether the bound passed the outer-algorithm contract gate.
    pub bound_valid: Option<bool>,
}

/// 组合求解 attempt 的结论 / Conclusion of a combinatorial solve attempt.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SolveAttemptOutcome {
    /// backend 形成了报告 / The backend formed a report.
    Report,
    /// backend 在启动后失败 / The backend failed after startup.
    Failed,
    /// backend 被取消 / The backend was cancelled.
    Cancelled,
    /// attempt 尚未启动 / The attempt was not started.
    NotStarted,
}

/// attempt 中保存的结构化终态摘要 / Structured terminal summary stored by an attempt.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct SolveAttemptSummary {
    /// 数学问题结论 / Mathematical problem conclusion.
    pub problem_status: ProblemStatus,
    /// 执行终止原因 / Execution termination reason.
    pub termination_reason: TerminationReason,
    /// 解存在性 / Solution presence.
    pub solution_presence: SolutionPresence,
    /// incumbent 目标值 / Incumbent objective value.
    pub objective_value: Option<f64>,
    /// 当前有效下界 / Current valid bound.
    pub best_bound_value: Option<f64>,
    /// 相对 gap / Relative gap.
    pub relative_gap: Option<f64>,
}

/// 一次组合求解 attempt 的审计记录 / Audit record for one combinatorial solve attempt.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct SolveAttemptTrace {
    /// 稳定 attempt ID / Stable attempt ID.
    pub attempt_id: String,
    /// 父 attempt ID / Parent attempt ID.
    pub parent_attempt_id: Option<String>,
    /// solver 来源信息 / Solver provenance.
    pub provenance: SolverProvenance,
    /// attempt 开始时间（epoch milliseconds）/ Attempt start time in epoch milliseconds.
    pub started_at_epoch_ms: Option<u64>,
    /// wrapper 冻结 backend 结果的时间 / Time when the wrapper linearized the backend result.
    pub completed_at_epoch_ms: Option<u64>,
    /// attempt 耗时 / Attempt elapsed time.
    pub elapsed: Duration,
    /// attempt 结论类型 / Attempt conclusion type.
    pub outcome: SolveAttemptOutcome,
    /// 结构化终态摘要 / Structured terminal summary.
    pub summary: Option<SolveAttemptSummary>,
    /// 结构化失败信息 / Structured failure information.
    pub error: Option<SolveIssue>,
    /// 取消来源或原因 / Cancellation origin or reason.
    pub cancellation_reason: Option<String>,
}

impl SolveAttemptTrace {
    /// 创建成功报告 attempt / Create an attempt containing a report.
    pub fn from_report(
        attempt_id: impl Into<String>,
        provenance: SolverProvenance,
        report: &SolveReport<f64>,
        elapsed: Duration,
    ) -> Self {
        Self::from_report_with_parent(attempt_id, None, provenance, report, elapsed)
    }

    /// 创建带父身份的成功报告 attempt / Create a successful attempt with a parent identity.
    pub fn from_report_with_parent(
        attempt_id: impl Into<String>,
        parent_attempt_id: Option<String>,
        provenance: SolverProvenance,
        report: &SolveReport<f64>,
        elapsed: Duration,
    ) -> Self {
        let (started_at_epoch_ms, completed_at_epoch_ms) = attempt_epoch_ms(elapsed);
        Self {
            attempt_id: attempt_id.into(),
            parent_attempt_id,
            provenance,
            started_at_epoch_ms,
            completed_at_epoch_ms,
            elapsed,
            outcome: attempt_outcome(report.termination_reason),
            summary: Some(SolveAttemptSummary {
                problem_status: report.problem_status,
                termination_reason: report.termination_reason,
                solution_presence: report.solution_presence,
                objective_value: report
                    .solution
                    .as_ref()
                    .and_then(|solution| solution.objective_value.or(solution.objective)),
                best_bound_value: report.statistics.best_bound_value,
                relative_gap: report.statistics.relative_gap,
            }),
            error: None,
            cancellation_reason: report
                .diagnostics
                .extensions
                .get("cancellation.origin")
                .cloned(),
        }
    }

    /// 创建失败 attempt / Create a failed attempt.
    pub fn failed(
        attempt_id: impl Into<String>,
        provenance: SolverProvenance,
        error: SolveIssue,
        elapsed: Duration,
    ) -> Self {
        Self::failed_with_parent(attempt_id, None, provenance, error, elapsed)
    }

    /// 创建带父身份的失败 attempt / Create a failed attempt with a parent identity.
    pub fn failed_with_parent(
        attempt_id: impl Into<String>,
        parent_attempt_id: Option<String>,
        provenance: SolverProvenance,
        error: SolveIssue,
        elapsed: Duration,
    ) -> Self {
        let (started_at_epoch_ms, completed_at_epoch_ms) = attempt_epoch_ms(elapsed);
        Self {
            attempt_id: attempt_id.into(),
            parent_attempt_id,
            provenance,
            started_at_epoch_ms,
            completed_at_epoch_ms,
            elapsed,
            outcome: SolveAttemptOutcome::Failed,
            summary: None,
            error: Some(error),
            cancellation_reason: None,
        }
    }
}

fn attempt_outcome(termination_reason: TerminationReason) -> SolveAttemptOutcome {
    if matches!(
        termination_reason,
        TerminationReason::Cancelled | TerminationReason::Interrupted
    ) {
        SolveAttemptOutcome::Cancelled
    } else {
        SolveAttemptOutcome::Report
    }
}

/// 组合求解的统一报告 / Unified report for a combinatorial solve.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct CombinatorialSolveReport<V> {
    /// 聚合后的最终报告 / Aggregated final report.
    pub report: SolveReport<V>,
    /// 所有已执行 attempt / All executed attempts.
    pub attempts: Vec<SolveAttemptTrace>,
    /// 被选择的 attempt ID / Selected attempt ID.
    pub selected_attempt_id: Option<String>,
    /// 选择依据 / Selection rationale.
    pub selection_reason: Option<String>,
}

impl<V: SolveValue> CombinatorialSolveReport<V> {
    /// 创建单 attempt 组合报告 / Create a single-attempt combinatorial report.
    pub fn single(report: SolveReport<V>, attempt_id: impl Into<String>) -> Self {
        Self::single_with_parent(report, attempt_id, None)
    }

    /// 创建带父身份的单 attempt 报告 / Create a single-attempt report with a parent identity.
    pub fn single_with_parent(
        report: SolveReport<V>,
        attempt_id: impl Into<String>,
        parent_attempt_id: Option<String>,
    ) -> Self {
        let attempt_id = attempt_id.into();
        let (started_at_epoch_ms, completed_at_epoch_ms) =
            attempt_epoch_ms(report.statistics.solve_time);
        let attempt = SolveAttemptTrace {
            attempt_id: attempt_id.clone(),
            parent_attempt_id,
            provenance: report.provenance.clone(),
            started_at_epoch_ms,
            completed_at_epoch_ms,
            elapsed: report.statistics.solve_time,
            outcome: attempt_outcome(report.termination_reason),
            summary: Some(SolveAttemptSummary {
                problem_status: report.problem_status,
                termination_reason: report.termination_reason,
                solution_presence: report.solution_presence,
                objective_value: report.solution.as_ref().and_then(|solution| {
                    solution.objective_value.or_else(|| {
                        solution.objective.as_ref().and_then(|objective| {
                            objective
                                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                                .ok()
                        })
                    })
                }),
                best_bound_value: report.statistics.best_bound_value,
                relative_gap: report.statistics.relative_gap,
            }),
            error: None,
            cancellation_reason: report
                .diagnostics
                .extensions
                .get("cancellation.origin")
                .cloned(),
        };
        Self {
            report,
            attempts: vec![attempt],
            selected_attempt_id: Some(attempt_id),
            selection_reason: Some("single attempt".to_owned()),
        }
    }

    /// 校验聚合报告 / Validate the aggregate report.
    pub fn validate(&self) -> Result<()> {
        self.report.validate()?;
        let mut ids = BTreeSet::new();
        let mut aggregate_parent_id: Option<&str> = None;
        for attempt in &self.attempts {
            if !ids.insert(attempt.attempt_id.as_str()) {
                return Err(invalid_report("combinatorial attempt IDs must be unique"));
            }
            if let Some(parent) = attempt.parent_attempt_id.as_deref() {
                if parent.is_empty() || parent == attempt.attempt_id {
                    return Err(invalid_report(
                        "combinatorial attempt parent identity must be non-empty and distinct",
                    ));
                }
                if let Some(existing_parent) = aggregate_parent_id
                    && existing_parent != parent
                {
                    return Err(invalid_report(
                        "combinatorial attempts must share one aggregate parent identity",
                    ));
                }
                aggregate_parent_id = Some(parent);
            }
            if let (Some(started), Some(completed)) =
                (attempt.started_at_epoch_ms, attempt.completed_at_epoch_ms)
                && completed < started
            {
                return Err(invalid_report(
                    "attempt completedAt must not precede startedAt",
                ));
            }
            if matches!(
                attempt.outcome,
                SolveAttemptOutcome::Report | SolveAttemptOutcome::Cancelled
            ) && attempt.summary.is_none()
            {
                return Err(invalid_report(
                    "report attempts must carry a terminal summary",
                ));
            }
            if matches!(attempt.outcome, SolveAttemptOutcome::Failed) && attempt.error.is_none() {
                return Err(invalid_report(
                    "failed attempts must carry a structured error",
                ));
            }
        }
        if let Some(selected) = &self.selected_attempt_id {
            let selected_attempt = self
                .attempts
                .iter()
                .find(|attempt| attempt.attempt_id == *selected)
                .ok_or_else(|| {
                    invalid_report("selected attempt ID must refer to an executed attempt")
                })?;
            if !matches!(
                selected_attempt.outcome,
                SolveAttemptOutcome::Report | SolveAttemptOutcome::Cancelled
            ) {
                return Err(invalid_report(
                    "selected attempt must contain a solver report",
                ));
            }
            let summary = selected_attempt
                .summary
                .as_ref()
                .ok_or_else(|| invalid_report("selected report attempt has no summary"))?;
            let report_objective = self.report.solution.as_ref().and_then(|solution| {
                solution.objective_value.or_else(|| {
                    solution.objective.as_ref().and_then(|objective| {
                        objective
                            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                            .ok()
                    })
                })
            });
            if summary.problem_status != self.report.problem_status
                || summary.termination_reason != self.report.termination_reason
                || summary.solution_presence != self.report.solution_presence
                || !approximately_equal_optional(summary.objective_value, report_objective)
                || !approximately_equal_optional(
                    summary.best_bound_value,
                    self.report.statistics.best_bound_value,
                )
                || !approximately_equal_optional(
                    summary.relative_gap,
                    self.report.statistics.relative_gap,
                )
            {
                return Err(invalid_report(
                    "selected attempt summary does not match aggregate report status or bounds",
                ));
            }
        } else if self.report.has_incumbent() {
            return Err(invalid_report(
                "an aggregate incumbent must identify its selected attempt",
            ));
        }
        Ok(())
    }
}

/// 统一求解报告 / Unified solve report.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct SolveReport<V> {
    /// 报告 schema 版本 / Report schema version.
    pub schema_version: String,
    /// 问题结论 / Problem conclusion.
    pub problem_status: ProblemStatus,
    /// 执行终止原因 / Execution termination reason.
    pub termination_reason: TerminationReason,
    /// 解存在性 / Solution presence.
    pub solution_presence: SolutionPresence,
    /// incumbent 和 solver 解 / Incumbent and solver solution.
    pub solution: Option<SolveSolution<V>>,
    /// 证明信息 / Proof information.
    pub proof: Option<SolveProof<V>>,
    /// 求解统计 / Solve statistics.
    pub statistics: SolveStatistics<V>,
    /// 结构化诊断 / Structured diagnostics.
    pub diagnostics: SolveDiagnostics<V>,
    /// 执行来源 / Execution provenance.
    pub provenance: SolverProvenance,
    /// 审计指纹 / Audit fingerprints.
    pub fingerprints: SolveFingerprints,
    /// 报告级模型元素映射 / Report-level model-element mapping.
    pub model_mapping: Option<SolveModelMapping>,
    /// 顶层 warning / Top-level warnings.
    pub warnings: Vec<SolveWarning>,
    /// 算法 trace / Algorithm trace.
    pub trace: SolveTrace,
}

impl<V> SolveReport<V> {
    /// 创建受校验报告构建器 / Create a validated report builder.
    pub fn builder(
        problem_status: ProblemStatus,
        termination_reason: TerminationReason,
    ) -> SolveReportBuilder<V> {
        SolveReportBuilder {
            report: Self {
                schema_version: CURRENT_SOLVE_REPORT_SCHEMA_VERSION.to_owned(),
                problem_status,
                termination_reason,
                solution_presence: SolutionPresence::None,
                solution: None,
                proof: None,
                statistics: SolveStatistics::default(),
                diagnostics: SolveDiagnostics::default(),
                provenance: SolverProvenance::default(),
                fingerprints: SolveFingerprints::default(),
                model_mapping: None,
                warnings: Vec::new(),
                trace: SolveTrace::default(),
            },
        }
    }

    /// 校验报告不变量 / Validate report invariants.
    pub fn validate(&self) -> Result<()> {
        if let Some(mapping) = &self.model_mapping {
            mapping.validate()?;
        }
        let has_solution = self.solution.is_some();
        if has_solution == matches!(self.solution_presence, SolutionPresence::None) {
            return Err(invalid_report(
                "solution presence does not match the supplied incumbent",
            ));
        }
        match self.problem_status {
            ProblemStatus::Infeasible
            | ProblemStatus::Unbounded
            | ProblemStatus::InfeasibleOrUnbounded => {
                if has_solution {
                    return Err(invalid_report(
                        "an infeasible or unbounded report cannot carry an incumbent",
                    ));
                }
            }
            ProblemStatus::Feasible if !has_solution => {
                return Err(invalid_report(
                    "a feasible report must carry a validated incumbent",
                ));
            }
            ProblemStatus::Unknown | ProblemStatus::Feasible => {}
        }
        if let Some(proof) = &self.proof {
            if proof.status == ProofStatus::None {
                return Err(invalid_report(
                    "a supplied proof must have Claimed or Verified status",
                ));
            }
            if proof.status == ProofStatus::Verified
                && (proof.completeness != ProofCompleteness::Complete
                    || matches!(
                        proof.reliability,
                        ProofReliability::Heuristic | ProofReliability::Unknown
                    )
                    || !matches!(self.termination_reason, TerminationReason::Completed))
            {
                return Err(invalid_report(
                    "a verified proof must be complete, reliable, and completed",
                ));
            }
            if self.solution_presence == SolutionPresence::Optimal
                && (!has_solution
                    || !matches!(self.problem_status, ProblemStatus::Feasible)
                    || !matches!(self.termination_reason, TerminationReason::Completed)
                    || proof.kind != ProofKind::Optimality
                    || proof.status != ProofStatus::Verified
                    || !matches!(
                        proof.reliability,
                        ProofReliability::Exact | ProofReliability::Reliable
                    )
                    || proof.completeness != ProofCompleteness::Complete)
            {
                return Err(invalid_report(
                    "optimal solution presence requires a complete verified optimality proof",
                ));
            }
            match proof.kind {
                ProofKind::Optimality => {
                    if !matches!(self.problem_status, ProblemStatus::Feasible)
                        || (proof.status == ProofStatus::Verified
                            && (!has_solution
                                || matches!(
                                    proof.reliability,
                                    ProofReliability::Heuristic | ProofReliability::Unknown
                                )))
                    {
                        return Err(invalid_report(
                            "optimality proof requires a feasible incumbent",
                        ));
                    }
                }
                ProofKind::Infeasibility => {
                    if !matches!(self.problem_status, ProblemStatus::Infeasible) {
                        return Err(invalid_report(
                            "infeasibility proof requires an infeasible problem status",
                        ));
                    }
                }
                ProofKind::Unboundedness => {
                    if !matches!(self.problem_status, ProblemStatus::Unbounded) {
                        return Err(invalid_report(
                            "unboundedness proof requires an unbounded problem status",
                        ));
                    }
                }
                ProofKind::InfeasibleOrUnbounded => {
                    if !matches!(self.problem_status, ProblemStatus::InfeasibleOrUnbounded) {
                        return Err(invalid_report(
                            "infeasible-or-unbounded proof requires the matching problem status",
                        ));
                    }
                }
            }
        }
        if self.solution_presence == SolutionPresence::Optimal && self.proof.is_none() {
            return Err(invalid_report(
                "optimal solution presence requires an optimality proof",
            ));
        }
        for value in [
            self.solution
                .as_ref()
                .and_then(|solution| solution.objective_value),
            self.statistics.best_bound_value,
            self.statistics.absolute_gap,
            self.statistics.relative_gap,
            self.trace.global_lower_bound,
            self.trace.upper_bound,
            self.trace.relative_gap,
        ]
        .into_iter()
        .flatten()
        {
            if !value.is_finite() {
                return Err(invalid_report(
                    "report statistics contain a non-finite value",
                ));
            }
        }
        for gap in [
            self.statistics.absolute_gap,
            self.statistics.relative_gap,
            self.trace.relative_gap,
        ]
        .into_iter()
        .flatten()
        {
            if gap < 0.0 {
                return Err(invalid_report("report gap cannot be negative"));
            }
        }

        for (index, snapshot) in self.trace.iteration_snapshots.iter().enumerate() {
            let expected_iteration = index.saturating_add(1);
            if snapshot.iteration != expected_iteration {
                return Err(invalid_report(
                    "iteration trace must use contiguous one-based iteration numbers",
                ));
            }
            for value in [
                snapshot.objective_value,
                snapshot.best_bound,
                snapshot.relative_gap,
            ]
            .into_iter()
            .flatten()
            {
                if !value.is_finite() {
                    return Err(invalid_report(
                        "iteration trace contains a non-finite value",
                    ));
                }
            }
            if snapshot.relative_gap.is_some_and(|gap| gap < 0.0) {
                return Err(invalid_report("iteration trace gap cannot be negative"));
            }
            if snapshot.stage.trim().is_empty() {
                return Err(invalid_report("iteration trace stage cannot be blank"));
            }
            if snapshot.items_total < snapshot.items_added {
                return Err(invalid_report(
                    "iteration trace cumulative item count cannot be below the increment",
                ));
            }
            if let Some(previous) = index
                .checked_sub(1)
                .and_then(|previous| self.trace.iteration_snapshots.get(previous))
                && snapshot.items_total < previous.items_total
            {
                return Err(invalid_report(
                    "iteration trace cumulative item count cannot decrease",
                ));
            }
            if let (Some(objective), Some(best_bound), Some(relative_gap)) = (
                snapshot.objective_value,
                snapshot.best_bound,
                snapshot.relative_gap,
            ) {
                let expected_relative_gap =
                    (objective - best_bound).abs() / objective.abs().max(1.0);
                if !approximately_equal(relative_gap, expected_relative_gap) {
                    return Err(invalid_report(
                        "iteration trace gap does not match objective and best bound",
                    ));
                }
            }
        }

        let objective_value = self
            .solution
            .as_ref()
            .and_then(|solution| solution.objective_value);
        match (objective_value, self.statistics.best_bound_value) {
            (Some(objective), Some(best_bound)) => {
                let expected_absolute_gap = (objective - best_bound).abs();
                if let Some(actual_absolute_gap) = self.statistics.absolute_gap
                    && !approximately_equal(actual_absolute_gap, expected_absolute_gap)
                {
                    return Err(invalid_report(
                        "absolute gap does not match incumbent and best bound",
                    ));
                }
                if let Some(actual_relative_gap) = self.statistics.relative_gap
                    && !approximately_equal(
                        actual_relative_gap,
                        expected_absolute_gap / objective.abs().max(1.0),
                    )
                {
                    return Err(invalid_report(
                        "relative gap does not match incumbent and best bound",
                    ));
                }
            }
            _ => {
                if self.statistics.absolute_gap.is_some() || self.statistics.relative_gap.is_some()
                {
                    return Err(invalid_report(
                        "gap requires both incumbent objective and best bound",
                    ));
                }
            }
        }

        match (
            self.trace.upper_bound,
            self.trace.global_lower_bound,
            self.trace.relative_gap,
        ) {
            (Some(upper_bound), Some(lower_bound), relative_gap) => {
                let expected_relative_gap =
                    (upper_bound - lower_bound).abs() / upper_bound.abs().max(1.0);
                if let Some(actual_relative_gap) = relative_gap
                    && !approximately_equal(actual_relative_gap, expected_relative_gap)
                {
                    return Err(invalid_report(
                        "trace relative gap does not match trace bounds",
                    ));
                }
            }
            (_, _, Some(_)) => {
                return Err(invalid_report(
                    "trace relative gap requires both trace bounds",
                ));
            }
            _ => {}
        }
        Ok(())
    }

    /// 是否存在 incumbent / Whether an incumbent exists.
    pub fn has_incumbent(&self) -> bool {
        !matches!(self.solution_presence, SolutionPresence::None)
    }

    /// 是否具有可靠最优性证明 / Whether a reliable optimality proof exists.
    pub fn is_optimal(&self) -> bool {
        matches!(self.solution_presence, SolutionPresence::Optimal)
    }

    /// 获取领域 incumbent payload / Get the domain incumbent payload.
    pub fn incumbent(&self) -> Option<&V> {
        self.solution.as_ref()?.value.as_ref()
    }
}

/// 已验证的最优线性规划证书视图 / Verified optimal linear-programming certificate view.
#[derive(Debug, Clone, Copy)]
pub struct OptimalLinearProgrammingCertificate<'a, V> {
    /// 原始统一报告 / Original unified report.
    pub report: &'a SolveReport<V>,
    /// 与报告 incumbent 对应的对偶解 / Dual solution belonging to the report incumbent.
    pub dual: &'a [V],
}

/// 要求报告包含可用于精确定价的最优 LP 证书 / Require an optimal LP certificate suitable for exact pricing.
pub fn require_optimal_lp_certificate<V>(
    report: &SolveReport<V>,
) -> Result<OptimalLinearProgrammingCertificate<'_, V>> {
    require_optimal_lp_certificate_inner(report, None)
}

/// 要求已验证的不可行性证书 / Require a verified infeasibility certificate.
pub fn require_infeasibility_certificate<V>(report: &SolveReport<V>) -> Result<&SolveProof<V>> {
    require_infeasibility_certificate_inner(report, None)
}

/// 要求与指定模型指纹匹配的已验证不可行性证书 / Require a verified infeasibility certificate matching a model fingerprint.
pub fn require_infeasibility_certificate_for_model<'a, V>(
    report: &'a SolveReport<V>,
    expected_model: &ModelFingerprint,
) -> Result<&'a SolveProof<V>> {
    require_infeasibility_certificate_inner(report, Some(expected_model))
}

fn require_infeasibility_certificate_inner<'a, V>(
    report: &'a SolveReport<V>,
    expected_model: Option<&ModelFingerprint>,
) -> Result<&'a SolveProof<V>> {
    report.validate()?;
    if report.problem_status != ProblemStatus::Infeasible {
        return Err(invalid_report(
            "infeasibility certificate requires an infeasible problem status",
        ));
    }
    let proof = report
        .proof
        .as_ref()
        .ok_or_else(|| invalid_report("infeasibility certificate is missing"))?;
    if proof.kind != ProofKind::Infeasibility
        || proof.status != ProofStatus::Verified
        || !matches!(
            proof.reliability,
            ProofReliability::Exact | ProofReliability::Reliable
        )
        || proof.completeness != ProofCompleteness::Complete
    {
        return Err(invalid_report(
            "infeasibility proof is not reliable and complete",
        ));
    }
    if let Some(expected_model) = expected_model
        && report.fingerprints.model.as_ref() != Some(expected_model)
    {
        return Err(invalid_report(
            "infeasibility certificate model fingerprint does not match the requested model",
        ));
    }
    Ok(proof)
}

/// 要求与指定模型指纹匹配的最优 LP 证书 / Require an optimal LP certificate matching a model fingerprint.
pub fn require_optimal_lp_certificate_for_model<'a, V>(
    report: &'a SolveReport<V>,
    expected_model: &ModelFingerprint,
) -> Result<OptimalLinearProgrammingCertificate<'a, V>> {
    require_optimal_lp_certificate_inner(report, Some(expected_model))
}

/// 要求与线性模型完全一致的最优 LP 证书 / Require an optimal LP certificate fully validated against a linear model.
///
/// 该入口是精确定价、割生成和其它精确算法的公共门禁。除报告状态、证明、指纹和
/// 对偶维度外，它还重新计算 primal 可行性、目标值以及线性规划的对偶互补条件。
/// This is the public gate for exact pricing, cut generation, and other exact algorithms. In addition to report status, proof, fingerprint, and dual dimensions, it independently checks primal feasibility, objective consistency, and the LP dual complementary conditions.
pub fn require_optimal_lp_certificate_for_linear_model<'a>(
    report: &'a SolveReport<f64>,
    model: &LinearTriadModel,
) -> Result<OptimalLinearProgrammingCertificate<'a, f64>> {
    let expected_model = linear_model_fingerprint(model)?;
    let certificate = require_optimal_lp_certificate_for_model(report, &expected_model)?;
    let solution = certificate
        .report
        .solution
        .as_ref()
        .ok_or_else(|| invalid_report("optimal LP report has no linear solution"))?;
    if solution.values.len() != model.num_variables() {
        return Err(invalid_report(
            "optimal LP solution dimension does not match the linear model",
        ));
    }
    if certificate.dual.len() != model.num_constraints() {
        return Err(invalid_report(
            "optimal LP dual dimension does not match the linear model",
        ));
    }

    let diagnostics = evaluate_linear_solution(model, &solution.values, 1e-7)?;
    if diagnostics
        .constraint_evaluations
        .iter()
        .any(|evaluation| !evaluation.satisfied)
        || diagnostics
            .variable_bound_evaluations
            .iter()
            .any(|evaluation| !evaluation.satisfied)
    {
        return Err(invalid_report(
            "optimal LP incumbent failed independent linear-model feasibility evaluation",
        ));
    }

    let primal_objective = model
        .c
        .iter()
        .zip(solution.values.iter())
        .map(|(coefficient, value)| coefficient * value)
        .sum::<f64>();
    if !primal_objective.is_finite() {
        return Err(invalid_report("optimal LP primal objective is not finite"));
    }
    let reported_objective = solution
        .objective_value
        .or(solution.objective)
        .ok_or_else(|| invalid_report("optimal LP report has no objective value"))?;
    if !reported_objective.is_finite()
        || !approximately_equal_certificate(primal_objective, reported_objective)
    {
        return Err(invalid_report(
            "optimal LP report objective does not match the linear model and solution",
        ));
    }
    if !linear_dual_certificate_is_valid(model, &solution.values, certificate.dual) {
        return Err(invalid_report(
            "optimal LP dual fails independent sign, reduced-cost, or objective checks",
        ));
    }
    Ok(certificate)
}

const LINEAR_CERTIFICATE_TOLERANCE: f64 = 1e-7;

fn approximately_equal_certificate(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= LINEAR_CERTIFICATE_TOLERANCE * scale
}

fn valid_linear_bound_pair(lower: f64, upper: f64) -> bool {
    !(lower.is_nan()
        || upper.is_nan()
        || (lower.is_infinite() && lower.is_sign_positive())
        || (upper.is_infinite() && upper.is_sign_negative()))
        && lower <= upper
}

pub(crate) fn linear_dual_certificate_is_valid(
    model: &LinearTriadModel,
    values: &[f64],
    dual: &[f64],
) -> bool {
    if values.len() != model.num_variables()
        || dual.len() != model.num_constraints()
        || !values.iter().all(|value| value.is_finite())
        || !dual.iter().all(|value| value.is_finite())
    {
        return false;
    }

    let mut transposed_activity = vec![0.0; model.num_variables()];
    for (row_index, row) in model.basic.A.rows.iter().enumerate() {
        for (column, coefficient) in &row.entries {
            if *column >= transposed_activity.len() || !coefficient.is_finite() {
                return false;
            }
            transposed_activity[*column] += coefficient * dual[row_index];
        }
    }
    if transposed_activity.iter().any(|value| !value.is_finite()) {
        return false;
    }

    let dual_sign_is_valid = match model.objective_category {
        crate::model::ObjectiveCategory::Maximum => dual
            .iter()
            .all(|value| *value >= -LINEAR_CERTIFICATE_TOLERANCE),
        crate::model::ObjectiveCategory::Minimum => dual
            .iter()
            .all(|value| *value <= LINEAR_CERTIFICATE_TOLERANCE),
    };
    if !dual_sign_is_valid {
        return false;
    }

    let primal_objective = model
        .c
        .iter()
        .zip(values.iter())
        .map(|(coefficient, value)| coefficient * value)
        .sum::<f64>();
    let mut dual_objective = model
        .basic
        .b
        .iter()
        .zip(dual.iter())
        .map(|(rhs, dual_value)| rhs * dual_value)
        .sum::<f64>();
    if !primal_objective.is_finite() || !dual_objective.is_finite() {
        return false;
    }

    for column in 0..model.num_variables() {
        let lower = model.basic.lb[column];
        let upper = model.basic.ub[column];
        if !valid_linear_bound_pair(lower, upper) {
            return false;
        }
        let value = values[column];
        if (lower.is_finite() && value + LINEAR_CERTIFICATE_TOLERANCE < lower)
            || (upper.is_finite() && value - LINEAR_CERTIFICATE_TOLERANCE > upper)
        {
            return false;
        }

        let reduced_cost = model.c[column] - transposed_activity[column];
        if !reduced_cost.is_finite() {
            return false;
        }
        let at_lower = lower.is_finite() && (value - lower).abs() <= LINEAR_CERTIFICATE_TOLERANCE;
        let at_upper = upper.is_finite() && (value - upper).abs() <= LINEAR_CERTIFICATE_TOLERANCE;
        let reduced_cost_is_valid = if at_lower && at_upper {
            true
        } else if at_lower {
            match model.objective_category {
                crate::model::ObjectiveCategory::Maximum => {
                    reduced_cost <= LINEAR_CERTIFICATE_TOLERANCE
                }
                crate::model::ObjectiveCategory::Minimum => {
                    reduced_cost >= -LINEAR_CERTIFICATE_TOLERANCE
                }
            }
        } else if at_upper {
            match model.objective_category {
                crate::model::ObjectiveCategory::Maximum => {
                    reduced_cost >= -LINEAR_CERTIFICATE_TOLERANCE
                }
                crate::model::ObjectiveCategory::Minimum => {
                    reduced_cost <= LINEAR_CERTIFICATE_TOLERANCE
                }
            }
        } else {
            approximately_equal_certificate(reduced_cost, 0.0)
        };
        if !reduced_cost_is_valid {
            return false;
        }
        if at_lower || at_upper {
            dual_objective += reduced_cost * value;
        }
    }

    dual_objective.is_finite() && approximately_equal_certificate(primal_objective, dual_objective)
}

fn require_optimal_lp_certificate_inner<'a, V>(
    report: &'a SolveReport<V>,
    expected_model: Option<&ModelFingerprint>,
) -> Result<OptimalLinearProgrammingCertificate<'a, V>> {
    report.validate()?;
    if report.problem_status != ProblemStatus::Feasible
        || report.solution_presence != SolutionPresence::Optimal
    {
        return Err(invalid_report(
            "exact LP pricing requires a feasible report with optimal solution presence",
        ));
    }
    let proof = report
        .proof
        .as_ref()
        .ok_or_else(|| invalid_report("exact LP pricing requires an optimality proof"))?;
    if proof.kind != ProofKind::Optimality
        || proof.status != ProofStatus::Verified
        || !matches!(
            proof.reliability,
            ProofReliability::Exact | ProofReliability::Reliable
        )
        || proof.completeness != ProofCompleteness::Complete
    {
        return Err(invalid_report(
            "LP optimality proof is not reliable and complete",
        ));
    }
    if let Some(expected_model) = expected_model
        && report.fingerprints.model.as_ref() != Some(expected_model)
    {
        return Err(invalid_report(
            "LP certificate model fingerprint does not match the requested model",
        ));
    }
    let solution = report
        .solution
        .as_ref()
        .ok_or_else(|| invalid_report("optimal LP report has no solution"))?;
    let dual = solution
        .dual_solution
        .as_deref()
        .ok_or_else(|| invalid_report("optimal LP report has no dual solution"))?;
    if dual.is_empty() {
        return Err(invalid_report(
            "optimal LP report has an empty dual solution",
        ));
    }
    if let Some(mapping) = report.model_mapping.as_ref()
        && dual.len() != mapping.constraints_by_row.len()
    {
        return Err(invalid_report(
            "optimal LP dual dimension does not match the report model mapping",
        ));
    }
    Ok(OptimalLinearProgrammingCertificate { report, dual })
}

/// 统一报告构建器 / Unified report builder.
pub struct SolveReportBuilder<V> {
    report: SolveReport<V>,
}

impl<V> SolveReportBuilder<V> {
    /// 设置解 / Set the solution.
    pub fn solution(mut self, solution: SolveSolution<V>) -> Self {
        self.report.solution = Some(solution);
        self
    }

    /// 设置证明 / Set the proof.
    pub fn proof(mut self, proof: SolveProof<V>) -> Self {
        self.report.proof = Some(proof);
        self
    }

    /// 设置统计 / Set statistics.
    pub fn statistics(mut self, statistics: SolveStatistics<V>) -> Self {
        self.report.statistics = statistics;
        self
    }

    /// 设置诊断 / Set diagnostics.
    pub fn diagnostics(mut self, diagnostics: SolveDiagnostics<V>) -> Self {
        self.report.diagnostics = diagnostics;
        self
    }

    /// 设置来源 / Set provenance.
    pub fn provenance(mut self, provenance: SolverProvenance) -> Self {
        self.report.provenance = provenance;
        self
    }

    /// 设置指纹 / Set fingerprints.
    pub fn fingerprints(mut self, fingerprints: SolveFingerprints) -> Self {
        self.report.fingerprints = fingerprints;
        self
    }

    /// 设置模型元素映射 / Set the model-element mapping.
    pub fn model_mapping(mut self, model_mapping: Option<SolveModelMapping>) -> Self {
        self.report.model_mapping = model_mapping;
        self
    }

    /// 添加 warning / Add a warning.
    pub fn warning(mut self, warning: SolveWarning) -> Self {
        self.report.warnings.push(warning);
        self
    }

    /// 设置算法 trace / Set algorithm trace.
    pub fn trace(mut self, trace: SolveTrace) -> Self {
        self.report.trace = trace;
        self
    }

    /// 完成并校验报告 / Finish and validate the report.
    pub fn build(mut self) -> Result<SolveReport<V>> {
        self.report.solution_presence = match self.report.solution {
            None => SolutionPresence::None,
            Some(_) => {
                if self.report.proof.as_ref().is_some_and(|proof| {
                    proof.kind == ProofKind::Optimality
                        && proof.status == ProofStatus::Verified
                        && matches!(
                            proof.reliability,
                            ProofReliability::Exact | ProofReliability::Reliable
                        )
                }) {
                    SolutionPresence::Optimal
                } else {
                    SolutionPresence::Incumbent
                }
            }
        };
        self.report.validate()?;
        Ok(self.report)
    }
}

/// 将旧输出转换为统一报告 / Convert a legacy output into a unified report.
pub fn solver_output_to_report(output: SolverOutput) -> Result<SolveReport<f64>> {
    solver_output_to_report_inner(output, None, None)
}

/// 从取消句柄创建统一取消报告 / Create a unified cancelled report from a solve handle.
pub fn cancelled_solve_report(
    solver_provenance: SolverProvenance,
    handle: &SolveHandle,
) -> Result<SolveReport<f64>> {
    let cancellation = handle
        .cancellation()
        .ok_or_else(|| invalid_report("cancelled report requires a cancellation record"))?;
    let mut diagnostics = SolveDiagnostics::default();
    diagnostics.extensions.insert(
        "cancellation.origin".to_owned(),
        cancellation.origin.to_string(),
    );
    diagnostics.extensions.insert(
        "cancellation.requestedAtEpochMs".to_owned(),
        cancellation.requested_at_epoch_ms.to_string(),
    );
    SolveReport::builder(ProblemStatus::Unknown, TerminationReason::Cancelled)
        .diagnostics(diagnostics)
        .provenance(solver_provenance)
        .build()
}

/// 将求解器输出转换为统一报告，并附加 native provenance / Convert solver output into a unified report with native provenance.
pub fn solver_output_to_report_with_provenance(
    output: SolverOutput,
    provenance: SolverProvenance,
) -> Result<SolveReport<f64>> {
    solver_output_to_report_inner(output, Some(provenance), None)
}

/// 将 backend 输出转换为报告，并保留取消句柄的来源 / Convert backend output into a report while preserving the cancellation origin.
pub fn solver_output_to_report_with_cancellation(
    output: SolverOutput,
    provenance: SolverProvenance,
    handle: &SolveHandle,
) -> Result<SolveReport<f64>> {
    solver_output_to_report_inner(output, Some(provenance), Some(handle))
}

/// 将统一报告有损投影为旧 `SolverOutput` / Project a unified report into the legacy `SolverOutput` facade with documented loss.
pub fn solve_report_to_solver_output(report: &SolveReport<f64>) -> SolverOutput {
    let status = legacy_status_from_report(report).unwrap_or_else(|| {
        if matches!(report.problem_status, ProblemStatus::Infeasible) {
            SolverStatus::Infeasible
        } else if matches!(report.problem_status, ProblemStatus::Unbounded) {
            SolverStatus::Unbounded
        } else if matches!(report.problem_status, ProblemStatus::InfeasibleOrUnbounded) {
            SolverStatus::InfeasibleOrUnbounded
        } else if report.is_optimal() {
            SolverStatus::Optimal
        } else {
            match report.termination_reason {
                TerminationReason::Completed if report.has_incumbent() => SolverStatus::Feasible,
                TerminationReason::Completed => SolverStatus::Unknown,
                TerminationReason::TimeLimit => SolverStatus::TimeLimit,
                TerminationReason::NodeLimit => SolverStatus::NodeLimit,
                TerminationReason::TotalNodeLimit => SolverStatus::TotalNodeLimit,
                TerminationReason::StallNodeLimit => SolverStatus::StallNodeLimit,
                TerminationReason::IterationLimit => SolverStatus::IterationLimit,
                TerminationReason::SolutionLimit => SolverStatus::SolutionLimit,
                TerminationReason::BestSolutionLimit => SolverStatus::BestSolutionLimit,
                TerminationReason::GapLimit => SolverStatus::GapLimit,
                TerminationReason::MemoryLimit => SolverStatus::MemoryLimit,
                TerminationReason::WorkLimit => SolverStatus::WorkLimit,
                TerminationReason::ObjectiveLimit => SolverStatus::ObjectiveLimit,
                TerminationReason::Cutoff => SolverStatus::Cutoff,
                TerminationReason::RestartLimit => SolverStatus::RestartLimit,
                TerminationReason::Suboptimal => SolverStatus::Suboptimal,
                TerminationReason::Cancelled | TerminationReason::Interrupted => {
                    SolverStatus::UserInterrupt
                }
                TerminationReason::NumericalFailure => SolverStatus::NumericError,
                TerminationReason::BackendFailure | TerminationReason::Unknown => {
                    SolverStatus::Unknown
                }
            }
        }
    });
    let mut output = SolverOutput::new(status).with_time(report.statistics.solve_time);
    output.iterations = report.statistics.iterations;
    output.node_count = report.statistics.nodes;
    output.mip_gap = report.statistics.relative_gap;
    output.best_bound = report.statistics.best_bound_value;
    output.solution_count = report.statistics.solution_count;
    if let Some(solution) = &report.solution {
        output.objective_value = solution.objective_value.or(solution.objective);
        if !solution.values.is_empty() {
            output.solution = Some(solution.values.clone());
        }
        output.dual_solution = solution.dual_solution.clone();
        output.quadratic_dual_solution = solution.quadratic_dual_solution.clone();
    }
    output
}

fn legacy_status_from_report(report: &SolveReport<f64>) -> Option<SolverStatus> {
    let status = report.diagnostics.extensions.get("legacy.solverStatus")?;
    Some(match status.as_str() {
        "Optimal" => SolverStatus::Optimal,
        "Feasible" => SolverStatus::Feasible,
        "Infeasible" => SolverStatus::Infeasible,
        "InfeasibleOrUnbounded" => SolverStatus::InfeasibleOrUnbounded,
        "Unbounded" => SolverStatus::Unbounded,
        "IterationLimit" => SolverStatus::IterationLimit,
        "NodeLimit" => SolverStatus::NodeLimit,
        "TotalNodeLimit" => SolverStatus::TotalNodeLimit,
        "StallNodeLimit" => SolverStatus::StallNodeLimit,
        "TimeLimit" => SolverStatus::TimeLimit,
        "SolutionLimit" => SolverStatus::SolutionLimit,
        "BestSolutionLimit" => SolverStatus::BestSolutionLimit,
        "GapLimit" => SolverStatus::GapLimit,
        "MemoryLimit" => SolverStatus::MemoryLimit,
        "WorkLimit" => SolverStatus::WorkLimit,
        "ObjectiveLimit" => SolverStatus::ObjectiveLimit,
        "Cutoff" => SolverStatus::Cutoff,
        "RestartLimit" => SolverStatus::RestartLimit,
        "Suboptimal" => SolverStatus::Suboptimal,
        "NumericError" => SolverStatus::NumericError,
        "NotStarted" => SolverStatus::NotStarted,
        "Solving" => SolverStatus::Solving,
        "UserInterrupt" => SolverStatus::UserInterrupt,
        "Unknown" => SolverStatus::Unknown,
        _ => return None,
    })
}

fn validate_legacy_numeric_fields(output: &SolverOutput) -> Result<()> {
    let mut values = Vec::new();
    values.extend(output.objective_value);
    values.extend(output.mip_gap);
    values.extend(output.best_bound);
    if let Some(solution) = &output.solution {
        values.extend(solution.iter().copied());
    }
    if let Some(dual) = &output.dual_solution {
        values.extend(dual.iter().copied());
    }
    if let Some(dual) = &output.quadratic_dual_solution {
        values.extend(dual.iter().copied());
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(invalid_report(
            "legacy solver output contains a non-finite numeric value",
        ));
    }
    if output.mip_gap.is_some_and(|gap| gap < 0.0) {
        return Err(invalid_report(
            "legacy solver output gap cannot be negative",
        ));
    }
    Ok(())
}

fn solver_output_to_report_inner(
    output: SolverOutput,
    provenance: Option<SolverProvenance>,
    cancellation_handle: Option<&SolveHandle>,
) -> Result<SolveReport<f64>> {
    validate_legacy_numeric_fields(&output)?;
    let SolverOutput {
        status,
        objective_value,
        solution,
        dual_solution,
        quadratic_dual_solution,
        solve_time,
        iterations,
        node_count,
        mip_gap,
        best_bound,
        solution_count,
    } = output;
    let (problem_status, termination_reason, proof) = map_legacy_status_with_cancellation(
        status,
        solution.is_some(),
        cancellation_handle.is_some_and(SolveHandle::cancellation_preceded_completion),
    );
    let native_farkas = provenance.is_some()
        && problem_status == ProblemStatus::Infeasible
        && dual_solution
            .as_ref()
            .is_some_and(|values| !values.is_empty());
    let mut builder = SolveReport::builder(problem_status, termination_reason);
    let mut warnings = Vec::new();
    if matches!(status, SolverStatus::Feasible | SolverStatus::Optimal) && solution.is_none() {
        warnings.push(SolveWarning::new(
            "LegacyStatusWithoutIncumbent",
            "legacy solver status claimed feasibility without a solution vector",
        ));
    }
    if provenance.is_none() {
        warnings.push(SolveWarning::new(
            "LegacyStatusMapping",
            "terminal status was inferred from the compatibility SolverOutput facade; native adapters must use report-first APIs",
        ));
    }
    let relative_gap = objective_value
        .zip(best_bound)
        .map(|(objective, bound)| (objective - bound).abs() / objective.abs().max(1.0));
    if let (Some(native_gap), Some(expected_gap)) = (mip_gap, relative_gap)
        && !approximately_equal(native_gap, expected_gap)
    {
        return Err(invalid_report(
            "native mip gap does not match incumbent and best bound",
        ));
    }
    if mip_gap.is_some() && relative_gap.is_none() {
        warnings.push(SolveWarning::new(
            "LegacyGapWithoutBound",
            "legacy mip gap was omitted because incumbent objective and best bound were not both available",
        ));
    }
    let statistics = SolveStatistics {
        solve_time,
        iterations,
        nodes: node_count,
        best_bound,
        best_bound_value: best_bound,
        absolute_gap: objective_value
            .zip(best_bound)
            .map(|(objective, bound)| (objective - bound).abs()),
        relative_gap,
        solution_count: solution_count.or_else(|| solution.as_ref().map(|_| 1)),
        extensions: BTreeMap::new(),
    };
    builder = builder.statistics(statistics);
    if let Some(provenance) = provenance {
        builder = builder.provenance(provenance);
    }
    if let Some(handle) = cancellation_handle
        && let Some(cancellation) = handle.cancellation()
        && matches!(status, SolverStatus::UserInterrupt)
    {
        let mut diagnostics = SolveDiagnostics::default();
        diagnostics.extensions.insert(
            "cancellation.origin".to_owned(),
            cancellation.origin.to_string(),
        );
        diagnostics.extensions.insert(
            "cancellation.requestedAtEpochMs".to_owned(),
            cancellation.requested_at_epoch_ms.to_string(),
        );
        builder = builder.diagnostics(diagnostics);
    }
    for warning in warnings {
        builder = builder.warning(warning);
    }
    if let Some(values) = solution {
        let mut converted = SolveSolution::vector(values);
        converted.objective = objective_value;
        converted.objective_value = objective_value;
        converted.dual_solution = dual_solution.clone();
        converted.quadratic_dual_solution = quadratic_dual_solution;
        builder = builder.solution(converted);
    }
    if let Some(proof) = proof {
        builder = builder.proof(proof);
    } else if native_farkas {
        builder = builder.proof(SolveProof {
            kind: ProofKind::Infeasibility,
            status: ProofStatus::Claimed,
            reliability: ProofReliability::Reliable,
            completeness: ProofCompleteness::Complete,
            reference: Some("native-farkas".to_owned()),
            evidence: dual_solution,
        });
    }
    builder.build()
}

/// 将旧 `SolverOutput` 转成统一报告 / Convert legacy `SolverOutput` into a unified report.
impl SolverOutput {
    /// 生成统一报告 / Produce a unified report.
    pub fn try_into_solve_report(self) -> Result<SolveReport<f64>> {
        solver_output_to_report(self)
    }
}

fn map_legacy_status(
    status: SolverStatus,
    has_solution: bool,
) -> (ProblemStatus, TerminationReason, Option<SolveProof<f64>>) {
    map_legacy_status_with_cancellation(status, has_solution, false)
}

fn map_legacy_status_with_cancellation(
    status: SolverStatus,
    has_solution: bool,
    cancellation_requested: bool,
) -> (ProblemStatus, TerminationReason, Option<SolveProof<f64>>) {
    match status {
        // A legacy enum is not a proof source.  Native adapters promote the
        // corresponding terminal only after they have observed the backend
        // status directly; generic compatibility conversion must remain
        // conservative so custom/fake SolverOutput values cannot create an
        // exact certificate by naming a status.
        SolverStatus::Optimal if has_solution => {
            (ProblemStatus::Feasible, TerminationReason::Completed, None)
        }
        SolverStatus::Optimal => (
            ProblemStatus::Unknown,
            TerminationReason::BackendFailure,
            None,
        ),
        SolverStatus::Feasible if has_solution => {
            (ProblemStatus::Feasible, TerminationReason::Completed, None)
        }
        SolverStatus::Feasible => (ProblemStatus::Unknown, TerminationReason::Completed, None),
        SolverStatus::Infeasible => (
            ProblemStatus::Infeasible,
            TerminationReason::Completed,
            None,
        ),
        SolverStatus::InfeasibleOrUnbounded => (
            ProblemStatus::InfeasibleOrUnbounded,
            TerminationReason::Completed,
            None,
        ),
        SolverStatus::Unbounded => (
            ProblemStatus::Unbounded,
            TerminationReason::Completed,
            Some(SolveProof {
                kind: ProofKind::Unboundedness,
                status: ProofStatus::Claimed,
                reliability: ProofReliability::Unknown,
                completeness: ProofCompleteness::Unavailable,
                reference: None,
                evidence: None,
            }),
        ),
        SolverStatus::IterationLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::IterationLimit,
            None,
        ),
        SolverStatus::NodeLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::NodeLimit,
            None,
        ),
        SolverStatus::TotalNodeLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::TotalNodeLimit,
            None,
        ),
        SolverStatus::StallNodeLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::StallNodeLimit,
            None,
        ),
        SolverStatus::TimeLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::TimeLimit,
            None,
        ),
        SolverStatus::SolutionLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::SolutionLimit,
            None,
        ),
        SolverStatus::BestSolutionLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::BestSolutionLimit,
            None,
        ),
        SolverStatus::GapLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::GapLimit,
            None,
        ),
        SolverStatus::MemoryLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::MemoryLimit,
            None,
        ),
        SolverStatus::WorkLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::WorkLimit,
            None,
        ),
        SolverStatus::ObjectiveLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::ObjectiveLimit,
            None,
        ),
        SolverStatus::Cutoff => (ProblemStatus::Unknown, TerminationReason::Cutoff, None),
        SolverStatus::RestartLimit => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::RestartLimit,
            None,
        ),
        SolverStatus::Suboptimal => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::Suboptimal,
            None,
        ),
        SolverStatus::UserInterrupt => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            if cancellation_requested {
                TerminationReason::Cancelled
            } else {
                TerminationReason::Interrupted
            },
            None,
        ),
        SolverStatus::NumericError => (
            if has_solution {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            },
            TerminationReason::NumericalFailure,
            None,
        ),
        SolverStatus::NotStarted | SolverStatus::Solving => (
            ProblemStatus::Unknown,
            TerminationReason::BackendFailure,
            None,
        ),
        SolverStatus::Unknown => (ProblemStatus::Unknown, TerminationReason::Unknown, None),
    }
}

fn invalid_report(message: &str) -> CoreError {
    CoreError::Solver(SolverError::ContractViolation(format!(
        "invalid SolveReport: {}",
        message
    )))
}

fn approximately_equal(left: f64, right: f64) -> bool {
    let tolerance = 1e-8 * left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= tolerance
}

fn approximately_equal_optional(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => approximately_equal(left, right),
        (None, None) => true,
        _ => false,
    }
}

/// 将旧状态转换为兼容报告 / Convert a legacy status into a compatibility report.
pub fn legacy_status_to_report(
    status: SolverStatus,
    has_solution: bool,
) -> (ProblemStatus, TerminationReason) {
    let (problem_status, termination_reason, _) = map_legacy_status(status, has_solution);
    (problem_status, termination_reason)
}

/// 将 backend 原生终态提升为可消费的证明 / Promote a native terminal into a consumable proof.
///
/// `SolverOutput` 只是兼容 facade，不能单独产生 verified certificate。只有直接观察到
/// backend 原生终态的 adapter 才应调用本函数；未知或非证明终态保持原报告不变。
/// `SolverOutput` is only a compatibility facade and cannot create a verified certificate by itself. Only an adapter that observed the native backend terminal should call this function; unknown or non-certifying terminals leave the report unchanged.
pub fn verify_native_terminal_proof(
    report: &mut SolveReport<f64>,
    native_status: SolverStatus,
) -> Result<()> {
    match native_status {
        SolverStatus::Optimal
            if report.problem_status == ProblemStatus::Feasible
                && report.has_incumbent()
                && report.termination_reason == TerminationReason::Completed =>
        {
            report.proof = Some(SolveProof::optimality());
            report.solution_presence = SolutionPresence::Optimal;
        }
        SolverStatus::Infeasible
            if report.problem_status == ProblemStatus::Infeasible
                && !report.has_incumbent()
                && report.termination_reason == TerminationReason::Completed =>
        {
            let evidence = report
                .proof
                .as_ref()
                .and_then(|proof| proof.evidence.clone());
            report.proof = Some(SolveProof {
                evidence,
                ..SolveProof::infeasibility()
            });
        }
        SolverStatus::Unbounded
            if report.problem_status == ProblemStatus::Unbounded
                && !report.has_incumbent()
                && report.termination_reason == TerminationReason::Completed =>
        {
            report.proof = Some(SolveProof::unboundedness());
        }
        SolverStatus::InfeasibleOrUnbounded
            if report.problem_status == ProblemStatus::InfeasibleOrUnbounded
                && !report.has_incumbent()
                && report.termination_reason == TerminationReason::Completed =>
        {
            report.proof = Some(SolveProof::infeasible_or_unbounded());
        }
        _ => {}
    }
    report.validate()
}

/// 将旧 typed 输出转换为 typed 报告 / Convert a legacy typed output into a typed report.
pub fn convert_report_value<V>(
    report: SolveReport<f64>,
    policy: SolveValueConversionPolicy,
) -> Result<SolveReport<V>>
where
    V: SolveValue,
{
    let SolveReport {
        schema_version,
        problem_status,
        termination_reason,
        solution_presence: _,
        solution,
        proof,
        statistics,
        diagnostics,
        provenance,
        fingerprints,
        model_mapping,
        warnings,
        trace,
    } = report;
    let convert = |value: f64| V::from_f64_with_policy(value, policy);
    let solution = solution
        .map(|solution| {
            let value = solution.value.map(convert).transpose()?;
            let values = solution
                .values
                .into_iter()
                .map(&convert)
                .collect::<Result<Vec<_>>>()?;
            let stable_values = solution
                .stable_values
                .into_iter()
                .map(|(variable_id, value)| convert(value).map(|value| (variable_id, value)))
                .collect::<Result<BTreeMap<StableVariableId, V>>>()?;
            let objective = solution.objective.map(convert).transpose()?;
            let dual_solution = solution
                .dual_solution
                .map(|values| values.into_iter().map(&convert).collect::<Result<Vec<_>>>())
                .transpose()?;
            let quadratic_dual_solution = solution
                .quadratic_dual_solution
                .map(|values| values.into_iter().map(&convert).collect::<Result<Vec<_>>>())
                .transpose()?;
            let pool = solution
                .pool
                .into_iter()
                .map(|values| values.into_iter().map(&convert).collect::<Result<Vec<_>>>())
                .collect::<Result<Vec<_>>>()?;
            Ok::<SolveSolution<V>, CoreError>(SolveSolution {
                value,
                values,
                stable_values,
                objective,
                objective_value: solution.objective_value,
                dual_solution,
                quadratic_dual_solution,
                pool,
            })
        })
        .transpose()?;
    let constraint_evaluations = diagnostics
        .constraint_evaluations
        .into_iter()
        .map(|evaluation| {
            Ok::<ConstraintEvaluation<V>, CoreError>(ConstraintEvaluation {
                constraint_id: evaluation.constraint_id,
                lhs: convert(evaluation.lhs)?,
                rhs: convert(evaluation.rhs)?,
                residual: convert(evaluation.residual)?,
                violation: convert(evaluation.violation)?,
                tolerance: convert(evaluation.tolerance)?,
                satisfied: evaluation.satisfied,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let variable_bound_evaluations = diagnostics
        .variable_bound_evaluations
        .into_iter()
        .map(|evaluation| {
            let lower = evaluation.lower.map(convert).transpose()?;
            let upper = evaluation.upper.map(convert).transpose()?;
            Ok::<VariableBoundEvaluation<V>, CoreError>(VariableBoundEvaluation {
                variable_id: evaluation.variable_id,
                value: convert(evaluation.value)?,
                lower,
                upper,
                satisfied: evaluation.satisfied,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut builder = SolveReport::builder(problem_status, termination_reason)
        .statistics(SolveStatistics {
            solve_time: statistics.solve_time,
            iterations: statistics.iterations,
            nodes: statistics.nodes,
            best_bound: statistics.best_bound.map(convert).transpose()?,
            best_bound_value: statistics.best_bound_value,
            absolute_gap: statistics.absolute_gap,
            relative_gap: statistics.relative_gap,
            solution_count: statistics.solution_count,
            extensions: statistics.extensions,
        })
        .provenance(provenance)
        .fingerprints(fingerprints)
        .model_mapping(model_mapping)
        .trace(trace)
        .diagnostics(SolveDiagnostics {
            constraint_evaluations,
            variable_bound_evaluations,
            infeasibility_evidence: diagnostics.infeasibility_evidence,
            warnings: diagnostics.warnings,
            issues: diagnostics.issues,
            extensions: diagnostics.extensions,
        });
    if let Some(solution) = solution {
        builder = builder.solution(solution);
    }
    if let Some(proof) = proof {
        builder = builder.proof(SolveProof {
            kind: proof.kind,
            status: proof.status,
            reliability: proof.reliability,
            completeness: proof.completeness,
            reference: proof.reference,
            evidence: proof
                .evidence
                .map(|values| values.into_iter().map(convert).collect::<Result<Vec<_>>>())
                .transpose()?,
        });
    }
    for warning in warnings {
        builder = builder.warning(warning);
    }
    let mut converted = builder.build()?;
    converted.schema_version = schema_version;
    converted.validate()?;
    Ok(converted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ObjectiveCategory;
    use crate::model::intermediate::{BasicLinearTriadModel, SparseVector};
    use crate::solver::CancellationOrigin;
    use crate::token::Token;
    use crate::variable::{UContinuousVariableItem, VariableType};

    fn certificate_model() -> LinearTriadModel {
        let mut basic = BasicLinearTriadModel::new("certificate_model");
        basic.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );
        let mut row = SparseVector::new();
        row.add(0, 1.0);
        basic.add_constraint(row, 2.0);
        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![1.0], ObjectiveCategory::Maximum);
        model
    }

    #[test]
    fn report_keeps_limit_without_incumbent_as_unknown() {
        let report =
            SolveReport::<f64>::builder(ProblemStatus::Unknown, TerminationReason::TimeLimit)
                .build()
                .expect("limit without incumbent is valid");
        assert_eq!(report.solution_presence, SolutionPresence::None);
        assert!(!report.has_incumbent());
    }

    #[test]
    fn builder_rejects_feasible_without_incumbent() {
        let result =
            SolveReport::<f64>::builder(ProblemStatus::Feasible, TerminationReason::Completed)
                .build();
        assert!(result.is_err());
    }

    #[test]
    fn legacy_limit_does_not_invent_a_solution() {
        let report = SolverOutput::new(SolverStatus::TimeLimit)
            .try_into_solve_report()
            .expect("legacy limit should map to report");
        assert_eq!(report.problem_status, ProblemStatus::Unknown);
        assert_eq!(report.termination_reason, TerminationReason::TimeLimit);
        assert!(!report.has_incumbent());
    }

    #[test]
    fn report_rejects_gap_without_incumbent_bound_pair() {
        let no_incumbent =
            SolveReport::<f64>::builder(ProblemStatus::Unknown, TerminationReason::TimeLimit)
                .statistics(SolveStatistics {
                    relative_gap: Some(0.0),
                    ..SolveStatistics::default()
                })
                .build();
        assert!(no_incumbent.is_err());

        let no_bound =
            SolveReport::<f64>::builder(ProblemStatus::Feasible, TerminationReason::TimeLimit)
                .solution(SolveSolution {
                    objective: Some(3.0),
                    objective_value: Some(3.0),
                    ..SolveSolution::vector(vec![1.0])
                })
                .statistics(SolveStatistics {
                    relative_gap: Some(0.0),
                    ..SolveStatistics::default()
                })
                .build();
        assert!(no_bound.is_err());
    }

    #[test]
    fn report_rejects_trace_gap_without_trace_bounds() {
        let report =
            SolveReport::<f64>::builder(ProblemStatus::Feasible, TerminationReason::TimeLimit)
                .solution(SolveSolution {
                    objective: Some(3.0),
                    objective_value: Some(3.0),
                    ..SolveSolution::vector(vec![1.0])
                })
                .trace(SolveTrace {
                    upper_bound: Some(3.0),
                    relative_gap: Some(0.0),
                    ..SolveTrace::default()
                })
                .build();
        assert!(report.is_err());
    }

    #[test]
    fn report_rejects_invalid_iteration_trace_invariants() {
        let solution = SolveSolution {
            objective: Some(3.0),
            objective_value: Some(3.0),
            ..SolveSolution::vector(vec![1.0])
        };
        let invalid_number =
            SolveReport::<f64>::builder(ProblemStatus::Feasible, TerminationReason::Completed)
                .solution(solution.clone())
                .trace(SolveTrace {
                    iteration_snapshots: vec![SolveIterationSnapshot {
                        iteration: 2,
                        stage: "test".to_owned(),
                        objective_value: Some(3.0),
                        best_bound: Some(2.0),
                        relative_gap: Some(1.0 / 3.0),
                        items_added: 0,
                        items_total: 0,
                        proof_reference: None,
                        ..SolveIterationSnapshot::default()
                    }],
                    ..SolveTrace::default()
                })
                .build();
        assert!(invalid_number.is_err());

        let invalid_total =
            SolveReport::<f64>::builder(ProblemStatus::Feasible, TerminationReason::Completed)
                .solution(solution)
                .trace(SolveTrace {
                    iteration_snapshots: vec![
                        SolveIterationSnapshot {
                            iteration: 1,
                            stage: "test".to_owned(),
                            objective_value: None,
                            best_bound: None,
                            relative_gap: None,
                            items_added: 2,
                            items_total: 2,
                            proof_reference: None,
                            ..SolveIterationSnapshot::default()
                        },
                        SolveIterationSnapshot {
                            iteration: 2,
                            stage: "test".to_owned(),
                            objective_value: None,
                            best_bound: None,
                            relative_gap: None,
                            items_added: 0,
                            items_total: 1,
                            proof_reference: None,
                            ..SolveIterationSnapshot::default()
                        },
                    ],
                    ..SolveTrace::default()
                })
                .build();
        assert!(invalid_total.is_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn iteration_trace_round_trips_through_serde() {
        let report =
            SolveReport::<f64>::builder(ProblemStatus::Feasible, TerminationReason::Completed)
                .solution(SolveSolution {
                    objective: Some(3.0),
                    objective_value: Some(3.0),
                    ..SolveSolution::vector(vec![1.0])
                })
                .trace(SolveTrace {
                    iteration_snapshots: vec![SolveIterationSnapshot {
                        iteration: 1,
                        stage: "benders/master-subproblem".to_owned(),
                        objective_value: Some(3.0),
                        best_bound: Some(2.0),
                        relative_gap: Some(1.0 / 3.0),
                        items_added: 1,
                        items_total: 1,
                        proof_reference: Some("master-optimality-gate".to_owned()),
                        ..SolveIterationSnapshot::default()
                    }],
                    ..SolveTrace::default()
                })
                .build()
                .expect("report with a valid iteration trace should build");

        let encoded = serde_json::to_vec(&report).expect("report should serialize");
        let decoded: SolveReport<f64> =
            serde_json::from_slice(&encoded).expect("report should deserialize");
        assert_eq!(decoded, report);
    }

    #[test]
    fn legacy_gap_without_bound_is_dropped_with_warning() {
        let mut output = SolverOutput::new(SolverStatus::NodeLimit)
            .with_objective(3.0)
            .with_solution(vec![1.0]);
        output.mip_gap = Some(0.5);

        let report = output
            .try_into_solve_report()
            .expect("legacy report without a bound should remain representable");
        assert!(report.statistics.best_bound_value.is_none());
        assert!(report.statistics.relative_gap.is_none());
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.code == "LegacyGapWithoutBound")
        );
    }

    #[test]
    fn optimal_report_requires_verified_proof() {
        let report =
            SolveReport::<f64>::builder(ProblemStatus::Feasible, TerminationReason::Completed)
                .solution(SolveSolution::vector(vec![1.0]))
                .proof(SolveProof {
                    kind: ProofKind::Optimality,
                    status: ProofStatus::Claimed,
                    reliability: ProofReliability::Unknown,
                    completeness: ProofCompleteness::Partial,
                    reference: None,
                    evidence: None,
                })
                .build()
                .expect("a non-verified proof still permits a feasible incumbent");
        assert_eq!(report.solution_presence, SolutionPresence::Incumbent);
        assert!(!report.is_optimal());
    }

    #[test]
    fn legacy_projection_keeps_bound_and_solution_count() {
        let mut output = SolverOutput::new(SolverStatus::NodeLimit)
            .with_objective(3.0)
            .with_solution(vec![1.0])
            .with_solution_count(4);
        output.best_bound = Some(2.0);
        output.mip_gap = Some(1.0 / 3.0);

        let report = output
            .try_into_solve_report()
            .expect("legacy output should project");

        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(report.termination_reason, TerminationReason::NodeLimit);
        assert_eq!(report.statistics.best_bound, Some(2.0));
        assert_eq!(report.statistics.best_bound_value, Some(2.0));
        assert_eq!(report.statistics.solution_count, Some(4));
        assert_eq!(report.statistics.absolute_gap, Some(1.0));
    }

    #[test]
    fn limit_without_incumbent_preserves_native_work_statistics() {
        let mut output = SolverOutput::new(SolverStatus::NodeLimit)
            .with_iterations(17)
            .with_solution_count(0);
        output.node_count = Some(42);
        output.best_bound = Some(9.0);

        let report = solver_output_to_report_with_provenance(
            output,
            SolverProvenance {
                solver_id: "fake/native".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            },
        )
        .expect("a limit without an incumbent should remain representable");

        assert_eq!(report.problem_status, ProblemStatus::Unknown);
        assert_eq!(report.termination_reason, TerminationReason::NodeLimit);
        assert_eq!(report.statistics.iterations, Some(17));
        assert_eq!(report.statistics.nodes, Some(42));
        assert_eq!(report.statistics.best_bound_value, Some(9.0));
        assert_eq!(report.statistics.solution_count, Some(0));
        assert!(!report.has_incumbent());
    }

    #[test]
    fn legacy_projection_rejects_non_finite_numeric_fields() {
        let output = SolverOutput::optimal(f64::NAN, vec![1.0]);
        assert!(output.try_into_solve_report().is_err());
    }

    #[test]
    fn native_projection_keeps_provenance_without_legacy_warning() {
        let report = solver_output_to_report_with_provenance(
            SolverOutput::optimal(1.0, vec![1.0]),
            SolverProvenance {
                solver_id: "fake/native".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            },
        )
        .expect("native output should project");

        assert_eq!(report.provenance.solver_id, "fake/native");
        assert!(
            !report
                .warnings
                .iter()
                .any(|warning| warning.code == "LegacyStatusMapping")
        );
        assert!(
            !report.is_optimal(),
            "provenance alone must not turn a compatibility status into a proof"
        );
    }

    #[test]
    fn legacy_terminal_statuses_do_not_create_verified_certificates() {
        let optimal = SolverOutput::optimal(1.0, vec![1.0])
            .try_into_solve_report()
            .expect("legacy optimal output should project");
        assert!(!optimal.is_optimal());
        assert!(optimal.proof.is_none());

        let infeasible = SolverOutput::infeasible()
            .try_into_solve_report()
            .expect("legacy infeasible output should project");
        assert_eq!(infeasible.problem_status, ProblemStatus::Infeasible);
        assert!(infeasible.proof.is_none());
        assert!(require_infeasibility_certificate(&infeasible).is_err());
    }

    #[test]
    fn legacy_optimal_projection_is_explicitly_marked_as_lossy() {
        let report = SolverOutput::optimal(1.0, vec![1.0])
            .try_into_solve_report()
            .expect("legacy optimal output should project");
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.code == "LegacyStatusMapping")
        );
    }

    #[test]
    fn native_terminal_status_can_explicitly_promote_a_certificate() {
        let mut optimal = SolverOutput::optimal(1.0, vec![1.0])
            .try_into_solve_report()
            .expect("legacy payload should project before native promotion");
        verify_native_terminal_proof(&mut optimal, SolverStatus::Optimal)
            .expect("native optimal status should promote the report");
        assert!(optimal.is_optimal());

        let mut infeasible = SolverOutput::infeasible()
            .try_into_solve_report()
            .expect("legacy payload should project before native promotion");
        verify_native_terminal_proof(&mut infeasible, SolverStatus::Infeasible)
            .expect("native infeasible status should promote the report");
        assert!(require_infeasibility_certificate(&infeasible).is_ok());

        let mut unbounded = SolverOutput::unbounded()
            .try_into_solve_report()
            .expect("legacy unbounded payload should project before native promotion");
        verify_native_terminal_proof(&mut unbounded, SolverStatus::Unbounded)
            .expect("native unbounded status should promote the report");
        let unbounded_proof = unbounded
            .proof
            .as_ref()
            .expect("native unbounded report should carry a proof");
        assert_eq!(unbounded_proof.kind, ProofKind::Unboundedness);
        assert_eq!(unbounded_proof.status, ProofStatus::Verified);

        let mut ambiguous = SolverOutput::new(SolverStatus::InfeasibleOrUnbounded)
            .try_into_solve_report()
            .expect("legacy ambiguous payload should project before native promotion");
        verify_native_terminal_proof(&mut ambiguous, SolverStatus::InfeasibleOrUnbounded)
            .expect("native ambiguous status should promote the report");
        let ambiguous_proof = ambiguous
            .proof
            .as_ref()
            .expect("native ambiguous report should carry a proof");
        assert_eq!(ambiguous_proof.kind, ProofKind::InfeasibleOrUnbounded);
        assert_eq!(ambiguous_proof.status, ProofStatus::Verified);
    }

    #[test]
    fn typed_projection_preserves_domain_and_stable_values() {
        let mut stable_values = BTreeMap::new();
        stable_values.insert(StableVariableId::from("x"), 2.0);
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(SolveSolution {
                value: Some(7.0),
                values: vec![2.0],
                stable_values,
                objective: Some(3.0),
                objective_value: Some(3.0),
                dual_solution: None,
                quadratic_dual_solution: None,
                pool: vec![vec![2.0]],
            })
            .build()
            .expect("report should build");

        let converted = convert_report_value::<f64>(report, SolveValueConversionPolicy::Strict)
            .expect("typed conversion should succeed");
        let solution = converted.solution.expect("solution should be preserved");
        assert_eq!(solution.value, Some(7.0));
        assert_eq!(
            solution.stable_values.get(&StableVariableId::from("x")),
            Some(&2.0)
        );
        assert_eq!(solution.pool, vec![vec![2.0]]);
    }

    #[test]
    fn optimal_lp_certificate_requires_verified_dual() {
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(SolveSolution {
                value: None,
                values: vec![1.0],
                stable_values: BTreeMap::new(),
                objective: Some(2.0),
                objective_value: Some(2.0),
                dual_solution: Some(vec![3.0]),
                quadratic_dual_solution: None,
                pool: Vec::new(),
            })
            .proof(SolveProof::optimality())
            .build()
            .expect("optimal report should build");

        let certificate = require_optimal_lp_certificate(&report)
            .expect("verified optimal report should expose a certificate");
        assert_eq!(certificate.dual, &[3.0]);
        assert!(certificate.report.is_optimal());
    }

    #[test]
    fn infeasibility_certificate_requires_verified_complete_proof() {
        let report =
            SolveReport::<f64>::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .proof(SolveProof::infeasibility())
                .build()
                .expect("verified infeasibility report should build");
        assert!(require_infeasibility_certificate(&report).is_ok());

        let unverified_report =
            SolveReport::<f64>::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .proof(SolveProof {
                    status: ProofStatus::Claimed,
                    ..SolveProof::infeasibility()
                })
                .build()
                .expect("claimed infeasibility report should still build");
        assert!(require_infeasibility_certificate(&unverified_report).is_err());
    }

    #[test]
    fn infeasibility_certificate_requires_matching_model_fingerprint() {
        let model = AuditFingerprint {
            schema_version: CURRENT_SOLVE_REPORT_SCHEMA_VERSION.to_owned(),
            algorithm: "sha256".to_owned(),
            value: "infeasible-model-a".to_owned(),
        };
        let report =
            SolveReport::<f64>::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .proof(SolveProof::infeasibility())
                .fingerprints(SolveFingerprints {
                    model: Some(model.clone()),
                    ..SolveFingerprints::default()
                })
                .build()
                .expect("verified infeasibility report should build");

        require_infeasibility_certificate_for_model(&report, &model)
            .expect("matching model fingerprint should be accepted");
        let other_model = AuditFingerprint {
            value: "infeasible-model-b".to_owned(),
            ..model.clone()
        };
        assert!(require_infeasibility_certificate_for_model(&report, &other_model).is_err());
        let report_without_fingerprint =
            SolveReport::<f64>::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .proof(SolveProof::infeasibility())
                .build()
                .expect("report without a fingerprint should still be representable");
        assert!(
            require_infeasibility_certificate_for_model(&report_without_fingerprint, &model)
                .is_err()
        );
    }

    #[test]
    fn optimal_lp_certificate_can_require_matching_model_fingerprint() {
        let model = AuditFingerprint {
            schema_version: CURRENT_SOLVE_REPORT_SCHEMA_VERSION.to_owned(),
            algorithm: "sha256".to_owned(),
            value: "model-a".to_owned(),
        };
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(SolveSolution {
                value: None,
                values: vec![1.0],
                stable_values: BTreeMap::new(),
                objective: Some(2.0),
                objective_value: Some(2.0),
                dual_solution: Some(vec![3.0]),
                quadratic_dual_solution: None,
                pool: Vec::new(),
            })
            .proof(SolveProof::optimality())
            .fingerprints(SolveFingerprints {
                model: Some(model.clone()),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("report with model fingerprint should build");

        require_optimal_lp_certificate_for_model(&report, &model)
            .expect("matching model fingerprint should be accepted");
        let other_model = AuditFingerprint {
            value: "model-b".to_owned(),
            ..model
        };
        assert!(require_optimal_lp_certificate_for_model(&report, &other_model).is_err());
    }

    #[test]
    fn linear_model_certificate_rechecks_primal_dual_and_objective() {
        let model = certificate_model();
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(SolveSolution {
                value: None,
                values: vec![2.0],
                stable_values: BTreeMap::new(),
                objective: Some(2.0),
                objective_value: Some(2.0),
                dual_solution: Some(vec![1.0]),
                quadratic_dual_solution: None,
                pool: Vec::new(),
            })
            .proof(SolveProof::optimality())
            .fingerprints(SolveFingerprints {
                model: Some(linear_model_fingerprint(&model).expect("model fingerprint")),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("valid linear certificate report");

        require_optimal_lp_certificate_for_linear_model(&report, &model)
            .expect("model-bound certificate should pass independent checks");

        let mut tampered = report.clone();
        tampered.solution.as_mut().expect("solution").dual_solution = Some(vec![0.0]);
        assert!(require_optimal_lp_certificate_for_linear_model(&tampered, &model).is_err());

        let mut infeasible_solution = report;
        infeasible_solution
            .solution
            .as_mut()
            .expect("solution")
            .values = vec![3.0];
        assert!(
            require_optimal_lp_certificate_for_linear_model(&infeasible_solution, &model).is_err()
        );
    }

    #[test]
    fn report_reverse_projection_keeps_terminal_semantics() {
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::NodeLimit)
            .solution(SolveSolution {
                value: None,
                values: vec![1.0],
                stable_values: BTreeMap::new(),
                objective: Some(4.0),
                objective_value: Some(4.0),
                dual_solution: None,
                quadratic_dual_solution: None,
                pool: Vec::new(),
            })
            .statistics(SolveStatistics {
                best_bound_value: Some(2.0),
                relative_gap: Some(0.5),
                ..SolveStatistics::default()
            })
            .build()
            .expect("limit report should build");

        let output = solve_report_to_solver_output(&report);
        assert_eq!(output.status, SolverStatus::NodeLimit);
        assert_eq!(output.solution, Some(vec![1.0]));
        assert_eq!(output.best_bound, Some(2.0));
        assert_eq!(output.mip_gap, Some(0.5));
    }

    #[test]
    fn combinatorial_report_validates_attempt_identity() {
        let report =
            SolveReport::<f64>::builder(ProblemStatus::Unknown, TerminationReason::BackendFailure)
                .build()
                .expect("backend failure report should build");
        let aggregate = CombinatorialSolveReport::single(report, "attempt-1");
        aggregate
            .validate()
            .expect("single attempt report should validate");
        assert_eq!(aggregate.selected_attempt_id.as_deref(), Some("attempt-1"));
    }

    #[test]
    fn combinatorial_attempt_summary_falls_back_to_typed_objective() {
        let mut solution = SolveSolution::vector(vec![1.0]);
        solution.objective = Some(7.0);
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(solution)
            .build()
            .expect("typed objective-only report should be valid");
        let aggregate = CombinatorialSolveReport::single(report, "objective-only");

        aggregate
            .validate()
            .expect("objective fallback should keep the aggregate valid");
        assert_eq!(
            aggregate.attempts[0]
                .summary
                .as_ref()
                .and_then(|summary| summary.objective_value),
            Some(7.0)
        );
    }

    #[test]
    fn combinatorial_report_rejects_tampered_selected_summary() {
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::NodeLimit)
            .solution(SolveSolution {
                value: None,
                values: vec![1.0],
                stable_values: BTreeMap::new(),
                objective: Some(4.0),
                objective_value: Some(4.0),
                dual_solution: None,
                quadratic_dual_solution: None,
                pool: Vec::new(),
            })
            .statistics(SolveStatistics {
                best_bound_value: Some(2.0),
                relative_gap: Some(0.5),
                absolute_gap: Some(2.0),
                ..SolveStatistics::default()
            })
            .build()
            .expect("valid limited report should build");

        let mut objective_tampered = CombinatorialSolveReport::single(report.clone(), "attempt-1");
        objective_tampered
            .attempts
            .first_mut()
            .expect("selected attempt")
            .summary
            .as_mut()
            .expect("selected summary")
            .objective_value = Some(5.0);
        assert!(objective_tampered.validate().is_err());

        let mut bound_tampered = CombinatorialSolveReport::single(report.clone(), "attempt-1");
        bound_tampered
            .attempts
            .first_mut()
            .expect("selected attempt")
            .summary
            .as_mut()
            .expect("selected summary")
            .best_bound_value = Some(1.0);
        assert!(bound_tampered.validate().is_err());

        let mut gap_tampered = CombinatorialSolveReport::single(report, "attempt-1");
        gap_tampered
            .attempts
            .first_mut()
            .expect("selected attempt")
            .summary
            .as_mut()
            .expect("selected summary")
            .relative_gap = Some(0.25);
        assert!(gap_tampered.validate().is_err());
    }

    #[test]
    fn combinatorial_cancelled_attempt_preserves_origin() {
        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::FrameworkLoser));
        let report = cancelled_solve_report(SolverProvenance::default(), &handle)
            .expect("cancelled report should be created");
        let aggregate = CombinatorialSolveReport::single(report, "cancelled-attempt");

        aggregate
            .validate()
            .expect("cancelled single attempt should validate");
        assert_eq!(
            aggregate.attempts[0].outcome,
            SolveAttemptOutcome::Cancelled
        );
        assert_eq!(
            aggregate.attempts[0].cancellation_reason.as_deref(),
            Some("FRAMEWORK_LOSER")
        );
    }

    #[test]
    fn report_rejects_empty_or_partial_verified_proofs() {
        let empty_status =
            SolveReport::<f64>::builder(ProblemStatus::Feasible, TerminationReason::Completed)
                .solution(SolveSolution::vector(vec![1.0]))
                .proof(SolveProof {
                    kind: ProofKind::Optimality,
                    status: ProofStatus::None,
                    reliability: ProofReliability::Unknown,
                    completeness: ProofCompleteness::Unavailable,
                    reference: None,
                    evidence: None,
                })
                .build();
        assert!(empty_status.is_err());

        let partial_verified =
            SolveReport::<f64>::builder(ProblemStatus::Feasible, TerminationReason::Completed)
                .solution(SolveSolution::vector(vec![1.0]))
                .proof(SolveProof {
                    kind: ProofKind::Optimality,
                    status: ProofStatus::Verified,
                    reliability: ProofReliability::Reliable,
                    completeness: ProofCompleteness::Partial,
                    reference: None,
                    evidence: None,
                })
                .build();
        assert!(partial_verified.is_err());
    }

    #[test]
    fn report_rejects_inconsistent_bound_and_gap() {
        let mut output = SolverOutput::new(SolverStatus::NodeLimit)
            .with_objective(4.0)
            .with_solution(vec![1.0]);
        output.best_bound = Some(2.0);
        output.mip_gap = Some(0.0);
        assert!(output.try_into_solve_report().is_err());
    }

    #[test]
    fn user_interrupt_is_distinguished_from_handle_cancellation() {
        let output = SolverOutput::new(SolverStatus::UserInterrupt).with_solution(vec![1.0]);
        let interrupted = solver_output_to_report(output.clone()).expect("interrupt report");
        assert_eq!(
            interrupted.termination_reason,
            TerminationReason::Interrupted
        );

        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::External));
        let cancelled = solver_output_to_report_with_cancellation(
            output.clone(),
            SolverProvenance::default(),
            &handle,
        )
        .expect("cancelled report");
        assert_eq!(cancelled.termination_reason, TerminationReason::Cancelled);
        assert_eq!(
            cancelled.diagnostics.extensions.get("cancellation.origin"),
            Some(&"EXTERNAL".to_owned())
        );

        let late_handle = SolveHandle::new();
        late_handle.mark_completed();
        assert!(late_handle.cancel(CancellationOrigin::FrameworkLoser));
        let late_interrupt = solver_output_to_report_with_cancellation(
            output,
            SolverProvenance::default(),
            &late_handle,
        )
        .expect("late cancellation report");
        assert_eq!(
            late_interrupt.termination_reason,
            TerminationReason::Interrupted
        );
    }
}
