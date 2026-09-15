//! 临界约束分析公共协议 / Critical-constraint analysis protocol.
//!
//! 本模块只保存 solver-neutral 的分析身份、状态、能力和不可变 CP 基线。
//! Solver rows、columns、native handles 以及 lowering 生成的元素属于 adapter 内部，
//! 不得通过这里的公共类型泄露。
//! This module contains only solver-neutral analysis identity, status, capability, and immutable
//! CP-baseline types. Solver rows, columns, native handles, and lowering-generated elements are
//! adapter details and must not leak through these public types.

pub mod activity;
pub mod activation;
pub mod conflict;
pub mod correction;
pub mod criticality;
pub mod fixed_integer;
pub mod multi_target;
pub mod perturbation;
pub mod pipeline;
pub mod target;

#[cfg(test)]
mod fixture_contract;

pub use activity::{
    ACTIVITY_REPORT_SCHEMA_VERSION, ActivityConfig, ActivityEvidence, ActivityGroupSummary,
    ActivityStatus, ActivitySummary, ConstraintActivity, ConstraintActivityAdapter,
    ConstraintActivityAnalyzer, ConstraintActivityReport, VariableBoundActivity,
};
pub use activation::{
    ACTIVATION_REPORT_SCHEMA_VERSION, ActivationState, ConflictExtractionTier,
    DiagnosticActivation, DiagnosticActivationSet,
};
pub use conflict::{
    CONFLICT_REPORT_SCHEMA_VERSION, ConflictAnalysisOptions, ConflictAnalyzer,
    ConflictExplanation, ConflictGroupSummary, ConflictMinimality, ConflictValidity,
    ConflictVerification, MinimalBlockingSet, MinimalConflictAnalyzer, ConflictCache,
};
pub use correction::{
    AlternativeImprovementPlan, CorrectionCandidate, CorrectionSet, RelaxabilityPolicy,
    RelaxationCost, alternative_improvement_plans_from_conflict,
    correction_sets_from_conflict, minimal_correction_set, relaxation_recommendations_from_conflict,
    weighted_correction_set, weighted_correction_sets_from_conflict,
};
pub use criticality::{
    analyze_criticality_targets, build_criticality_profile, CriticalityKind,
    CriticalityObservation, CriticalityProfile,
};
pub use fixed_integer::{
    FIXED_INTEGER_LP_REPORT_SCHEMA_VERSION, FixedIntegerIncumbentScope, FixedIntegerLpModel,
    FixedIntegerLpScope, FixedIntegerLpSensitivityAnalyzer, FixedIntegerLpSensitivityCache,
    FixedIntegerLpSensitivityConfig, LocalConstraintSensitivity, LocalConstraintSensitivityReport,
    SensitivityScope, linear_constraint_ids,
};
pub use multi_target::{
    MULTI_TARGET_ANALYSIS_REPORT_SCHEMA_VERSION, MultiTargetAnalysisOptions,
    MultiTargetAnalysisReport, MultiTargetAnalysisResult, MultiTargetAnalyzer, analyze_targets,
    relaxation_plans_for_result,
};
pub use target::{
    OBJECTIVE_TARGET_CONSTRAINT_PREFIX, TARGET_FEASIBILITY_REPORT_SCHEMA_VERSION,
    TargetFeasibilityAnalyzer, TargetFeasibilityCache, TargetFeasibilityReport,
    exact_integer_bound, target_constraint_id,
};
pub use perturbation::{
    ADAPTIVE_PERTURBATION_REPORT_SCHEMA_VERSION, AdaptivePerturbationConfig,
    AdaptivePerturbationOutcome, ConstraintPerturbationAnalyzer, ConstraintPerturbationCache,
    ConstraintPerturbationObservation, ConstraintPerturbationPolicy, ConstraintPerturbationReport,
    PERTURBATION_REPORT_SCHEMA_VERSION, PerturbationObservation, PerturbationPolicy,
    RemovalTestObservation,
};
pub use pipeline::{
    CRITICAL_ANALYSIS_REPORT_SCHEMA_VERSION, AnalysisBaselineSummary, AnalysisGroupSummary,
    CandidateFunnelConfig, CandidateFunnelRanking, CandidateTier, ConstraintCandidate,
    ConstraintPerturbationAdapter, ConstraintPerturbationAnalysisAdapter,
    ConstraintPerturbationAnalysisRequest, CriticalConstraintAnalysisOptions,
    CriticalConstraintAnalysisPipeline, CriticalConstraintAnalysisReport,
    CriticalConstraintAnalysisReportBuilder, EffectivenessEntry, EffectivenessRanking,
    FixedIntegerLpAdapter, FixedIntegerLpAnalysisAdapter, FixedIntegerLpAnalysisRequest,
    ConstraintProgrammingFixedIntegerLpAdapter, build_candidate_funnel,
    build_effectiveness_ranking, classify_candidate, combine_status, summarize_groups,
};

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

use crate::error::{CoreError, Result, SolverError};
use crate::model::constraint_programming::ConstraintProgrammingSnapshot;
pub use crate::solver::CapabilitySupport;
use crate::solver::SolverCapabilities;
use crate::solver::SolverDescriptor;
pub use crate::solver::constraint_programming::ConstraintProgrammingSupport;
use crate::solver::constraint_programming::{
    ConstraintProgrammingSolver, ConstraintProgrammingSupportReport,
};
use crate::solver::report::{ProblemStatus, SolveReport};

/// 分析能力支持级别别名 / Analysis capability-support alias.
pub type AnalysisCapabilitySupport = CapabilitySupport;

/// 报告级稳定约束身份别名 / Report-level stable constraint identity alias.
pub type ConstraintId = crate::solver::StableConstraintId;

/// 报告级稳定变量身份别名 / Report-level stable variable identity alias.
pub type VariableId = crate::solver::StableVariableId;

/// 求解器无关的分析结论 / Solver-neutral analysis conclusion.
///
/// `Unknown` 表示求解未形成可用的数学结论；`Unsupported` 表示当前能力边界不包含该分析。
/// 两者不能互相降级，尤其不能把 `Unknown` 当作目标不可达。
/// `Unknown` means that no usable mathematical conclusion was formed; `Unsupported` means that
/// the declared capability boundary does not include the requested analysis. They must not be
/// collapsed, especially by treating `Unknown` as an unreachable target.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AnalysisStatus {
    /// 目标可达或已证明可行 / Target is reachable or feasibility was proved.
    Reachable,
    /// 目标不可达或已证明不可行 / Target is unreachable or infeasibility was proved.
    Unreachable,
    /// 尚未形成数学结论 / No mathematical conclusion is available yet.
    #[default]
    Unknown,
    /// 当前分析能力不支持 / The requested analysis is unsupported.
    Unsupported,
}

impl AnalysisStatus {
    /// 将统一求解状态映射到分析状态 / Map a unified solve status to an analysis status.
    ///
    /// 这个映射只回答"满足性问题"：找到可行解即证明可达。带预算的求解器可能返回
    /// `Feasible` 而并未证明最优，因此**当结论依赖最优性时**（目标值、扰动改善、
    /// 有效性排序），必须改用 [`Self::from_problem_status_with_proof`]。
    /// This mapping only answers satisfaction questions: a feasible point proves reachability.
    /// A budget-limited backend can report `Feasible` without proving optimality, so any
    /// conclusion that depends on optimality (objective values, perturbation improvement,
    /// effectiveness ranking) must use [`Self::from_problem_status_with_proof`] instead.
    pub const fn from_problem_status(status: ProblemStatus) -> Self {
        match status {
            ProblemStatus::Feasible => Self::Reachable,
            ProblemStatus::Infeasible => Self::Unreachable,
            ProblemStatus::Unbounded
            | ProblemStatus::InfeasibleOrUnbounded
            | ProblemStatus::Unknown => Self::Unknown,
        }
    }

    /// 带证明门控的映射 / Proof-gated mapping.
    ///
    /// 只有在 `proven` 为真时才把 `Feasible`/`Infeasible` 升格为已证明结论；
    /// 否则一律降为 [`Self::Unknown`]，防止"超时但拿到 incumbent"被读成已证明。
    /// Only a proven run may upgrade `Feasible`/`Infeasible` into a proven conclusion.
    /// Otherwise the result degrades to [`Self::Unknown`], so "timed out with an incumbent"
    /// can never be read as proven.
    pub const fn from_problem_status_with_proof(status: ProblemStatus, proven: bool) -> Self {
        match status {
            ProblemStatus::Feasible if proven => Self::Reachable,
            ProblemStatus::Infeasible if proven => Self::Unreachable,
            ProblemStatus::Feasible | ProblemStatus::Infeasible => Self::Unknown,
            ProblemStatus::Unbounded
            | ProblemStatus::InfeasibleOrUnbounded
            | ProblemStatus::Unknown => Self::Unknown,
        }
    }

