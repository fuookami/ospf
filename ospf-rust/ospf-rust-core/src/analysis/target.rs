//! Objective-target feasibility analysis / 目标可行性分析。
//!
//! A target is checked by rebuilding an immutable CP snapshot with one temporary
//! integer constraint. The source snapshot and its objective are never mutated,
//! and the public report exposes only source-model identities.
//! 目标检查通过在不可变 CP snapshot 上追加一个临时整数约束完成。原始 snapshot
//! 和目标定义不会被修改，公共报告只暴露原始模型身份。

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use crate::error::{CoreError, Result, SolverError};
use crate::model::constraint_programming::{
    ConstraintProgrammingConstraint, ConstraintProgrammingSnapshot, ConstraintSnapshot,
    IntegerDomain, IntegerRelation,
};
use crate::solver::constraint_programming::{
    ConstraintProgrammingAssumption, ConstraintProgrammingSolveOptions,
};
use crate::solver::fingerprint::solver_descriptor_fingerprint;
use crate::solver::report::{
    InfeasibilityEvidence, ProblemStatus, ProofStatus, SolveReport, TerminationReason,
};
use crate::solver::{ConstraintProgrammingSolver, StableConstraintId, StableVariableId};

use super::{
    AnalysisCacheKind, AnalysisCapability, AnalysisStatus, CapabilityMatrix, CapabilitySupport,
    CriticalConstraintAnalysisSession, DiagnosticSource, ObjectiveTarget, ObjectiveTargetRelation,
};

/// Target-feasibility report schema / 目标可行性报告 schema。
pub const TARGET_FEASIBILITY_REPORT_SCHEMA_VERSION: &str = "1.0";

/// Stable ID prefix for a temporary target constraint / 临时 target 约束的稳定 ID 前缀。
pub const OBJECTIVE_TARGET_CONSTRAINT_PREFIX: &str = "analysis-target:";

/// Result of checking one objective target / 一个目标条件的检查结果。
///
/// `Reachable` is justified by a validated witness. `Unreachable` is emitted
/// only after the solver report passes the verified infeasibility gate. A time
/// limit, backend failure, or unverified infeasibility remains `Unknown`.
/// `Reachable` 由经过校验的见证解支持；`Unreachable` 只有在 solver report
/// 通过不可行性证明门槛后才会产生。超时、后端失败或未验证不可行结果均保持 `Unknown`。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct TargetFeasibilityReport {
    /// Report schema / 报告 schema。
    pub schema_version: String,
    /// Requested target / 请求的目标条件。
    pub target: ObjectiveTarget,
    /// Exact integer bound inserted into the CP model / 写入 CP 模型的精确整数边界。
    pub effective_integer_bound: Option<i64>,
    /// Stable target evidence source / 稳定的 target 证据来源。
    pub source: DiagnosticSource,
    /// Reachability conclusion / 可达性结论。
    pub status: AnalysisStatus,
    /// Validated source-variable witness / 经过校验的原始变量见证解。
    pub solution: Option<BTreeMap<StableVariableId, i64>>,
    /// Exact objective value from the source CP expression / 原始 CP 表达式的精确目标值。
    pub exact_objective: Option<i64>,
    /// Floating-point compatibility value / 浮点兼容目标值。
    pub objective_value: Option<f64>,
    /// Proof status supplied by the backend / 后端提供的证明状态。
    pub proof_status: ProofStatus,
    /// Backend termination reason / 后端终止原因。
    pub termination_reason: Option<TerminationReason>,
    /// Backend solve duration / 后端求解耗时。
    pub solve_time: Option<Duration>,
    /// Whether a target constraint was inserted / 是否确实插入了 target 约束。
    pub target_fixed: bool,
    /// Human-readable diagnostic detail / 面向诊断的详情。
    pub message: Option<String>,
}

impl TargetFeasibilityReport {
    /// Validate report invariants / 校验报告不变量。
    pub fn validate(&self) -> Result<()> {
        if self.schema_version.trim().is_empty() {
            return Err(invalid_target(
                "target report schema version must not be blank",
            ));
        }
        self.target.validate()?;
        let expected_source = DiagnosticSource::ObjectiveTarget {
            target: self.target.clone(),
        };
        if self.source != expected_source {
            return Err(invalid_target(
                "target report source does not match the requested target",
            ));
        }
        if self.effective_integer_bound.is_some_and(|bound| {
            !target_bound_matches(self.target.relation(), self.target.value(), bound)
        }) {
            return Err(invalid_target(
                "target report effective integer bound is not an exact target bound",
            ));
        }
        if self.objective_value.is_some_and(|value| !value.is_finite()) {
            return Err(invalid_target(
                "target report objective value must be finite",
            ));
        }
        if self.status == AnalysisStatus::Reachable && self.solution.is_none() {
            return Err(invalid_target(
                "reachable target report must contain a validated witness",
            ));
        }
        if let Some(solution) = &self.solution
            && solution.keys().any(|id| id.0.trim().is_empty())
        {
            return Err(invalid_target(
                "target report solution contains a blank variable identity",
            ));
        }
        Ok(())
    }