    /// 是否形成已证明的可达性结论 / Whether this is a proven reachability conclusion.
    pub const fn is_proven(self) -> bool {
        matches!(self, Self::Reachable | Self::Unreachable)
    }

    /// 是否表示目标可达 / Whether the target is reachable.
    pub const fn is_reachable(self) -> bool {
        matches!(self, Self::Reachable)
    }

    /// 是否表示目标不可达 / Whether the target is unreachable.
    pub const fn is_unreachable(self) -> bool {
        matches!(self, Self::Unreachable)
    }

    /// SAT 语义别名 / SAT semantic alias.
    #[allow(non_upper_case_globals)]
    pub const Sat: Self = Self::Reachable;

    /// UNSAT 语义别名 / UNSAT semantic alias.
    #[allow(non_upper_case_globals)]
    pub const Unsat: Self = Self::Unreachable;

    /// SAT 语义别名 / SAT semantic alias.
    #[allow(non_upper_case_globals)]
    pub const SAT: Self = Self::Reachable;

    /// UNSAT 语义别名 / UNSAT semantic alias.
    #[allow(non_upper_case_globals)]
    pub const UNSAT: Self = Self::Unreachable;
}

impl From<ProblemStatus> for AnalysisStatus {
    fn from(status: ProblemStatus) -> Self {
        Self::from_problem_status(status)
    }
}

/// 稳定目标身份 / Stable objective identity.
///
/// Rust 核心的线性模型映射目前以字符串保存目标身份；使用这个透明 newtype 可以在
/// 分析 API 中表达同一身份，同时不暴露 solver 的目标索引。
/// The Rust core currently stores linear-model objective identities as strings. This transparent
/// newtype expresses that same identity in the analysis API without exposing solver indices.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ObjectiveId(pub String);

impl ObjectiveId {
    /// 创建目标身份 / Create an objective identity.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 返回字符串形式 / Return the string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ObjectiveId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<String> for ObjectiveId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ObjectiveId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// 目标边界方向 / Objective-bound direction.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectiveTargetRelation {
    /// 目标值至少达到给定值 / Objective value must be at least the bound.
    AtLeast,
    /// 目标值至多达到给定值 / Objective value must be at most the bound.
    AtMost,
}

impl ObjectiveTargetRelation {
    fn stable_name(self) -> &'static str {
        match self {
            Self::AtLeast => "at-least",
            Self::AtMost => "at-most",
        }
    }
}

/// 目标可行性分析条件 / Objective target for feasibility analysis.
///
/// 目标只包含稳定目标身份和有限数值。临时 target 约束的构建属于后续 analyzer，不能
/// 修改原始 snapshot。
/// A target contains only a stable objective identity and a finite value. Building a temporary
/// target constraint belongs to a later analyzer and must not mutate the original snapshot.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectiveTarget {
    /// 目标至少达到给定值 / Objective must be at least the value.
    AtLeast {
        /// 稳定目标身份 / Stable objective identity.
        objective_id: ObjectiveId,
        /// 目标边界值 / Objective bound.
        value: f64,
    },
    /// 目标至多达到给定值 / Objective must be at most the value.
    AtMost {
        /// 稳定目标身份 / Stable objective identity.
        objective_id: ObjectiveId,
        /// 目标边界值 / Objective bound.
        value: f64,
    },
}

impl ObjectiveTarget {
    /// 创建下界 target，并校验身份和值 / Create and validate a lower target.
    pub fn at_least(objective_id: impl Into<ObjectiveId>, value: f64) -> Result<Self> {
        let target = Self::AtLeast {
            objective_id: objective_id.into(),
            value,
        };
        target.validate()?;
        Ok(target)
    }

    /// 创建上界 target，并校验身份和值 / Create and validate an upper target.
    pub fn at_most(objective_id: impl Into<ObjectiveId>, value: f64) -> Result<Self> {
        let target = Self::AtMost {
            objective_id: objective_id.into(),
            value,
        };
        target.validate()?;
        Ok(target)
    }

    /// Build an absolute lower target from a baseline and a fractional relative change.
    pub fn at_least_relative(
        objective_id: impl Into<ObjectiveId>,
        baseline: f64,
        fraction: f64,
    ) -> Result<Self> {
        Self::at_least(objective_id, baseline * (1.0 + fraction))
    }

    /// Build an absolute upper target from a baseline and a fractional relative change.
    pub fn at_most_relative(
        objective_id: impl Into<ObjectiveId>,
        baseline: f64,
        fraction: f64,
    ) -> Result<Self> {
        Self::at_most(objective_id, baseline * (1.0 + fraction))
    }

    /// Build a target from a caller-provided business-unit delta.
    pub fn at_least_business_unit(
        objective_id: impl Into<ObjectiveId>,
        baseline: f64,
        delta: f64,
    ) -> Result<Self> {
        Self::at_least(objective_id, baseline + delta)
    }

    /// Build an upper target from a caller-provided business-unit delta.
    pub fn at_most_business_unit(
        objective_id: impl Into<ObjectiveId>,
        baseline: f64,
        delta: f64,
    ) -> Result<Self> {
        Self::at_most(objective_id, baseline + delta)
    }

    /// 校验目标身份和值 / Validate the target identity and value.
    pub fn validate(&self) -> Result<()> {
        if self.objective_id().as_str().trim().is_empty() {
            return Err(invalid_analysis("objective ID must not be blank"));
        }
        if !self.value().is_finite() {
            return Err(invalid_analysis("objective target must be finite"));
        }
        Ok(())
    }

    /// 返回目标身份 / Return the objective identity.
    pub fn objective_id(&self) -> &ObjectiveId {
        match self {
            Self::AtLeast { objective_id, .. } | Self::AtMost { objective_id, .. } => objective_id,
        }
    }

    /// 返回边界值 / Return the bound value.
    pub fn value(&self) -> f64 {
        match self {
            Self::AtLeast { value, .. } | Self::AtMost { value, .. } => *value,
        }
    }

    /// 返回方向 / Return the bound direction.
    pub const fn relation(&self) -> ObjectiveTargetRelation {
        match self {
            Self::AtLeast { .. } => ObjectiveTargetRelation::AtLeast,
            Self::AtMost { .. } => ObjectiveTargetRelation::AtMost,
        }
    }

    /// 返回稳定 target 身份 / Return the stable target identity.
    pub fn stable_id(&self) -> String {
        format!(
            "objective-target:{}:{}:{}",
            self.objective_id(),
            self.relation().stable_name(),
            canonical_float(self.value())
        )
    }

    /// 判断目标值是否满足条件 / Check whether an objective value satisfies the target.
    pub fn is_satisfied(&self, objective_value: f64) -> bool {
        if !objective_value.is_finite() || !self.value().is_finite() {
            return false;
        }
        match self.relation() {
            ObjectiveTargetRelation::AtLeast => objective_value >= self.value(),
            ObjectiveTargetRelation::AtMost => objective_value <= self.value(),
        }
    }

    /// 返回到目标边界的有符号距离 / Return signed distance to the target boundary.
    ///
    /// 正值表示满足，负值表示违反 / Positive values satisfy the target; negative values violate it.
    pub fn signed_distance(&self, objective_value: f64) -> f64 {
        match self.relation() {
            ObjectiveTargetRelation::AtLeast => objective_value - self.value(),
            ObjectiveTargetRelation::AtMost => self.value() - objective_value,
        }
    }
}

/// 变量边界方向 / Variable-bound side.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoundSide {
    /// 下界 / Lower bound.
    Lower,
    /// 上界 / Upper bound.
    Upper,
}

/// 稳定变量边界引用 / Stable variable-bound reference.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariableBoundRef {
    /// 稳定变量身份 / Stable variable identity.
    pub variable_id: crate::solver::StableVariableId,
    /// 边界方向 / Bound side.
    pub side: BoundSide,
}

/// 稳定变量值域引用 / Stable variable-domain reference.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariableDomainRef {
    /// 稳定变量身份 / Stable variable identity.
    pub variable_id: crate::solver::StableVariableId,
}

/// 公共诊断证据来源 / Public diagnostic evidence source.
///
/// 这些变体只引用原始模型稳定身份。这里没有 solver row、column、auxiliary variable 或
/// backend handle；lowering provenance 必须在映射边界完成后再构造这些值。
/// These variants reference only original-model stable identities. There are no solver rows,
/// columns, auxiliary variables, or backend handles here; lowering provenance must be resolved
/// before constructing these values.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticSource {
    /// 原始约束 / Original constraint.
    Constraint {
        /// 稳定约束身份 / Stable constraint identity.
        id: crate::solver::StableConstraintId,
    },
    /// 原始变量下界 / Original variable lower bound.
    VariableLowerBound {
        /// 稳定变量身份 / Stable variable identity.
        variable_id: crate::solver::StableVariableId,
    },
    /// 原始变量上界 / Original variable upper bound.
    VariableUpperBound {
        /// 稳定变量身份 / Stable variable identity.
        variable_id: crate::solver::StableVariableId,
    },
    /// 可区分方向的变量边界 / Variable bound with an explicit side.
    VariableBound {
        /// 稳定边界引用 / Stable bound reference.
        bound: VariableBoundRef,
    },
    /// 稀疏整数值域 / Sparse integer domain.
    SparseDomain {
        /// 稳定变量身份 / Stable variable identity.
        variable_id: crate::solver::StableVariableId,
    },
    /// 目标突破条件 / Objective target condition.
    ObjectiveTarget {
        /// 目标条件 / Objective target.
        target: ObjectiveTarget,
    },
}

impl DiagnosticSource {
    /// 创建原始约束来源 / Create an original-constraint source.
    pub fn constraint(id: impl Into<crate::solver::StableConstraintId>) -> Result<Self> {
        let source = Self::Constraint { id: id.into() };
        source.validate()?;
        Ok(source)
    }

    /// 创建变量下界来源 / Create a variable-lower-bound source.
    pub fn variable_lower_bound(
        variable_id: impl Into<crate::solver::StableVariableId>,
    ) -> Result<Self> {
        let source = Self::VariableLowerBound {
            variable_id: variable_id.into(),
        };
        source.validate()?;
        Ok(source)
    }

    /// 创建变量上界来源 / Create a variable-upper-bound source.
    pub fn variable_upper_bound(
        variable_id: impl Into<crate::solver::StableVariableId>,
    ) -> Result<Self> {
        let source = Self::VariableUpperBound {
            variable_id: variable_id.into(),
        };
        source.validate()?;
        Ok(source)
    }

    /// 创建变量边界来源 / Create a variable-bound source.
    pub fn variable_bound(bound: VariableBoundRef) -> Result<Self> {
        let source = Self::VariableBound { bound };
        source.validate()?;
        Ok(source)
    }

    /// 创建稀疏域来源 / Create a sparse-domain source.
    pub fn sparse_domain(variable_id: impl Into<crate::solver::StableVariableId>) -> Result<Self> {
        let source = Self::SparseDomain {
            variable_id: variable_id.into(),
        };
        source.validate()?;
        Ok(source)
    }

    /// 创建目标来源 / Create an objective-target source.
    pub fn objective_target(target: ObjectiveTarget) -> Result<Self> {
        target.validate()?;
        Ok(Self::ObjectiveTarget { target })
    }

    /// 校验稳定来源身份 / Validate source identities.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Constraint { id } if id.0.trim().is_empty() => {
                Err(invalid_analysis("constraint ID must not be blank"))
            }
            Self::VariableLowerBound { variable_id }
            | Self::VariableUpperBound { variable_id }
            | Self::SparseDomain { variable_id }
                if variable_id.0.trim().is_empty() =>
            {
                Err(invalid_analysis("variable ID must not be blank"))
            }
            Self::VariableBound { bound } if bound.variable_id.0.trim().is_empty() => {
                Err(invalid_analysis("variable ID must not be blank"))
            }
            Self::ObjectiveTarget { target } => target.validate(),
            _ => Ok(()),
        }
    }

    /// 返回稳定来源类型 / Return the stable source kind.
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Constraint { .. } => "constraint",
            Self::VariableLowerBound { .. } => "variable-lower-bound",
            Self::VariableUpperBound { .. } => "variable-upper-bound",
            Self::VariableBound { .. } => "variable-bound",
            Self::SparseDomain { .. } => "sparse-domain",
            Self::ObjectiveTarget { .. } => "objective-target",
        }
    }

    /// 返回稳定来源身份 / Return the stable source identity.
    pub fn stable_id(&self) -> String {
        match self {
            Self::Constraint { id } => format!("constraint:{}", id.0),
            Self::VariableLowerBound { variable_id } => {
                format!("variable:{}:lower", variable_id.0)
            }
            Self::VariableUpperBound { variable_id } => {
                format!("variable:{}:upper", variable_id.0)
            }
            Self::VariableBound { bound } => format!(
                "variable:{}:{}",
                bound.variable_id.0,
                match bound.side {
                    BoundSide::Lower => "lower",
                    BoundSide::Upper => "upper",
                }
            ),
            Self::SparseDomain { variable_id } => format!("variable:{}:domain", variable_id.0),
            Self::ObjectiveTarget { target } => target.stable_id(),
        }
    }

    /// 映射到现有不可行证据成员 / Project to an existing infeasibility-evidence member.
    ///
    /// 目标 target 不是原始约束成员，因而返回 `None`；它应在更高层报告中作为独立证据。
    /// An objective target is not an original constraint member and therefore returns `None`; it
    /// remains a separate evidence item in the higher-level report.
    pub fn as_infeasibility_member(&self) -> Option<crate::solver::InfeasibilityEvidenceMember> {
        match self {
            Self::Constraint { id } => Some(
                crate::solver::InfeasibilityEvidenceMember::Constraint(id.0.clone()),
            ),
            Self::VariableLowerBound { variable_id } => Some(
                crate::solver::InfeasibilityEvidenceMember::LowerBound(variable_id.0.clone()),
            ),
            Self::VariableUpperBound { variable_id } => Some(
                crate::solver::InfeasibilityEvidenceMember::UpperBound(variable_id.0.clone()),
            ),
            Self::VariableBound { bound } => Some(match bound.side {
                BoundSide::Lower => crate::solver::InfeasibilityEvidenceMember::LowerBound(
                    bound.variable_id.0.clone(),
                ),
                BoundSide::Upper => crate::solver::InfeasibilityEvidenceMember::UpperBound(
                    bound.variable_id.0.clone(),
                ),
            }),
            Self::SparseDomain { variable_id } => Some(
                crate::solver::InfeasibilityEvidenceMember::Domain(variable_id.0.clone()),
            ),
            Self::ObjectiveTarget { .. } => None,
        }
    }
}

impl Display for DiagnosticSource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.stable_id())
    }
}

/// 分析操作能力 / Analysis operation capability.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnalysisCapability {
    /// 活动性分析 / Activity analysis.
    Activity,
    /// 固定整数后的 LP 灵敏度 / Fixed-integer LP sensitivity.
    FixedIntegerLpSensitivity,
    /// RHS 扰动 / RHS perturbation.
    RhsPerturbation,
    /// 自适应扰动 / Adaptive perturbation.
    AdaptivePerturbation,
    /// 删除重优化 / Removal test.
    RemovalTest,
    /// 目标 target 构造 / Objective target.
    ObjectiveTarget,
    /// 假设求解 / Assumption solving.
    AssumptionSolving,
    /// 原生不可满足核心 / Native unsat core.
    NativeUnsatCore,
    /// 冲突提取 / Conflict extraction.
    Conflict,
    /// 最小不可行冲突 / Minimal unsatisfiable conflict.
    Mus,
    /// 约束组汇总 / Constraint-group aggregation.
    ConstraintGroupAggregation,
    /// warm start / Warm start.
    WarmStart,
    /// 精确 CP lowering / Exact CP lowering.
    ExactCpLowering,
}

impl AnalysisCapability {
    /// 返回稳定能力名称 / Return the stable capability name.
    pub const fn stable_name(self) -> &'static str {
        match self {
            Self::Activity => "activity",
            Self::FixedIntegerLpSensitivity => "fixed_integer_lp_sensitivity",
            Self::RhsPerturbation => "rhs_perturbation",
            Self::AdaptivePerturbation => "adaptive_perturbation",
            Self::RemovalTest => "removal_test",
            Self::ObjectiveTarget => "objective_target",
            Self::AssumptionSolving => "assumption_solving",
            Self::NativeUnsatCore => "native_unsat_core",
            Self::Conflict => "conflict",
            Self::Mus => "mus",
            Self::ConstraintGroupAggregation => "constraint_group_aggregation",
            Self::WarmStart => "warm_start",
            Self::ExactCpLowering => "exact_cp_lowering",
        }
    }
}

/// CP 能力 / CP feature.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConstraintProgrammingFeature {
    /// 布尔逻辑 / Boolean logic.
    BooleanLogic,
    /// 重ification / Reification.
    Reification,
    /// 稀疏值域 / Sparse domain.
    SparseDomain,
    /// AllDifferent / AllDifferent.
    AllDifferent,
    /// Element / Element.
    Element,
    /// Table / Table.
    Table,
    /// 区间 / Interval.
    Interval,
    /// 可选区间 / Optional interval.
    OptionalInterval,
    /// NoOverlap / NoOverlap.
    NoOverlap,
    /// Cumulative / Cumulative.
    Cumulative,
    /// Circuit / Circuit.
    Circuit,
    /// Automaton / Automaton.
    Automaton,
    /// Reservoir / Reservoir.
    Reservoir,
    /// 假设求解 / Assumption solving.
    Assumption,
    /// 冲突核心 / Conflict core.
    ConflictCore,
    /// 解提示 / Solution hint.
    SolutionHint,
    /// 增量求解 / Incremental solving.
    IncrementalSolve,
}