    /// Compatibility reachability projection / 兼容的可达性投影。
    pub fn reachable(&self) -> Option<bool> {
        match self.status {
            AnalysisStatus::Reachable => Some(true),
            AnalysisStatus::Unreachable => Some(false),
            AnalysisStatus::Unknown | AnalysisStatus::Unsupported => None,
        }
    }

    /// Whether the target result is proven / 是否为已证明结论。
    pub const fn is_proven(&self) -> bool {
        self.status.is_proven()
    }

    /// Return the target source / 返回 target 来源。
    pub fn target_source(&self) -> &DiagnosticSource {
        &self.source
    }
}

/// Explicit cache for target-feasibility reports / target 可行性报告的显式缓存。
#[derive(Debug, Clone, Default)]
pub struct TargetFeasibilityCache {
    entries: BTreeMap<String, TargetFeasibilityReport>,
}

impl TargetFeasibilityCache {
    /// Create an empty cache / 创建空缓存。
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all cached reports / 清空缓存。
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Return the number of cached reports / 返回缓存条目数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty / 判断缓存是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Objective-target feasibility analyzer / ObjectiveTarget 可行性分析器。
///
/// The analyzer is stateless. A solver is supplied for each call so that the
/// same report protocol works with the fake solver, exact MIP-backed CP, and
/// future native CP adapters.
/// 分析器本身无状态，每次调用传入 solver，使 fake、精确 MIP-backed CP 和未来原生
/// CP adapter 共享同一报告协议。
#[derive(Debug, Clone, Copy, Default)]
pub struct TargetFeasibilityAnalyzer;

impl TargetFeasibilityAnalyzer {
    /// Create an analyzer / 创建分析器。
    pub const fn new() -> Self {
        Self
    }

    /// Analyze a target using runtime solver capabilities / 使用 solver 运行时能力分析 target。
    pub fn analyze<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<TargetFeasibilityReport> {
        let support = solver.analyze_support(snapshot);
        let descriptor = solver.descriptor();
        let capability =
            CapabilityMatrix::from_constraint_programming_support(&descriptor, &support);
        self.analyze_with_capability(solver, snapshot, target, options, &capability)
    }

    /// Analyze using an explicit capability matrix / 使用显式能力矩阵分析 target。
    pub fn analyze_with_capability<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        options: &ConstraintProgrammingSolveOptions<'_>,
        capability: &CapabilityMatrix,
    ) -> Result<TargetFeasibilityReport> {
        self.analyze_with_capability_and_conflict(
            solver,
            snapshot,
            target,
            options,
            capability,
            false,
        )
        .map(|(report, _)| report)
    }

    /// Analyze a target and optionally request backend conflict evidence.
    /// 分析 target，并可选请求 backend 冲突证据。
    ///
    /// The evidence is returned only through this crate-private bridge so the public target
    /// report remains target-only. Any generated row identity must already have been remapped by
    /// the CP backend before the conflict analyzer consumes it.
    /// 证据只通过 crate 内部桥接返回，使公开 target report 保持仅包含 target；任何生成行身份
    /// 都必须在冲突分析器消费前由 CP backend 完成回映。
    pub(crate) fn analyze_with_capability_and_conflict<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        options: &ConstraintProgrammingSolveOptions<'_>,
        capability: &CapabilityMatrix,
        request_conflict: bool,
    ) -> Result<(TargetFeasibilityReport, Option<InfeasibilityEvidence>)> {
        self.analyze_with_capability_and_conflict_and_assumptions(
            solver,
            snapshot,
            target,
            options,
            capability,
            &[],
            request_conflict,
        )
    }

    /// Analyze a target with an explicit active-assumption set.
    /// 使用显式激活 assumptions 分析 target。
    pub(crate) fn analyze_with_capability_and_conflict_and_assumptions<
        S: ConstraintProgrammingSolver + ?Sized,
    >(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        options: &ConstraintProgrammingSolveOptions<'_>,
        capability: &CapabilityMatrix,
        assumptions: &[ConstraintProgrammingAssumption],
        request_conflict: bool,
    ) -> Result<(TargetFeasibilityReport, Option<InfeasibilityEvidence>)> {
        snapshot.validate_identity()?;
        target.validate()?;
        validate_target_objective(snapshot, target)?;
        if capability.support(AnalysisCapability::ObjectiveTarget) == CapabilitySupport::Unsupported
        {
            return Ok((unsupported_report(
                target,
                None,
                "solver capability matrix does not support objective targets",
            ), None));
        }

        let support = solver.analyze_support(snapshot);
        if !support.satisfaction || !support.integer_objective {
            return Ok((unsupported_report(
                target,
                None,
                "solver does not provide exact CP satisfaction with an integer objective",
            ), None));
        }

        let bound = match exact_integer_bound(target) {
            Some(bound) => bound,
            None => {
                return Ok((unsupported_report(
                    target,
                    None,
                    "objective target is outside the representable i64 integer bound",
                ), None));
            }
        };
        if snapshot.objective.is_none() {
            return Ok((unsupported_report(
                target,
                Some(bound),
                "CP snapshot has no objective expression to target",
            ), None));
        }
        let active_sources = if assumptions.is_empty() {
            None
        } else {
            Some(active_sources_for_assumptions(snapshot, assumptions))
        };
        let derived = derive_target_snapshot(
            snapshot,
            target,
            bound,
            None,
            active_sources.as_deref(),
        )?;
        self.solve_derived_with_conflict(
            solver,
            &derived,
            target,
            bound,
            options,
            assumptions,
            request_conflict,
        )
    }

    /// Analyze with default CP options / 使用默认 CP 参数分析 target。
    pub fn analyze_default<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
    ) -> Result<TargetFeasibilityReport> {
        let options = ConstraintProgrammingSolveOptions::new();
        self.analyze(solver, snapshot, target, &options)
    }

    /// Analyze an ordered batch of objective targets / 按顺序分析一批 objective target。
    ///
    /// Each target is checked against the same immutable snapshot. Duplicate stable target IDs
    /// are rejected so a caller cannot accidentally produce ambiguous batch results.
    /// 每个 target 都针对同一份不可变 snapshot 检查。重复的稳定 target ID 会被拒绝，避免调用方
    /// 意外生成含义不明确的批量结果。
    pub fn analyze_many<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        targets: &[ObjectiveTarget],
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<Vec<TargetFeasibilityReport>> {
        if targets.is_empty() {
            return Err(invalid_target(
                "target batch requires at least one objective target",
            ));
        }
        let mut seen = BTreeSet::new();
        for target in targets {
            target.validate()?;
            if !seen.insert(target.stable_id()) {
                return Err(invalid_target("target batch contains duplicate targets"));
            }
        }
        targets
            .iter()
            .map(|target| self.analyze(solver, snapshot, target, options))
            .collect()
    }

    /// Analyze a batch with default CP options / 使用默认 CP 选项分析一批 target。
    pub fn analyze_many_default<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        targets: &[ObjectiveTarget],
    ) -> Result<Vec<TargetFeasibilityReport>> {
        self.analyze_many(
            solver,
            snapshot,
            targets,
            &ConstraintProgrammingSolveOptions::new(),
        )
    }

    /// Analyze and use an explicit report cache / 分析并使用显式报告缓存。
    pub fn analyze_cached<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        options: &ConstraintProgrammingSolveOptions<'_>,
        cache: &mut TargetFeasibilityCache,
    ) -> Result<TargetFeasibilityReport> {
        let key = target_cache_key(solver, snapshot, target, options);
        if let Some(report) = cache.entries.get(&key) {
            return Ok(report.clone());
        }
        let report = self.analyze(solver, snapshot, target, options)?;
        cache.entries.insert(key, report.clone());
        Ok(report)
    }

    /// Analyze a session baseline and record the stable target status / 分析 session 基线并记录稳定状态。
    pub fn analyze_session<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        session: &mut CriticalConstraintAnalysisSession,
        target: &ObjectiveTarget,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<TargetFeasibilityReport> {
        let report = self.analyze(solver, session.baseline_snapshot(), target, options)?;
        session.cache_status(AnalysisCacheKind::Target, &report.source, report.status)?;
        Ok(report)
    }

    /// Internal active-constraint target check used by conflict shrinking.
    /// 冲突收缩使用的活动约束子集 target 检查。
    pub(crate) fn analyze_active_constraints<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        active_ids: &BTreeSet<StableConstraintId>,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<TargetFeasibilityReport> {
        let active_sources = snapshot
            .constraints
            .iter()
            .filter(|constraint| active_ids.contains(&constraint.id))
            .map(|constraint| DiagnosticSource::Constraint {
                id: constraint.id.clone(),
            })
            .chain(
                crate::analysis::activation::DiagnosticActivationSet::from_snapshot(snapshot)
                    .active_sources()
                    .into_iter()
                    .filter(|source| !matches!(source, DiagnosticSource::Constraint { .. })),
            )
            .collect::<Vec<_>>();
        self.analyze_active_sources(solver, snapshot, target, &active_sources, options)
    }

    /// Analyze a target with an explicitly activated original evidence set.
    /// 使用显式激活的原始证据集合分析 target。
    pub(crate) fn analyze_active_sources<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        active_sources: &[DiagnosticSource],
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<TargetFeasibilityReport> {
        snapshot.validate_identity()?;
        target.validate()?;
        validate_target_objective(snapshot, target)?;
        let support = solver.analyze_support(snapshot);
        let descriptor = solver.descriptor();
        let capability =
            CapabilityMatrix::from_constraint_programming_support(&descriptor, &support);
        if capability.support(AnalysisCapability::ObjectiveTarget) == CapabilitySupport::Unsupported
            || !support.satisfaction
            || !support.integer_objective
        {
            return Ok(unsupported_report(
                target,
                None,
                "solver does not support exact CP target feasibility for this snapshot",
            ));
        }
        let bound = match exact_integer_bound(target) {
            Some(bound) => bound,
            None => {
                return Ok(unsupported_report(
                    target,
                    None,
                    "objective target is outside the representable i64 integer bound",
                ));
            }
        };
        if snapshot.objective.is_none() {
            return Ok(unsupported_report(
                target,
                Some(bound),
                "CP snapshot has no objective expression to target",
            ));
        }
        let activation_set = activation_set_for_sources(snapshot, active_sources)?;
        let assumptions = activation_set.to_assumptions(snapshot);
        let derived = derive_target_snapshot(snapshot, target, bound, None, Some(active_sources))?;
        self.solve_derived_with_conflict(
            solver,
            &derived,
            target,
            bound,
            options,
            &assumptions,
            false,
        )
        .map(|(report, _)| report)
    }

    fn solve_derived<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        derived: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        bound: i64,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<TargetFeasibilityReport> {
        self.solve_derived_with_conflict(solver, derived, target, bound, options, &[], false)
            .map(|(report, _)| report)
    }

    pub(crate) fn solve_derived_with_conflict<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        derived: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        bound: i64,
        options: &ConstraintProgrammingSolveOptions<'_>,
        assumptions: &[ConstraintProgrammingAssumption],
        request_conflict: bool,
    ) -> Result<(TargetFeasibilityReport, Option<InfeasibilityEvidence>)> {
        let started = std::time::Instant::now();
        let conflict_options = ConstraintProgrammingSolveOptions {
            request_conflict: true,
            shrink_conflict: false,
            ..*options
        };
        let effective_options = if request_conflict {
            &conflict_options
        } else {
            options
        };
        let backend_report = match if request_conflict && assumptions.is_empty() {
            solver.solve_constraint_programming_with_conflict(derived, effective_options)
        } else if request_conflict || !assumptions.is_empty() {
            solver.solve_constraint_programming_with_assumptions_and_conflict(
                derived,
                assumptions,
                effective_options,
            )
        } else {
            solver.solve_constraint_programming(derived, effective_options)
        } {
            Ok(report) => report,
            Err(error) => {
                return Ok((unknown_report(
                    target,
                    Some(bound),
                    Some(started.elapsed()),
                    None,
                    None,
                    "CP target solve failed",
                    Some(error.to_string()),
                ), None));
            }
        };
        let evidence = backend_report.diagnostics.infeasibility_evidence.clone();
        let proof_status = backend_report
            .proof
            .as_ref()
            .map_or(ProofStatus::None, |proof| proof.status);
        let solve_time = Some(backend_report.statistics.solve_time);
        if let Err(error) = backend_report.validate() {
            return Ok((unknown_report(
                target,
                Some(bound),
                solve_time,
                Some(proof_status),
                Some(backend_report.termination_reason),
                "CP target backend report failed validation",
                Some(error.to_string()),
            ), evidence));
        }

        match backend_report.problem_status {
            ProblemStatus::Feasible | ProblemStatus::Unknown => {
                if let Some(assignment) = assignment_from_report(derived, &backend_report)
                    && derived.validate_assignment(&assignment).is_ok()
                {
                    let exact_objective =
                        derived.objective_value(&assignment)?.ok_or_else(|| {
                            invalid_target("CP target witness has no objective value")
                        })?;
                    if target_integer_satisfies(target, exact_objective, bound) {
                        let objective_value = backend_report
                            .solution
                            .as_ref()
                            .and_then(|solution| solution.objective_value)
                            .or_else(|| finite_i64(exact_objective));
                        return Ok((TargetFeasibilityReport {
                            schema_version: TARGET_FEASIBILITY_REPORT_SCHEMA_VERSION.to_owned(),
                            target: target.clone(),
                            effective_integer_bound: Some(bound),
                            source: target_source(target),
                            status: AnalysisStatus::Reachable,
                            solution: Some(assignment),
                            exact_objective: Some(exact_objective),
                            objective_value,
                            proof_status,
                            termination_reason: Some(backend_report.termination_reason),
                            solve_time,
                            target_fixed: true,
                            message: if backend_report.problem_status == ProblemStatus::Unknown {
                                Some("target witness found without a completed proof".to_owned())
                            } else {
                                None
                            },
                        }, evidence));
                    }
                }
                Ok((unknown_report(
                    target,
                    Some(bound),
                    solve_time,
                    Some(proof_status),
                    Some(backend_report.termination_reason),
                    "CP target solve did not provide a valid target witness",
                    None,
                ), evidence))
            }
            ProblemStatus::Infeasible => {
                if crate::solver::require_infeasibility_certificate(&backend_report).is_ok() {
                    Ok((TargetFeasibilityReport {
                        schema_version: TARGET_FEASIBILITY_REPORT_SCHEMA_VERSION.to_owned(),
                        target: target.clone(),
                        effective_integer_bound: Some(bound),
                        source: target_source(target),
                        status: AnalysisStatus::Unreachable,
                        solution: None,
                        exact_objective: None,
                        objective_value: None,
                        proof_status,
                        termination_reason: Some(backend_report.termination_reason),
                        solve_time,
                        target_fixed: true,
                        message: None,
                    }, evidence))
                } else {
                    Ok((unknown_report(
                        target,
                        Some(bound),
                        solve_time,
                        Some(proof_status),
                        Some(backend_report.termination_reason),
                        "CP backend reported infeasible without a verified certificate",
                        None,
                    ), evidence))
                }
            }
            ProblemStatus::Unbounded | ProblemStatus::InfeasibleOrUnbounded => Ok((
                unknown_report(
                    target,
                    Some(bound),
                    solve_time,
                    Some(proof_status),
                    Some(backend_report.termination_reason),
                    "CP target solve returned an ambiguous or unbounded status",
                    None,
                ),
                evidence,
            )),
        }
    }
}