impl ConstraintProgrammingFeature {
    /// 返回稳定 CP feature 名称 / Return the stable CP feature name.
    pub const fn stable_name(self) -> &'static str {
        match self {
            Self::BooleanLogic => "boolean_logic",
            Self::Reification => "reification",
            Self::SparseDomain => "sparse_domain",
            Self::AllDifferent => "all_different",
            Self::Element => "element",
            Self::Table => "table",
            Self::Interval => "interval",
            Self::OptionalInterval => "optional_interval",
            Self::NoOverlap => "no_overlap",
            Self::Cumulative => "cumulative",
            Self::Circuit => "circuit",
            Self::Automaton => "automaton",
            Self::Reservoir => "reservoir",
            Self::Assumption => "assumption",
            Self::ConflictCore => "conflict_core",
            Self::SolutionHint => "solution_hint",
            Self::IncrementalSolve => "incremental_solve",
        }
    }
}

/// 可分析的模型种类 / Model kinds understood by analysis dispatch.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SolverModelType {
    /// 线性连续模型 / Linear model.
    Linear,
    /// 混合整数线性模型 / Mixed-integer linear model.
    Mip,
    /// 二次模型 / Quadratic model.
    Quadratic,
    /// 混合整数二次模型 / Mixed-integer quadratic model.
    Miqp,
    /// 约束规划模型 / Constraint-programming model.
    ConstraintProgramming,
}

/// 分析能力矩阵 / Solver-neutral analysis capability matrix.
///
/// 原始 solver descriptor 只提供基础能力；这里保留分析阶段能力和 CP feature 的显式
/// 覆盖点，使 adapter 可以声明“条件支持”而不把未实现能力误报为可用。
/// The solver descriptor provides primitive capabilities; this matrix adds explicit analysis
/// stages and CP-feature overrides so adapters can declare conditional support without claiming
/// unsupported behavior.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityMatrix {
    /// 可处理模型种类 / Supported model kinds.
    pub model_types: BTreeSet<SolverModelType>,
    /// 是否原生支持假设求解 / Whether native assumption solving is available.
    pub native_assumption_solving: bool,
    /// 是否原生支持不可满足核心 / Whether native unsat-core extraction is available.
    pub native_unsat_core: bool,
    /// 是否允许 satisfaction-only fallback / Whether satisfaction-only fallback is allowed.
    pub fallback_satisfaction_only: bool,
    /// 是否能访问 LP dual / Whether LP duals are available.
    pub dual: bool,
    /// 是否支持 warm start / Whether warm starts are available.
    pub warm_start: bool,
    /// 是否存在精确 CP lowering / Whether exact CP lowering is available.
    pub exact_cp_lowering: bool,
    /// CP feature 支持级别 / CP feature support levels.
    pub constraint_programming_features:
        BTreeMap<ConstraintProgrammingFeature, ConstraintProgrammingSupport>,
    /// 分析能力显式覆盖 / Explicit analysis-capability overrides.
    pub analysis_capabilities: BTreeMap<AnalysisCapability, CapabilitySupport>,
}

impl Default for CapabilityMatrix {
    fn default() -> Self {
        Self {
            model_types: BTreeSet::new(),
            native_assumption_solving: false,
            native_unsat_core: false,
            // An empty matrix describes no backend at all, so it must not claim a
            // satisfaction-only fallback: that would report unsupported stages as usable.
            // 空矩阵没有描述任何后端，因此不得声称具备 satisfaction-only 回退，
            // 否则会把不受支持的分析阶段报告为可用。
            fallback_satisfaction_only: false,
            dual: false,
            warm_start: false,
            exact_cp_lowering: false,
            constraint_programming_features: BTreeMap::new(),
            analysis_capabilities: BTreeMap::new(),
        }
    }
}

/// 能力查询对象 / Capability query object.
pub trait CapabilityQuery {
    /// 查询该对象在矩阵中的支持情况 / Query support for this object in a matrix.
    fn support_in(matrix: &CapabilityMatrix, value: Self) -> bool;
}

impl CapabilityQuery for AnalysisCapability {
    fn support_in(matrix: &CapabilityMatrix, value: Self) -> bool {
        matrix.support(value) != CapabilitySupport::Unsupported
    }
}

impl CapabilityQuery for ConstraintProgrammingFeature {
    fn support_in(matrix: &CapabilityMatrix, value: Self) -> bool {
        matrix.cp_support(value) != ConstraintProgrammingSupport::Unsupported
    }
}

impl CapabilityMatrix {
    /// LP dual 支持别名 / LP-dual support alias.
    pub const fn lp_dual(&self) -> bool {
        self.dual
    }

    /// 查询分析能力 / Query an analysis capability.
    pub fn support(&self, capability: AnalysisCapability) -> CapabilitySupport {
        self.analysis_capabilities
            .get(&capability)
            .copied()
            .unwrap_or_else(|| self.inferred_support(capability))
    }

    /// 查询分析或 CP feature 是否可用 / Check whether an analysis or CP feature is available.
    pub fn supports<C: CapabilityQuery>(&self, capability: C) -> bool {
        C::support_in(self, capability)
    }

    /// 查询是否为原生能力 / Check whether an analysis capability is native.
    pub fn is_native(&self, capability: AnalysisCapability) -> bool {
        match capability {
            AnalysisCapability::AssumptionSolving => self.native_assumption_solving,
            AnalysisCapability::NativeUnsatCore => self.native_unsat_core,
            AnalysisCapability::WarmStart => self.warm_start,
            AnalysisCapability::ExactCpLowering => self.exact_cp_lowering,
            _ => false,
        }
    }

    /// 查询 CP feature 支持级别 / Query CP feature support level.
    pub fn cp_support(
        &self,
        feature: ConstraintProgrammingFeature,
    ) -> ConstraintProgrammingSupport {
        self.constraint_programming_features
            .get(&feature)
            .copied()
            .unwrap_or(ConstraintProgrammingSupport::Unsupported)
    }

    /// 查询 CP feature 是否可用 / Check whether a CP feature is available.
    pub fn supports_cp(&self, feature: ConstraintProgrammingFeature) -> bool {
        self.cp_support(feature) != ConstraintProgrammingSupport::Unsupported
    }

    /// 设置分析能力覆盖 / Set an analysis-capability override.
    pub fn with_capability(
        mut self,
        capability: AnalysisCapability,
        support: CapabilitySupport,
    ) -> Self {
        self.analysis_capabilities.insert(capability, support);
        self
    }

    /// 设置 CP feature 支持级别 / Set a CP feature support level.
    pub fn with_cp_feature(
        mut self,
        feature: ConstraintProgrammingFeature,
        support: ConstraintProgrammingSupport,
    ) -> Self {
        self.constraint_programming_features
            .insert(feature, support);
        self
    }

    /// 设置模型种类 / Add a supported model kind.
    pub fn with_model_type(mut self, model_type: SolverModelType) -> Self {
        self.model_types.insert(model_type);
        self
    }

    /// 从 solver descriptor 构建能力矩阵 / Build a matrix from a solver descriptor.
    pub fn from_descriptor(descriptor: &SolverDescriptor) -> Self {
        Self::from(descriptor)
    }

    /// 从现有 CP 支持报告合并能力 / Merge capabilities from a CP support report.
    pub fn from_constraint_programming_support(
        descriptor: &SolverDescriptor,
        support: &ConstraintProgrammingSupportReport,
    ) -> Self {
        let mut matrix = Self::from(descriptor);
        matrix
            .model_types
            .insert(SolverModelType::ConstraintProgramming);
        matrix.constraint_programming_features.insert(
            ConstraintProgrammingFeature::BooleanLogic,
            if support.satisfaction {
                ConstraintProgrammingSupport::Conditional
            } else {
                ConstraintProgrammingSupport::Unsupported
            },
        );
        matrix.constraint_programming_features.insert(
            ConstraintProgrammingFeature::SparseDomain,
            if support.sparse_domain {
                ConstraintProgrammingSupport::Conditional
            } else {
                ConstraintProgrammingSupport::Unsupported
            },
        );
        matrix.constraint_programming_features.insert(
            ConstraintProgrammingFeature::Assumption,
            if support.assumptions {
                ConstraintProgrammingSupport::Conditional
            } else {
                ConstraintProgrammingSupport::Unsupported
            },
        );
        matrix.constraint_programming_features.insert(
            ConstraintProgrammingFeature::SolutionHint,
            if support.solution_hint {
                ConstraintProgrammingSupport::Conditional
            } else {
                ConstraintProgrammingSupport::Unsupported
            },
        );
        matrix.constraint_programming_features.insert(
            ConstraintProgrammingFeature::ConflictCore,
            if support.verified_conflict_seed {
                ConstraintProgrammingSupport::Conditional
            } else {
                ConstraintProgrammingSupport::Unsupported
            },
        );
        // The CP support report describes what the CP facade can provide, including
        // snapshot-rebuild fallbacks. It does not, by itself, prove that the wrapped backend has
        // a native assumption or native-core API. Native analysis capabilities must therefore be
        // explicitly declared by the backend descriptor; otherwise the router must use the
        // rebuild/fallback tier even when the facade can produce a verified seed.
        // CP 支持报告描述的是 CP facade 能提供的能力，其中包括 snapshot 重建回退；它本身
        // 不能证明包装的 backend 具有原生 assumption 或原生 core API。原生分析能力必须由
        // backend descriptor 显式声明，否则即使 facade 能生成已验证 seed，路由仍须使用重建/回退层。
        matrix.native_assumption_solving = descriptor_declares_native(
            descriptor,
            &["assumption", "native_assumption_solving"],
        );
        matrix.native_unsat_core = descriptor_declares_native(
            descriptor,
            &["unsat_core", "native_unsat_core", "conflict_core"],
        );
        matrix.constraint_programming_features.insert(
            ConstraintProgrammingFeature::IncrementalSolve,
            if support.incremental_session {
                ConstraintProgrammingSupport::Native
            } else {
                ConstraintProgrammingSupport::Unsupported
            },
        );
        matrix.fallback_satisfaction_only = support.satisfaction;
        // Exact lowering is a whole-snapshot capability. One lowered constraint does not make a
        // mixed snapshot safe to solve, because an unsupported sibling would otherwise be
        // silently omitted by an adapter. Empty snapshots are supported vacuously when the
        // backend reports a complete satisfaction path.
        // 精确 lowering 是整个 snapshot 的能力：单条约束可 lower 不能掩盖同一模型中其它
        // 不支持的约束，否则 adapter 可能静默丢失约束。后端确认完整 satisfaction 路径时，
        // 空约束集合按空真处理。
        matrix.exact_cp_lowering = support.satisfaction
            && support.integer_objective
            && support
                .constraints
                .values()
                .all(|level| *level == ConstraintProgrammingSupport::ExactLowering);
        matrix
    }

    fn inferred_support(&self, capability: AnalysisCapability) -> CapabilitySupport {
        match capability {
            AnalysisCapability::Activity | AnalysisCapability::ConstraintGroupAggregation => {
                CapabilitySupport::Supported
            }
            AnalysisCapability::FixedIntegerLpSensitivity => {
                if self.dual
                    && (self.model_types.contains(&SolverModelType::Mip) || self.exact_cp_lowering)
                {
                    CapabilitySupport::Supported
                } else if self.dual {
                    CapabilitySupport::Conditional
                } else {
                    CapabilitySupport::Unsupported
                }
            }
            AnalysisCapability::RhsPerturbation
            | AnalysisCapability::RemovalTest
            | AnalysisCapability::ObjectiveTarget => {
                if self.model_types.is_empty() {
                    CapabilitySupport::Conditional
                } else {
                    CapabilitySupport::Supported
                }
            }
            AnalysisCapability::AdaptivePerturbation => {
                match self.support(AnalysisCapability::RhsPerturbation) {
                    CapabilitySupport::Supported => CapabilitySupport::Supported,
                    CapabilitySupport::Conditional => CapabilitySupport::Conditional,
                    CapabilitySupport::Unsupported => CapabilitySupport::Unsupported,
                }
            }
            AnalysisCapability::AssumptionSolving => {
                if self.native_assumption_solving {
                    CapabilitySupport::Supported
                } else if self.fallback_satisfaction_only {
                    CapabilitySupport::Conditional
                } else {
                    CapabilitySupport::Unsupported
                }
            }
            AnalysisCapability::NativeUnsatCore => {
                if self.native_unsat_core {
                    CapabilitySupport::Supported
                } else {
                    CapabilitySupport::Unsupported
                }
            }
            AnalysisCapability::Conflict => {
                if self.native_unsat_core {
                    CapabilitySupport::Supported
                } else if self.fallback_satisfaction_only {
                    CapabilitySupport::Conditional
                } else {
                    CapabilitySupport::Unsupported
                }
            }
            AnalysisCapability::Mus => {
                if self.support(AnalysisCapability::Conflict) == CapabilitySupport::Unsupported {
                    CapabilitySupport::Unsupported
                } else if self.model_types.is_empty() {
                    CapabilitySupport::Conditional
                } else {
                    CapabilitySupport::Supported
                }
            }
            AnalysisCapability::WarmStart => {
                if self.warm_start {
                    CapabilitySupport::Supported
                } else {
                    CapabilitySupport::Unsupported
                }
            }
            AnalysisCapability::ExactCpLowering => {
                if self.exact_cp_lowering {
                    CapabilitySupport::Supported
                } else {
                    CapabilitySupport::Unsupported
                }
            }
        }
    }
}

fn descriptor_declares_native(descriptor: &SolverDescriptor, names: &[&str]) -> bool {
    names.iter().any(|name| {
        descriptor.capabilities.support(name) == CapabilitySupport::Supported
    })
}

impl From<SolverDescriptor> for CapabilityMatrix {
    fn from(descriptor: SolverDescriptor) -> Self {
        Self::from(&descriptor)
    }
}

impl From<&SolverDescriptor> for CapabilityMatrix {
    fn from(descriptor: &SolverDescriptor) -> Self {
        Self::from(&descriptor.capabilities)
    }
}

impl From<SolverCapabilities> for CapabilityMatrix {
    fn from(capabilities: SolverCapabilities) -> Self {
        Self::from(&capabilities)
    }
}

impl From<&SolverCapabilities> for CapabilityMatrix {
    fn from(capabilities: &SolverCapabilities) -> Self {
        let mut matrix = Self::default();
        for (name, support) in &capabilities.levels {
            if *support == CapabilitySupport::Unsupported {
                continue;
            }
            match name.as_str() {
                "linear" => {
                    matrix.model_types.insert(SolverModelType::Linear);
                }
                "mip" => {
                    matrix.model_types.insert(SolverModelType::Mip);
                }
                "quadratic" => {
                    matrix.model_types.insert(SolverModelType::Quadratic);
                }
                "miqp" => {
                    matrix.model_types.insert(SolverModelType::Miqp);
                }
                "constraint_programming" => {
                    matrix
                        .model_types
                        .insert(SolverModelType::ConstraintProgramming);
                }
                "dual" | "lp_dual" => matrix.dual = true,
                "warm_start" => matrix.warm_start = true,
                "assumption" | "native_assumption_solving" => {
                    matrix.native_assumption_solving = true;
                }
                "unsat_core" | "native_unsat_core" | "conflict_core" => {
                    matrix.native_unsat_core = true;
                }
                "exact_cp_lowering" => matrix.exact_cp_lowering = true,
                _ => {}
            }
        }
        matrix
    }
}

/// 后续分析阶段的缓存桶 / Cache bucket reserved for later analysis stages.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnalysisCacheKind {
    /// 活动性 / Activity.
    Activity,
    /// 固定整数 LP / Fixed-integer LP.
    FixedIntegerLp,
    /// RHS 扰动 / Perturbation.
    Perturbation,
    /// 目标可行性 / Target feasibility.
    Target,
    /// 冲突 / Conflict.
    Conflict,
}

/// 基于不可变 CP 快照的分析会话 / Analysis session over an immutable CP snapshot.
///
/// S0 只实现生命周期、稳定基线和状态缓存；实际求解、lowering、扰动及冲突提取由后续
/// analyzer 完成。会话不保存 `SolveReport`，因此不会把其 row-oriented mapping 泄露到
/// analysis 公共类型。
/// S0 provides lifecycle, stable baseline, and status caches only. Later analyzers perform solving,
/// lowering, perturbation, and conflict extraction. The session does not retain `SolveReport`, so
/// row-oriented mappings cannot leak into the public analysis types.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct CriticalConstraintAnalysisSession {
    /// 不可变 CP 基线 / Immutable CP baseline.
    baseline_snapshot: ConstraintProgrammingSnapshot,
    /// 按稳定变量身份保存的 baseline incumbent / Baseline incumbent keyed by stable variable identity.
    baseline_solution: Option<BTreeMap<crate::solver::StableVariableId, i64>>,
    /// baseline 目标值 / Baseline objective value.
    baseline_objective_value: Option<f64>,
    /// 求解器来源描述 / Solver descriptor.
    solver_descriptor: Option<SolverDescriptor>,
    /// 分析能力矩阵 / Analysis capability matrix.
    capability_matrix: CapabilityMatrix,
    /// 各阶段的稳定状态缓存 / Per-stage cache of stable analysis statuses.
    #[cfg_attr(feature = "serde", serde(default))]
    caches: BTreeMap<AnalysisCacheKind, BTreeMap<String, AnalysisStatus>>,
    /// 是否已关闭 / Whether the session has been closed.
    #[cfg_attr(feature = "serde", serde(skip))]
    closed: bool,
}