fn derive_target_snapshot(
    snapshot: &ConstraintProgrammingSnapshot,
    target: &ObjectiveTarget,
    bound: i64,
    active_ids: Option<&BTreeSet<StableConstraintId>>,
    active_sources: Option<&[DiagnosticSource]>,
) -> Result<ConstraintProgrammingSnapshot> {
    let objective = snapshot
        .objective
        .as_ref()
        .ok_or_else(|| invalid_target("CP snapshot has no objective expression"))?;
    validate_target_objective(snapshot, target)?;
    let target_id = target_constraint_id(target);
    if snapshot.constraint(&target_id).is_some() {
        return Err(invalid_target(format!(
            "temporary objective target ID collides with source constraint {}",
            target_id.0
        )));
    }
    let relation = match target.relation() {
        ObjectiveTargetRelation::AtLeast => IntegerRelation::GreaterOrEqual,
        ObjectiveTargetRelation::AtMost => IntegerRelation::LessOrEqual,
    };
    let target_constraint = ConstraintSnapshot {
        id: target_id,
        name: format!("{OBJECTIVE_TARGET_CONSTRAINT_PREFIX}{}", target.stable_id()),
        group: None,
        origin: Some("ospf.analysis.objective-target".to_owned()),
        constraint: ConstraintProgrammingConstraint::integer(
            objective.expression.clone(),
            relation,
            bound,
        ),
    };
    let mut derived = snapshot.clone();
    if let Some(active_sources) = active_sources {
        relax_inactive_domains(&mut derived, active_sources, target)?;
    }
    let mut constraints = if let Some(active_ids) = active_ids {
        snapshot
            .constraints
            .iter()
            .filter(|constraint| active_ids.contains(&constraint.id))
            .cloned()
            .collect::<Vec<_>>()
    } else if let Some(active_sources) = active_sources {
        let active_ids = active_sources
            .iter()
            .filter_map(|source| match source {
                DiagnosticSource::Constraint { id } => Some(id),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        snapshot
            .constraints
            .iter()
            .filter(|constraint| active_ids.contains(&constraint.id))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        snapshot.constraints.clone()
    };
    constraints.push(target_constraint);
    derived.constraints = constraints;
    derived.constraints.sort_by(|left, right| left.id.cmp(&right.id));
    derived.fingerprint = derived.compute_fingerprint();
    derived.validate_identity()?;
    Ok(derived)
}

fn activation_set_for_sources(
    snapshot: &ConstraintProgrammingSnapshot,
    sources: &[DiagnosticSource],
) -> Result<crate::analysis::activation::DiagnosticActivationSet> {
    let mut set = crate::analysis::activation::DiagnosticActivationSet::new();
    let mut identities = BTreeSet::new();
    for source in sources {
        source.validate()?;
        if !identities.insert(source.stable_id()) {
            return Err(invalid_target("duplicate active diagnostic evidence source"));
        }
        match source {
            DiagnosticSource::Constraint { id } if snapshot.constraint(id).is_none() => {
                return Err(invalid_target(format!(
                    "active diagnostic source references unknown constraint {}",
                    id.0
                )));
            }
            DiagnosticSource::VariableLowerBound { variable_id }
            | DiagnosticSource::VariableUpperBound { variable_id }
            | DiagnosticSource::SparseDomain { variable_id }
                if snapshot.variable(variable_id).is_none() =>
            {
                return Err(invalid_target(format!(
                    "active diagnostic source references unknown variable {}",
                    variable_id.0
                )));
            }
            DiagnosticSource::VariableBound { bound }
                if snapshot.variable(&bound.variable_id).is_none() =>
            {
                return Err(invalid_target(format!(
                    "active diagnostic source references unknown variable {}",
                    bound.variable_id.0
                )));
            }
            DiagnosticSource::ObjectiveTarget { .. } => {
                return Err(invalid_target(
                    "objective target cannot be an active model evidence source",
                ));
            }
            _ => {}
        }
        set.activate_source(source.clone())?;
    }
    Ok(set)
}

fn active_sources_for_assumptions(
    snapshot: &ConstraintProgrammingSnapshot,
    _assumptions: &[ConstraintProgrammingAssumption],
) -> Vec<DiagnosticSource> {
    // Assumption-backed bounds/domains must be represented exactly once. Keep only the original
    // constraints in the derived snapshot; `solve_with_assumptions` supplies the variable bounds
    // and sparse domains. Retaining them in both places can make the empty-assumption baseline
    // infeasible before the backend consumes the assumptions, falsely downgrading a valid
    // assumption extraction to repeated solving.
    // 对由 assumption 承载的边界/值域只保留一份表达：derived snapshot 只保留原始约束，变量
    // 边界和稀疏域由 `solve_with_assumptions` 注入。两处同时保留会使空 assumption 基线可能
    // 在后端消费 assumptions 前已经不可行，从而把有效提取错误降级为重复求解。
    let sources = snapshot
        .constraints
        .iter()
        .map(|constraint| DiagnosticSource::Constraint {
            id: constraint.id.clone(),
        })
        .collect::<Vec<_>>();
    sources
}

fn relax_inactive_domains(
    snapshot: &mut ConstraintProgrammingSnapshot,
    active_sources: &[DiagnosticSource],
    target: &ObjectiveTarget,
) -> Result<()> {
    let active_ids = active_sources
        .iter()
        .map(DiagnosticSource::stable_id)
        .collect::<BTreeSet<_>>();
    let target_bound = exact_integer_bound(target);
    for variable in &mut snapshot.variables {
        let variable_id = variable.variable.stable_id.clone();
        let lower_active = active_ids.contains(
            &DiagnosticSource::VariableLowerBound {
                variable_id: variable_id.clone(),
            }
            .stable_id(),
        ) || active_ids.contains(
            &DiagnosticSource::VariableBound {
                bound: super::VariableBoundRef {
                    variable_id: variable_id.clone(),
                    side: super::BoundSide::Lower,
                },
            }
            .stable_id(),
        );
        let upper_active = active_ids.contains(
            &DiagnosticSource::VariableUpperBound {
                variable_id: variable_id.clone(),
            }
            .stable_id(),
        ) || active_ids.contains(
            &DiagnosticSource::VariableBound {
                bound: super::VariableBoundRef {
                    variable_id: variable_id.clone(),
                    side: super::BoundSide::Upper,
                },
            }
            .stable_id(),
        );
        let domain_active = active_ids.contains(
            &DiagnosticSource::SparseDomain {
                variable_id,
            }
            .stable_id(),
        );
        let (mut lower, mut upper) = (variable.domain.lower(), variable.domain.upper());
        if !lower_active && let Some(bound) = target_bound {
            lower = lower.min(bound);
        }
        if !upper_active && let Some(bound) = target_bound {
            upper = upper.max(bound);
        }
        // An active sparse domain is already the exact representation of that evidence. Never
        // replace it with its continuous hull merely because one of its redundant endpoint
        // bounds was removed during MUS shrinking; doing so can admit values that the sparse
        // evidence explicitly forbids.
        // 激活的稀疏域已经是该证据的精确表达。MUS 收缩删除冗余端点边界时，不能因此把它
        // 替换为连续包络，否则会放行稀疏证据明确禁止的值。
        let preserve_sparse_domain =
            domain_active && matches!(variable.domain, IntegerDomain::Values(_));
        if !preserve_sparse_domain && (!domain_active || !lower_active || !upper_active) {
            variable.domain = crate::model::constraint_programming::IntegerDomain::range(lower, upper)?;
        }
    }
    Ok(())
}

fn validate_target_objective(
    snapshot: &ConstraintProgrammingSnapshot,
    target: &ObjectiveTarget,
) -> Result<()> {
    let Some(objective) = snapshot.objective.as_ref() else {
        return Ok(());
    };
    if objective.id != target.objective_id().as_str() {
        return Err(invalid_target(format!(
            "objective target references '{}', but the snapshot objective is '{}'",
            target.objective_id(),
            objective.id
        )));
    }
    Ok(())
}

/// Return the stable identity used by a temporary target constraint.
/// 返回临时 target 约束使用的稳定身份。
pub fn target_constraint_id(target: &ObjectiveTarget) -> StableConstraintId {
    StableConstraintId(format!(
        "{OBJECTIVE_TARGET_CONSTRAINT_PREFIX}{}",
        target.stable_id()
    ))
}

/// Convert an objective target to the exact integer bound used by CP.
/// 将 objective target 转成 CP 使用的精确整数边界。
pub fn exact_integer_bound(target: &ObjectiveTarget) -> Option<i64> {
    exact_integer_bound_value(target.relation(), target.value())
}

fn target_integer_satisfies(target: &ObjectiveTarget, objective: i64, bound: i64) -> bool {
    match target.relation() {
        ObjectiveTargetRelation::AtLeast => objective >= bound,
        ObjectiveTargetRelation::AtMost => objective <= bound,
    }
}

fn target_bound_matches(relation: ObjectiveTargetRelation, value: f64, bound: i64) -> bool {
    exact_integer_bound_value(relation, value).is_some_and(|expected| expected == bound)
}

fn exact_integer_bound_value(relation: ObjectiveTargetRelation, value: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    let value = match relation {
        ObjectiveTargetRelation::AtLeast => value.ceil(),
        ObjectiveTargetRelation::AtMost => value.floor(),
    };
    const MAX_EXCLUSIVE: f64 = 9_223_372_036_854_775_808.0;
    const MIN: f64 = -9_223_372_036_854_775_808.0;
    if value < MIN || value >= MAX_EXCLUSIVE {
        return None;
    }
    Some(value as i64)
}

fn assignment_from_report(
    snapshot: &ConstraintProgrammingSnapshot,
    report: &SolveReport<i64>,
) -> Option<BTreeMap<StableVariableId, i64>> {
    let solution = report.solution.as_ref()?;
    if solution.stable_values.len() == snapshot.variables.len()
        && snapshot.variables.iter().all(|variable| {
            solution
                .stable_values
                .contains_key(&variable.variable.stable_id)
        })
    {
        return Some(solution.stable_values.clone());
    }
    if solution.values.len() != snapshot.variables.len() {
        return None;
    }
    Some(
        snapshot
            .variables
            .iter()
            .zip(solution.values.iter().copied())
            .map(|(variable, value)| (variable.variable.stable_id.clone(), value))
            .collect(),
    )
}

fn target_source(target: &ObjectiveTarget) -> DiagnosticSource {
    DiagnosticSource::ObjectiveTarget {
        target: target.clone(),
    }
}

fn unsupported_report(
    target: &ObjectiveTarget,
    bound: Option<i64>,
    message: &str,
) -> TargetFeasibilityReport {
    TargetFeasibilityReport {
        schema_version: TARGET_FEASIBILITY_REPORT_SCHEMA_VERSION.to_owned(),
        target: target.clone(),
        effective_integer_bound: bound,
        source: target_source(target),
        status: AnalysisStatus::Unsupported,
        solution: None,
        exact_objective: None,
        objective_value: None,
        proof_status: ProofStatus::None,
        termination_reason: None,
        solve_time: None,
        target_fixed: false,
        message: Some(message.to_owned()),
    }
}

fn unknown_report(
    target: &ObjectiveTarget,
    bound: Option<i64>,
    solve_time: Option<Duration>,
    proof_status: Option<ProofStatus>,
    termination_reason: Option<TerminationReason>,
    message: &str,
    detail: Option<String>,
) -> TargetFeasibilityReport {
    let message = detail.map_or_else(
        || message.to_owned(),
        |detail| format!("{message}: {detail}"),
    );
    TargetFeasibilityReport {
        schema_version: TARGET_FEASIBILITY_REPORT_SCHEMA_VERSION.to_owned(),
        target: target.clone(),
        effective_integer_bound: bound,
        source: target_source(target),
        status: AnalysisStatus::Unknown,
        solution: None,
        exact_objective: None,
        objective_value: None,
        proof_status: proof_status.unwrap_or(ProofStatus::None),
        termination_reason,
        solve_time,
        target_fixed: true,
        message: Some(message),
    }
}

fn finite_i64(value: i64) -> Option<f64> {
    let value = value as f64;
    value.is_finite().then_some(value)
}

fn target_cache_key<S: ConstraintProgrammingSolver + ?Sized>(
    solver: &S,
    snapshot: &ConstraintProgrammingSnapshot,
    target: &ObjectiveTarget,
    options: &ConstraintProgrammingSolveOptions<'_>,
) -> String {
    format!(
        "{}|{}|{}|{:?}|{:?}|{:?}|{}|{}|{}",
        solver_descriptor_fingerprint(&solver.descriptor()).value,
        snapshot.fingerprint.value,
        target.stable_id(),
        options.time_limit,
        options.node_limit,
        options.solution_limit,
        options.enumeration_limit,
        options.request_conflict,
        options.max_conflict_resolves,
    )
}

fn invalid_target(message: impl Into<String>) -> CoreError {
    CoreError::Solver(SolverError::InvalidInput(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::constraint_programming::{
        ConstraintDefinition, ConstraintProgrammingModel, IntegerDomain, IntegerExpression,
        IntegerObjective, IntegerVariable,
    };
    use crate::solver::constraint_programming::FakeConstraintProgrammingSolver;

    fn snapshot(category: crate::model::ObjectiveCategory) -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("target-analysis");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 10).expect("domain"))
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "capacity",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x.clone()),
                    IntegerRelation::LessOrEqual,
                    10,
                ),
            ))
            .expect("constraint");
        model.set_objective(match category {
            crate::model::ObjectiveCategory::Maximum => {
                IntegerObjective::maximize_with_id("payload", IntegerExpression::variable(x))
            }
            crate::model::ObjectiveCategory::Minimum => {
                IntegerObjective::minimize_with_id("payload", IntegerExpression::variable(x))
            }
        });
        model.freeze().expect("snapshot")
    }

    #[test]
    fn target_directions_and_verified_infeasibility_are_preserved() {
        let solver = FakeConstraintProgrammingSolver::new();
        let max_snapshot = snapshot(crate::model::ObjectiveCategory::Maximum);
        let options = ConstraintProgrammingSolveOptions::new();
        let analyzer = TargetFeasibilityAnalyzer::new();
        let reachable = analyzer
            .analyze(
                &solver,
                &max_snapshot,
                &ObjectiveTarget::at_least("payload", 10.0).expect("target"),
                &options,
            )
            .expect("reachable");
        assert_eq!(reachable.status, AnalysisStatus::Reachable);
        assert_eq!(reachable.exact_objective, Some(10));
        let unreachable = analyzer
            .analyze(
                &solver,
                &max_snapshot,
                &ObjectiveTarget::at_least("payload", 11.0).expect("target"),
                &options,
            )
            .expect("unreachable");
        assert_eq!(unreachable.status, AnalysisStatus::Unreachable);
        assert_eq!(unreachable.proof_status, ProofStatus::Verified);
        assert!(unreachable.target_fixed);

        let minimum = snapshot(crate::model::ObjectiveCategory::Minimum);
        let at_most = analyzer
            .analyze(
                &solver,
                &minimum,
                &ObjectiveTarget::at_most("payload", 0.0).expect("target"),
                &options,
            )
            .expect("minimum target");
        assert_eq!(at_most.status, AnalysisStatus::Reachable);
        assert_eq!(at_most.exact_objective, Some(0));
    }

    #[test]
    fn fractional_targets_use_integer_ceil_or_floor_and_keep_snapshot_unchanged() {
        let solver = FakeConstraintProgrammingSolver::new();
        let max_snapshot = snapshot(crate::model::ObjectiveCategory::Maximum);
        let fingerprint = max_snapshot.fingerprint.clone();
        let count = max_snapshot.constraints.len();
        let options = ConstraintProgrammingSolveOptions::new();
        let report = TargetFeasibilityAnalyzer::new()
            .analyze(
                &solver,
                &max_snapshot,
                &ObjectiveTarget::at_least("payload", 10.5).expect("target"),
                &options,
            )
            .expect("fractional target");
        assert_eq!(report.status, AnalysisStatus::Unreachable);
        assert!(report.target_fixed);
        assert_eq!(report.effective_integer_bound, Some(11));
        let at_most = TargetFeasibilityAnalyzer::new()
            .analyze(
                &solver,
                &snapshot(crate::model::ObjectiveCategory::Minimum),
                &ObjectiveTarget::at_most("payload", 0.5).expect("target"),
                &options,
            )
            .expect("fractional at-most target");
        assert_eq!(at_most.status, AnalysisStatus::Reachable);
        assert_eq!(at_most.effective_integer_bound, Some(0));
        assert_eq!(max_snapshot.fingerprint, fingerprint);
        assert_eq!(max_snapshot.constraints.len(), count);
        assert_eq!(
            exact_integer_bound(&ObjectiveTarget::at_least("p", 10.0).unwrap()),
            Some(10)
        );
        assert_eq!(
            exact_integer_bound(&ObjectiveTarget::at_least("p", 10.5).unwrap()),
            Some(11)
        );
        assert_eq!(
            exact_integer_bound(&ObjectiveTarget::at_most("p", 10.5).unwrap()),
            Some(10)
        );
    }

    #[test]
    fn active_sparse_domain_is_preserved_when_endpoint_bounds_are_removed() {
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("sparse-domain-preservation");
        model
            .register_variable(
                x.clone(),
                IntegerDomain::values([0, 2]).expect("sparse domain"),
            )
            .expect("variable");
        model.set_objective(IntegerObjective::maximize(IntegerExpression::variable(x.clone())));
        let snapshot = model.freeze().expect("snapshot");
        let target = ObjectiveTarget::at_least("objective", 1.0).expect("target");
        let active_sources = [DiagnosticSource::SparseDomain {
            variable_id: x.stable_id.clone(),
        }];

        let derived = derive_target_snapshot(
            &snapshot,
            &target,
            exact_integer_bound(&target).expect("integer target"),
            None,
            Some(&active_sources),
        )
        .expect("derived target snapshot");

        assert_eq!(
            derived.variable(&x.stable_id).expect("derived variable").domain,
            IntegerDomain::Values(vec![0, 2]),
            "an active sparse domain must not be widened to its continuous hull"
        );
    }

    #[test]
    fn target_rejects_an_unknown_objective_identity() {
        let solver = FakeConstraintProgrammingSolver::new();
        let snapshot = snapshot(crate::model::ObjectiveCategory::Maximum);
        let options = ConstraintProgrammingSolveOptions::new();
        let result = TargetFeasibilityAnalyzer::new().analyze(
            &solver,
            &snapshot,
            &ObjectiveTarget::at_least("other-objective", 10.0).expect("target"),
            &options,
        );
        assert!(result.is_err());
        assert!(result
            .expect_err("unknown objective identity must be rejected")
            .to_string()
            .contains("snapshot objective"));
    }

    #[test]
    fn target_report_has_only_target_evidence() {
        let target = ObjectiveTarget::at_least("payload", 11.0).expect("target");
        let source = target_source(&target);
        assert_eq!(source.kind(), "objective-target");
        assert!(!source.stable_id().contains("row"));
        assert!(!source.stable_id().contains("column"));
        assert_eq!(
            target_constraint_id(&target).0,
            "analysis-target:objective-target:payload:at-least:11.0"
        );
    }
}