impl CriticalConstraintAnalysisSession {
    /// 从快照创建空基线会话 / Create an empty-baseline session from a snapshot.
    pub fn new(snapshot: ConstraintProgrammingSnapshot) -> Result<Self> {
        Self::from_snapshot(snapshot)
    }

    /// 从快照创建会话 / Create a session from a snapshot.
    pub fn from_snapshot(snapshot: ConstraintProgrammingSnapshot) -> Result<Self> {
        Self::from_snapshot_with_options(snapshot, None, None, None, None)
    }

    /// 从快照、基线和能力元数据创建会话 / Create a session with baseline and capability metadata.
    pub fn from_snapshot_with_options(
        snapshot: ConstraintProgrammingSnapshot,
        baseline_solution: Option<BTreeMap<crate::solver::StableVariableId, i64>>,
        baseline_objective_value: Option<f64>,
        solver_descriptor: Option<SolverDescriptor>,
        capability_matrix: Option<CapabilityMatrix>,
    ) -> Result<Self> {
        snapshot.validate_identity()?;
        if let Some(value) = baseline_objective_value
            && !value.is_finite()
        {
            return Err(invalid_analysis("baseline objective value must be finite"));
        }
        if let Some(solution) = &baseline_solution {
            snapshot.validate_hint(solution)?;
        }
        validate_baseline_objective(
            &snapshot,
            baseline_solution.as_ref(),
            baseline_objective_value,
        )?;
        let capability_matrix = capability_matrix.unwrap_or_else(|| {
            solver_descriptor
                .as_ref()
                .map(CapabilityMatrix::from)
                .unwrap_or_default()
        });
        Ok(Self {
            baseline_snapshot: snapshot,
            baseline_solution,
            baseline_objective_value,
            solver_descriptor,
            capability_matrix,
            caches: empty_caches(),
            closed: false,
        })
    }

    /// 从 CP solver 创建会话并读取其运行时能力 / Create a session from a CP solver and inspect its capabilities.
    pub fn from_constraint_programming_solver<S>(
        snapshot: ConstraintProgrammingSnapshot,
        solver: &S,
    ) -> Result<Self>
    where
        S: ConstraintProgrammingSolver + ?Sized,
    {
        snapshot.validate_identity()?;
        let descriptor = solver.descriptor();
        let support = solver.analyze_support(&snapshot);
        let matrix = CapabilityMatrix::from_constraint_programming_support(&descriptor, &support);
        Self::from_snapshot_with_options(snapshot, None, None, Some(descriptor), Some(matrix))
    }

    /// 从 baseline report 创建会话，但只保留稳定证据 / Create a session from a baseline report while retaining stable evidence only.
    pub fn from_baseline_report(
        snapshot: ConstraintProgrammingSnapshot,
        report: &SolveReport<i64>,
        solver_descriptor: Option<SolverDescriptor>,
        capability_matrix: Option<CapabilityMatrix>,
    ) -> Result<Self> {
        let mut session = Self::from_snapshot_with_options(
            snapshot,
            None,
            None,
            solver_descriptor,
            capability_matrix,
        )?;
        session.record_baseline_report(report)?;
        Ok(session)
    }

    /// 记录 baseline report / Record a baseline report.
    pub fn record_baseline_report(&mut self, report: &SolveReport<i64>) -> Result<AnalysisStatus> {
        self.ensure_open()?;
        report.validate()?;
        if let Some(model_fingerprint) = report.fingerprints.model.as_ref()
            && model_fingerprint != &self.baseline_snapshot.fingerprint
        {
            return Err(invalid_analysis(
                "baseline report model fingerprint does not match the analysis snapshot",
            ));
        }
        let solution = report
            .solution
            .as_ref()
            .map(|solution| solution.stable_values.clone());
        if let Some(values) = &solution {
            self.baseline_snapshot.validate_hint(values)?;
        }
        let objective = report.solution.as_ref().and_then(|solution| {
            solution
                .objective_value
                .or_else(|| solution.objective.map(|value| value as f64))
        });
        if let Some(value) = objective
            && !value.is_finite()
        {
            return Err(invalid_analysis(
                "baseline report objective value must be finite",
            ));
        }
        validate_baseline_objective(&self.baseline_snapshot, solution.as_ref(), objective)?;
        self.baseline_solution = solution;
        self.baseline_objective_value = objective;
        self.clear_caches();
        // The recorded baseline feeds every later objective comparison, so its status must be
        // proof-gated: a budget-limited incumbent is not a proven baseline.
        // 记录的基线会参与后续所有目标比较，因此其状态必须经过证明门控：
        // 受限求解得到的 incumbent 不构成已证明基线。
        let proven =
            report.problem_status == ProblemStatus::Feasible && report.is_optimal();
        Ok(AnalysisStatus::from_problem_status_with_proof(
            report.problem_status,
            proven,
        ))
    }

    /// 返回基线快照 / Return the baseline snapshot.
    pub fn baseline_snapshot(&self) -> &ConstraintProgrammingSnapshot {
        &self.baseline_snapshot
    }

    /// 返回兼容别名 snapshot / Return the compatibility snapshot alias.
    pub fn snapshot(&self) -> &ConstraintProgrammingSnapshot {
        self.baseline_snapshot()
    }

    /// 返回稳定 incumbent / Return the stable incumbent values.
    pub fn baseline_solution(&self) -> Option<&BTreeMap<crate::solver::StableVariableId, i64>> {
        self.baseline_solution.as_ref()
    }

    /// 返回 baseline 目标值 / Return the baseline objective value.
    pub fn baseline_objective_value(&self) -> Option<f64> {
        self.baseline_objective_value
    }

    /// 返回兼容别名 baseline objective / Return the compatibility baseline-objective alias.
    pub fn baseline_objective(&self) -> Option<f64> {
        self.baseline_objective_value()
    }

    /// 返回 solver descriptor / Return the solver descriptor.
    pub fn solver_descriptor(&self) -> Option<&SolverDescriptor> {
        self.solver_descriptor.as_ref()
    }

    /// 返回能力矩阵 / Return the capability matrix.
    pub fn capability_matrix(&self) -> &CapabilityMatrix {
        &self.capability_matrix
    }

    /// 将来源对应的状态放入缓存 / Cache a status for a diagnostic source.
    pub fn cache_status(
        &mut self,
        kind: AnalysisCacheKind,
        source: &DiagnosticSource,
        status: AnalysisStatus,
    ) -> Result<()> {
        self.ensure_open()?;
        source.validate()?;
        self.caches
            .entry(kind)
            .or_default()
            .insert(source.stable_id(), status);
        Ok(())
    }

    /// 查询来源对应的缓存状态 / Read a cached status for a diagnostic source.
    pub fn cached_status(
        &self,
        kind: AnalysisCacheKind,
        source: &DiagnosticSource,
    ) -> Result<Option<AnalysisStatus>> {
        self.ensure_open()?;
        source.validate()?;
        Ok(self
            .caches
            .get(&kind)
            .and_then(|entries| entries.get(&source.stable_id()).copied()))
    }

    /// 返回各缓存桶大小 / Return cache sizes by stage.
    pub fn cache_sizes(&self) -> BTreeMap<AnalysisCacheKind, usize> {
        AnalysisCacheKind::all()
            .into_iter()
            .map(|kind| {
                let size = self.caches.get(&kind).map_or(0, BTreeMap::len);
                (kind, size)
            })
            .collect()
    }

    /// 清空派生缓存 / Clear derived caches.
    pub fn clear_caches(&mut self) {
        self.caches.values_mut().for_each(BTreeMap::clear);
    }

    /// 关闭会话并释放缓存 / Close the session and release caches.
    pub fn close(&mut self) {
        self.clear_caches();
        self.closed = true;
    }

    /// 是否已关闭 / Whether the session is closed.
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    fn ensure_open(&self) -> Result<()> {
        if self.closed {
            Err(invalid_analysis("analysis session is closed"))
        } else {
            Ok(())
        }
    }
}

fn validate_baseline_objective(
    snapshot: &ConstraintProgrammingSnapshot,
    baseline_solution: Option<&BTreeMap<crate::solver::StableVariableId, i64>>,
    baseline_objective: Option<f64>,
) -> Result<()> {
    let Some(baseline_objective) = baseline_objective else {
        return Ok(());
    };
    if !baseline_objective.is_finite() {
        return Err(invalid_analysis("baseline objective value must be finite"));
    }
    let solution = baseline_solution.ok_or_else(|| {
        invalid_analysis(
            "a baseline objective value requires a complete baseline CP assignment",
        )
    })?;
    // Objective consistency is checked against the immutable CP expression, not against a
    // backend-reported objective. This keeps later ranking and perturbation comparisons tied to
    // the same source semantics.
    // 目标一致性以不可变 CP 表达式为准，而不是以后端上报值为准，确保后续排序和扰动比较
    // 始终使用同一份源语义。
    snapshot.validate_assignment(solution)?;
    let evaluated = snapshot
        .objective_value(solution)?
        .ok_or_else(|| invalid_analysis("a baseline objective requires exactly one CP objective"))?;
    let evaluated_f64 = evaluated as f64;
    if evaluated_f64 != baseline_objective {
        return Err(invalid_analysis(format!(
            "baseline objective does not match the baseline CP solution: {baseline_objective} != {evaluated_f64}"
        )));
    }
    Ok(())
}

impl Drop for CriticalConstraintAnalysisSession {
    fn drop(&mut self) {
        self.clear_caches();
    }
}

impl AnalysisCacheKind {
    fn all() -> [Self; 5] {
        [
            Self::Activity,
            Self::FixedIntegerLp,
            Self::Perturbation,
            Self::Target,
            Self::Conflict,
        ]
    }
}

fn empty_caches() -> BTreeMap<AnalysisCacheKind, BTreeMap<String, AnalysisStatus>> {
    AnalysisCacheKind::all()
        .into_iter()
        .map(|kind| (kind, BTreeMap::new()))
        .collect()
}

fn canonical_float(value: f64) -> String {
    let mut text = value.to_string();
    if !text.contains('.') && !text.contains('e') && !text.contains('E') {
        text.push_str(".0");
    }
    text
}

pub(crate) fn invalid_analysis(message: impl Into<String>) -> CoreError {
    CoreError::Solver(SolverError::InvalidInput(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::constraint_programming::{
        ConstraintDefinition, ConstraintProgrammingConstraint, ConstraintProgrammingModel,
        IntegerDomain, IntegerExpression, IntegerObjective, IntegerRelation, IntegerVariable,
    };
    use crate::solver::report::SolveSolution;

    fn snapshot() -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("analysis-protocol");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 2).expect("domain"))
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "capacity",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x.clone()),
                    IntegerRelation::GreaterOrEqual,
                    1,
                ),
            ))
            .expect("constraint");
        model.set_objective(IntegerObjective::maximize_with_id(
            "payload",
            IntegerExpression::variable(x),
        ));
        model.freeze().expect("snapshot")
    }

    #[test]
    fn analysis_status_keeps_unknown_and_unsupported_distinct() {
        assert_eq!(
            AnalysisStatus::from(ProblemStatus::Feasible),
            AnalysisStatus::Reachable
        );
        assert_eq!(
            AnalysisStatus::from(ProblemStatus::Infeasible),
            AnalysisStatus::Unreachable
        );
        assert_eq!(
            AnalysisStatus::from(ProblemStatus::Unknown),
            AnalysisStatus::Unknown
        );
        assert!(!AnalysisStatus::Unknown.is_proven());
        assert!(!AnalysisStatus::Unsupported.is_proven());
        assert_eq!(AnalysisStatus::SAT, AnalysisStatus::Reachable);
        assert_eq!(AnalysisStatus::UNSAT, AnalysisStatus::Unreachable);
    }

    #[test]
    fn objective_targets_have_stable_identity_and_directional_semantics() {
        let lower = ObjectiveTarget::at_least("payload", 10.0).expect("lower target");
        let upper = ObjectiveTarget::at_most("payload", 20.0).expect("upper target");
        assert!(lower.is_satisfied(10.0));
        assert!(!lower.is_satisfied(9.0));
        assert!(upper.is_satisfied(20.0));
        assert!(!upper.is_satisfied(21.0));
        assert_eq!(lower.signed_distance(12.0), 2.0);
        assert_eq!(lower.stable_id(), "objective-target:payload:at-least:10.0");
        assert!(ObjectiveTarget::at_least("", 1.0).is_err());
        assert!(ObjectiveTarget::at_least("payload", f64::NAN).is_err());
    }

    #[test]
    fn objective_target_relative_and_business_unit_factories_validate_inputs() {
        let relative_lower =
            ObjectiveTarget::at_least_relative("payload", 100.0, 0.05).expect("relative lower");
        let relative_upper =
            ObjectiveTarget::at_most_relative("payload", 100.0, 0.05).expect("relative upper");
        let business_lower =
            ObjectiveTarget::at_least_business_unit("payload", 100.0, 25.0)
                .expect("business lower");
        let business_upper =
            ObjectiveTarget::at_most_business_unit("payload", 100.0, 25.0)
                .expect("business upper");

        assert_eq!(relative_lower.value(), 105.0);
        assert_eq!(relative_upper.value(), 105.0);
        assert_eq!(business_lower.value(), 125.0);
        assert_eq!(business_upper.value(), 125.0);
        assert!(ObjectiveTarget::at_least_relative("payload", f64::NAN, 0.1).is_err());
        assert!(ObjectiveTarget::at_most_relative("payload", 100.0, f64::INFINITY).is_err());
        assert!(ObjectiveTarget::at_least_business_unit("payload", 100.0, f64::NAN).is_err());
        assert!(ObjectiveTarget::at_most_business_unit("payload", f64::MAX, f64::MAX).is_err());
    }

    #[test]
    fn diagnostic_sources_project_only_to_original_evidence() {
        let constraint = DiagnosticSource::constraint("capacity").expect("constraint");
        let lower = DiagnosticSource::variable_lower_bound("x").expect("lower");
        let upper = DiagnosticSource::VariableUpperBound {
            variable_id: "x".into(),
        };
        let domain = DiagnosticSource::sparse_domain("x").expect("domain");
        let target = DiagnosticSource::objective_target(
            ObjectiveTarget::at_least("payload", 11.0).expect("target"),
        )
        .expect("target source");

        assert_eq!(constraint.stable_id(), "constraint:capacity");
        assert_eq!(lower.stable_id(), "variable:x:lower");
        assert_eq!(upper.stable_id(), "variable:x:upper");
        assert_eq!(domain.stable_id(), "variable:x:domain");
        assert!(matches!(
            constraint.as_infeasibility_member(),
            Some(crate::solver::InfeasibilityEvidenceMember::Constraint(_))
        ));
        assert!(matches!(
            lower.as_infeasibility_member(),
            Some(crate::solver::InfeasibilityEvidenceMember::LowerBound(_))
        ));
        assert!(matches!(
            domain.as_infeasibility_member(),
            Some(crate::solver::InfeasibilityEvidenceMember::Domain(_))
        ));
        assert!(target.as_infeasibility_member().is_none());
        assert!(!constraint.stable_id().contains("row"));
        assert!(!constraint.stable_id().contains("column"));
    }

    #[test]
    fn capability_matrix_maps_solver_descriptor_and_keeps_fallback_explicit() {
        let descriptor = SolverDescriptor {
            solver_id: "test".to_owned(),
            display_name: "test".to_owned(),
            backend_name: "test-backend".to_owned(),
            backend_version: None,
            runtime_available: Some(true),
            capabilities: SolverCapabilities::default()
                .with("mip", CapabilitySupport::Supported)
                .with("dual", CapabilitySupport::Supported)
                .with("warm_start", CapabilitySupport::Supported),
            warnings: Vec::new(),
        };
        let matrix = CapabilityMatrix::from(&descriptor);
        assert!(matrix.supports(AnalysisCapability::Activity));
        assert!(matrix.supports(AnalysisCapability::FixedIntegerLpSensitivity));
        assert!(matrix.supports(AnalysisCapability::WarmStart));
        assert_eq!(
            matrix.support(AnalysisCapability::NativeUnsatCore),
            CapabilitySupport::Unsupported
        );
        assert!(!matrix.native_assumption_solving);
        // The descriptor declares no CP satisfaction path and no assumption capability, so the
        // matrix must not claim a fallback it cannot honour.
        // 该 descriptor 未声明任何 CP 满足路径或 assumption 能力，因此能力矩阵不得声称
        // 自己无法兑现的回退。
        assert!(!matrix.fallback_satisfaction_only);
        assert_eq!(
            matrix.support(AnalysisCapability::AssumptionSolving),
            CapabilitySupport::Unsupported
        );
        assert!(!matrix.supports_cp(ConstraintProgrammingFeature::AllDifferent));
    }

    #[test]
    fn capability_matrix_does_not_promote_rebuild_conflict_to_native() {
        let descriptor = SolverDescriptor {
            solver_id: "cp".to_owned(),
            display_name: "cp".to_owned(),
            backend_name: "cp-backend".to_owned(),
            backend_version: Some("1".to_owned()),
            runtime_available: Some(true),
            capabilities: SolverCapabilities::default().with("mip", CapabilitySupport::Supported),
            warnings: Vec::new(),
        };
        // This mirrors what the MIP-backed CP adapter reports for a fully supported snapshot:
        // assumptions and a verified, irreducible seed are supplied through snapshot rebuild,
        // not through a native backend core API.
        // 这对应 MIP-backed CP adapter 对完整 snapshot 的报告：assumption 和已验证、不可约
        // seed 通过 snapshot 重建提供，并不代表 backend 暴露了原生 core API。
        let support = ConstraintProgrammingSupportReport {
            satisfaction: true,
            integer_objective: true,
            sparse_domain: true,
            one_shot: true,
            rebuild_session: true,
            incremental_session: false,
            assumptions: true,
            cancellation: true,
            solution_hint: true,
            verified_conflict_seed: true,
            irreducible_conflict: true,
            progress: true,
            deterministic: true,
            solution_pool: false,
            constraints: BTreeMap::new(),
            notes: Vec::new(),
        };
        let matrix = CapabilityMatrix::from_constraint_programming_support(&descriptor, &support);

        assert!(!matrix.native_assumption_solving);
        assert!(!matrix.native_unsat_core);
        assert_eq!(
            matrix.support(AnalysisCapability::NativeUnsatCore),
            CapabilitySupport::Unsupported
        );
        assert_eq!(
            matrix.support(AnalysisCapability::AssumptionSolving),
            CapabilitySupport::Conditional
        );
        assert!(!matrix.is_native(AnalysisCapability::NativeUnsatCore));
        assert_eq!(
            matrix.cp_support(ConstraintProgrammingFeature::ConflictCore),
            ConstraintProgrammingSupport::Conditional
        );
        // A backend without an incremental session must not advertise one.
        // 不具备增量会话的后端不得声称支持增量求解。
        assert_eq!(
            matrix.cp_support(ConstraintProgrammingFeature::IncrementalSolve),
            ConstraintProgrammingSupport::Unsupported
        );
    }

    #[test]
    fn capability_matrix_accepts_explicit_native_backend_declarations() {
        let descriptor = SolverDescriptor {
            solver_id: "native-cp".to_owned(),
            display_name: "native-cp".to_owned(),
            backend_name: "native-cp".to_owned(),
            backend_version: Some("1".to_owned()),
            runtime_available: Some(true),
            capabilities: SolverCapabilities::default()
                .with("constraint_programming", CapabilitySupport::Supported)
                .with("assumption", CapabilitySupport::Supported)
                .with("unsat_core", CapabilitySupport::Supported),
            warnings: Vec::new(),
        };
        let support = ConstraintProgrammingSupportReport {
            satisfaction: true,
            integer_objective: true,
            sparse_domain: true,
            one_shot: true,
            rebuild_session: true,
            incremental_session: true,
            assumptions: true,
            cancellation: true,
            solution_hint: true,
            verified_conflict_seed: true,
            irreducible_conflict: true,
            progress: true,
            deterministic: true,
            solution_pool: false,
            constraints: BTreeMap::new(),
            notes: Vec::new(),
        };
        let matrix = CapabilityMatrix::from_constraint_programming_support(&descriptor, &support);
        assert!(matrix.native_assumption_solving);
        assert!(matrix.native_unsat_core);
        assert_eq!(
            matrix.support(AnalysisCapability::NativeUnsatCore),
            CapabilitySupport::Supported
        );
        assert_eq!(
            matrix.support(AnalysisCapability::AssumptionSolving),
            CapabilitySupport::Supported
        );
    }

    #[test]
    fn analysis_status_requires_proof_before_claiming_a_conclusion() {
        // A budget-limited run can report Feasible without proving optimality; that must never
        // be upgraded into a proven reachability conclusion.
        // 受限运行可能在未证明最优性的情况下返回 Feasible，这绝不能升格为已证明的可达结论。
        assert_eq!(
            AnalysisStatus::from_problem_status_with_proof(ProblemStatus::Feasible, true),
            AnalysisStatus::Reachable
        );
        assert_eq!(
            AnalysisStatus::from_problem_status_with_proof(ProblemStatus::Feasible, false),
            AnalysisStatus::Unknown
        );
        assert_eq!(
            AnalysisStatus::from_problem_status_with_proof(ProblemStatus::Infeasible, true),
            AnalysisStatus::Unreachable
        );
        assert_eq!(
            AnalysisStatus::from_problem_status_with_proof(ProblemStatus::Infeasible, false),
            AnalysisStatus::Unknown
        );
        assert!(
            !AnalysisStatus::from_problem_status_with_proof(ProblemStatus::Feasible, false)
                .is_proven()
        );
        assert!(
            !AnalysisStatus::from_problem_status_with_proof(ProblemStatus::Infeasible, false)
                .is_proven()
        );
    }

    #[test]
    fn exact_cp_lowering_requires_every_snapshot_constraint() {
        let descriptor = SolverDescriptor {
            solver_id: "mixed-cp".to_owned(),
            display_name: "mixed-cp".to_owned(),
            backend_name: "mixed-cp".to_owned(),
            backend_version: Some("1".to_owned()),
            runtime_available: Some(true),
            capabilities: SolverCapabilities::default()
                .with("mip", CapabilitySupport::Supported),
            warnings: Vec::new(),
        };
        let support = ConstraintProgrammingSupportReport {
            constraints: BTreeMap::from([
                ("supported".into(), ConstraintProgrammingSupport::ExactLowering),
                ("unsupported".into(), ConstraintProgrammingSupport::Unsupported),
            ]),
            satisfaction: true,
            integer_objective: true,
            sparse_domain: true,
            one_shot: true,
            rebuild_session: true,
            incremental_session: false,
            assumptions: true,
            cancellation: true,
            solution_hint: true,
            verified_conflict_seed: true,
            irreducible_conflict: true,
            progress: true,
            deterministic: true,
            solution_pool: false,
            notes: Vec::new(),
        };
        let matrix = CapabilityMatrix::from_constraint_programming_support(&descriptor, &support);
        assert!(!matrix.exact_cp_lowering);
        assert_eq!(
            matrix.support(AnalysisCapability::ExactCpLowering),
            CapabilitySupport::Unsupported
        );
    }

    #[test]
    fn session_retains_snapshot_and_clears_derived_caches_on_close() {
        let snapshot = snapshot();
        let expected_fingerprint = snapshot.fingerprint.clone();
        let values = BTreeMap::from([(crate::solver::StableVariableId::from("x"), 1_i64)]);
        let mut session = CriticalConstraintAnalysisSession::from_snapshot_with_options(
            snapshot.clone(),
            Some(values),
            Some(1.0),
            None,
            None,
        )
        .expect("session");
        let source = DiagnosticSource::constraint("capacity").expect("source");
        session
            .cache_status(
                AnalysisCacheKind::Activity,
                &source,
                AnalysisStatus::Reachable,
            )
            .expect("cache");
        assert_eq!(session.snapshot(), &snapshot);
        assert_eq!(
            session.baseline_snapshot().fingerprint,
            expected_fingerprint
        );
        assert_eq!(session.cache_sizes()[&AnalysisCacheKind::Activity], 1);
        assert_eq!(
            session
                .cached_status(AnalysisCacheKind::Activity, &source)
                .unwrap(),
            Some(AnalysisStatus::Reachable)
        );
        assert!(!session.is_closed());

        session.close();

        assert!(session.is_closed());
        assert_eq!(session.cache_sizes().values().sum::<usize>(), 0);
        assert!(
            session
                .cached_status(AnalysisCacheKind::Activity, &source)
                .is_err()
        );
    }

    #[test]
    fn session_records_stable_baseline_without_retaining_row_mapping() {
        let snapshot = snapshot();
        let mut solution = SolveSolution::vector(vec![1_i64]);
        solution.stable_values.insert("x".into(), 1);
        solution.objective = Some(1);
        let report = SolveReport::builder(
            ProblemStatus::Feasible,
            crate::solver::TerminationReason::Completed,
        )
        .solution(solution)
        .fingerprints(crate::solver::SolveFingerprints {
            model: Some(snapshot.fingerprint.clone()),
            ..Default::default()
        })
        .proof(crate::solver::SolveProof::optimality())
        .build()
        .expect("baseline report");
        let session =
            CriticalConstraintAnalysisSession::from_baseline_report(snapshot, &report, None, None)
                .expect("session from report");
        assert_eq!(session.baseline_objective_value(), Some(1.0));
        assert_eq!(session.baseline_solution().map(BTreeMap::len), Some(1));
    }

    #[test]
    fn session_rejects_baseline_objective_that_does_not_match_expression() {
        let snapshot = snapshot();
        let values = BTreeMap::from([(crate::solver::StableVariableId::from("x"), 1_i64)]);
        let error = CriticalConstraintAnalysisSession::from_snapshot_with_options(
            snapshot,
            Some(values),
            Some(5.0),
            None,
            None,
        )
        .expect_err("mismatched baseline objective");
        assert!(format!("{error}").contains("baseline objective does not match"));
    }

    #[test]
    fn session_rejects_baseline_objective_without_a_complete_solution() {
        let snapshot = snapshot();
        let error = CriticalConstraintAnalysisSession::from_snapshot_with_options(
            snapshot,
            None,
            Some(1.0),
            None,
            None,
        )
        .expect_err("objective without baseline solution");
        assert!(format!("{error}").contains("complete baseline CP assignment"));
    }
}
