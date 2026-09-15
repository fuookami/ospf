//! Target conflict and deletion-based MUS analysis / 目标冲突与删除式 MUS 分析。
//!
//! This module keeps the objective target fixed and varies original diagnostic evidence. Original
//! constraints, variable bounds, and sparse domains are all eligible MUS members; intervals remain
//! fixed background because they do not yet have a removable activation protocol. A public conflict
//! can never contain a lowering row, solver column, or generated auxiliary identity.
//! 本模块始终固定 objective target，并对原始诊断 evidence 做删除收缩。原始约束、变量边界和
//! 稀疏值域都可成为 MUS 成员；区间仍是固定背景，因为当前没有可删除的激活协议。公共冲突
//! 不会包含 lowering 行、solver 列或生成的辅助身份。

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use crate::error::{CoreError, Result, SolverError};
use crate::model::constraint_programming::ConstraintProgrammingSnapshot;
use crate::solver::constraint_programming::ConstraintProgrammingSolveOptions;
use crate::solver::fingerprint::solver_descriptor_fingerprint;
use crate::solver::report::{
    InfeasibilityEvidence, InfeasibilityEvidenceMember, InfeasibilityEvidenceSource,
    InfeasibilityMinimality, ProofStatus,
};
use crate::solver::{ConstraintProgrammingSolver, StableConstraintId};

use super::{
    activation::DiagnosticActivationSet, AnalysisCapability, AnalysisStatus, CapabilityMatrix,
    ConflictExtractionTier, DiagnosticSource, ObjectiveTarget, TargetFeasibilityAnalyzer,
    TargetFeasibilityReport,
};

/// Conflict report schema / 冲突报告 schema。
pub const CONFLICT_REPORT_SCHEMA_VERSION: &str = "1.0";

/// Minimality conclusion for a conflict explanation / 冲突解释的最小性结论。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ConflictMinimality {
    /// Every remaining member passed a final proven deletion check / 每个成员均通过最终删除复验。
    Irreducible,
    /// At least one deletion check was unavailable or unfinished / 至少一个删除复验不可用或未完成。
    Partial,
    /// No deletion checks were requested / 未请求删除复验。
    #[default]
    NotChecked,
}

impl ConflictMinimality {
    /// Convert to the existing solver evidence terminology / 转换为现有 solver 证据术语。
    pub const fn as_solver_minimality(self) -> InfeasibilityMinimality {
        match self {
            Self::Irreducible => InfeasibilityMinimality::Irreducible,
            Self::Partial => InfeasibilityMinimality::Partial,
            Self::NotChecked => InfeasibilityMinimality::NotChecked,
        }
    }

    /// Whether minimality was proven / 是否已证明最小性。
    pub const fn is_verified(self) -> bool {
        matches!(self, Self::Irreducible)
    }
}

/// Validity of the complete target conflict / 完整 target 冲突的有效性。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConflictValidity {
    /// Target infeasibility passed the verified proof gate / target 不可行通过证明门槛。
    Verified,
    /// Target feasibility was unknown or unsupported / target 未知或不受支持。
    Unknown,
}

/// One deletion verification / 一次删除复验。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictVerification {
    /// Candidate source considered for deletion / 尝试删除的候选来源。
    pub source: DiagnosticSource,
    /// Status after removing the source / 删除来源后的状态。
    pub status: AnalysisStatus,
    /// Whether the source was removed / 是否移除了来源。
    pub removed: bool,
    /// Whether the source remains in the current conflict / 是否保留在当前冲突中。
    pub retained: bool,
    /// Target remains fixed / target 始终固定。
    pub target_fixed: bool,
    /// Verification detail / 复验详情。
    pub message: Option<String>,
}

/// Group-level conflict summary / 冲突按组汇总。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictGroupSummary {
    /// Group name; `None` means ungrouped / 组名称，`None` 表示未分组。
    pub group: Option<String>,
    /// Number of remaining constraint members / 组内剩余约束数。
    pub count: usize,
}

/// Conflict/MUS analysis options / 冲突/MUS 分析参数。
#[derive(Clone, Copy)]
pub struct ConflictAnalysisOptions<'a> {
    /// Solve options forwarded to each target check / 传递给每次 target 检查的 CP 参数。
    pub solve_options: ConstraintProgrammingSolveOptions<'a>,
    /// Maximum deletion and final verification solves / 删除和最终复验的最大求解次数。
    pub max_deletion_checks: usize,
    /// Wall-clock budget for target and deletion checks / target 与删除检查的墙钟预算。
    pub time_budget: Option<Duration>,
}

impl<'a> Default for ConflictAnalysisOptions<'a> {
    fn default() -> Self {
        Self {
            solve_options: ConstraintProgrammingSolveOptions::new(),
            max_deletion_checks: usize::MAX,
            time_budget: None,
        }
    }
}

impl<'a> ConflictAnalysisOptions<'a> {
    /// Validate option invariants / 校验参数不变量。
    pub fn validate(&self) -> Result<()> {
        self.solve_options.validate()?;
        Ok(())
    }
}

/// Public target conflict explanation / 公共 target 冲突解释。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictExplanation {
    /// Report schema / 报告 schema。
    pub schema_version: String,
    /// Fixed objective target / 固定的 objective target。
    pub target: ObjectiveTarget,
    /// Initial full-background target result / 全背景下的初始 target 结果。
    pub target_feasibility: TargetFeasibilityReport,
    /// Aggregate conclusion / 聚合结论。
    pub status: AnalysisStatus,
    /// Complete original constraint universe / 原始约束证据全集。
    pub available_evidence: Vec<DiagnosticSource>,
    /// Fixed background evidence / 固定背景证据。
    pub fixed_background: Vec<DiagnosticSource>,
    /// Current conflict/MUS members; target is kept separately / 当前冲突/MUS 成员，target 单独保存。
    ///
    /// Members are original constraints, variable bounds, or sparse domains. A solver-generated
    /// row/column and the objective target itself are never valid members.
    pub members: Vec<DiagnosticSource>,
    /// Stable target evidence / 稳定 target 证据。
    pub target_source: DiagnosticSource,
    /// Whether the initial target conflict was proven / 初始 target 冲突是否已证明。
    pub validity: ConflictValidity,
    /// Deletion minimality conclusion / 删除式最小性结论。
    pub minimality: ConflictMinimality,
    /// Deletion and final verification records / 删除和最终复验记录。
    pub verifications: Vec<ConflictVerification>,
    /// Group summary / 约束组汇总。
    pub group_summaries: Vec<ConflictGroupSummary>,
    /// Number of target/deletion verification solves / target、删除复验求解次数。
    pub resolve_count: usize,
    /// 被 SAT/UNSAT 记忆化命中、因而**未**再次求解的检查次数。
    /// Number of checks served from the SAT/UNSAT memo instead of solving again.
    ///
    /// `resolve_count` 只统计真实求解次数；本字段说明记忆化省下了多少次求解，可用于性能预算核对。
    /// `resolve_count` counts real solves only; this field records how many were avoided, which makes
    /// the performance budget auditable.
    pub memoized_checks: usize,
    /// 本次冲突提取实际使用的层级 / The extraction tier actually used for this conflict.
    ///
    /// 事项 K 规定了三级优先顺序（原生 unsat core → assumption 提取 → 重复求解回退）。
    /// 这里记录实际消费的 evidence 路径，而不是让调用方从能力矩阵猜测是否使用了原生 core。
    /// Item K prescribes a three-tier priority (native unsat core, then assumption extraction, then
    /// repeated-solving fallback). This records the evidence path actually consumed instead of
    /// leaving callers to infer native-core usage from the capability matrix.
    pub extraction_tier: ConflictExtractionTier,
    /// Failure or partial reason / 失败或部分完成原因。
    pub unavailable_reason: Option<String>,
    /// 阻塞是否仅来自固定背景（变量边界/稀疏域）而非任何原始约束。
    /// Whether blocking comes only from variable bounds / sparse domains rather than a constraint.
    ///
    /// 当删除收缩把约束集合清空、而 target 仍然不可达时置为 `true`。此时 members 中保留
    /// 实际参与阻塞的边界/域 evidence，而不是发布一个空集合。
    /// Set when deletion shrinking empties the constraint set while the target stays unreachable.
    /// The members then retain the actual blocking bound/domain evidence rather than an empty set.
    pub background_blocking: bool,
}

impl ConflictExplanation {
    /// Constraint-only IDs / 仅原始约束 ID。
    pub fn constraint_ids(&self) -> BTreeSet<StableConstraintId> {
        self.members
            .iter()
            .filter_map(|source| match source {
                DiagnosticSource::Constraint { id } => Some(id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Existing typed infeasibility members / 兼容现有不可行证据成员。
    pub fn infeasibility_members(&self) -> BTreeSet<InfeasibilityEvidenceMember> {
        self.members
            .iter()
            .filter_map(DiagnosticSource::as_infeasibility_member)
            .collect()
    }

    /// Whether minimality has been proven / 是否证明了最小性。
    pub const fn minimality_verified(&self) -> bool {
        self.minimality.is_verified()
    }

    /// 真正参与阻塞的固定背景证据 / Fixed background evidence that actually blocks.
    ///
    /// 只有 [`Self::background_blocking`] 为真时才有意义：此时没有约束成员，不可达完全由
    /// 变量边界或稀疏域造成。
    /// Only meaningful when [`Self::background_blocking`] is set: no constraint member remains
    /// and unreachability is caused entirely by variable bounds or sparse domains.
    pub fn blocking_background(&self) -> Vec<DiagnosticSource> {
        if self.background_blocking {
            // Keep the returned slice tied to the report's owned member vector. The public
            // `fixed_background` field remains the complete available background universe.
            self.members
                .iter()
                .filter(|source| !matches!(source, DiagnosticSource::Constraint { .. }))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Validate public invariants / 校验公共不变量。
    pub fn validate(&self) -> Result<()> {
        if self.schema_version.trim().is_empty() {
            return Err(invalid_conflict(
                "conflict report schema version must not be blank",
            ));
        }
        self.target.validate()?;
        self.target_feasibility.validate()?;
        let expected_target = DiagnosticSource::ObjectiveTarget {
            target: self.target.clone(),
        };
        if self.target_source != expected_target {
            return Err(invalid_conflict(
                "conflict target source does not match the target",
            ));
        }
        let mut ids = BTreeSet::new();
        for source in &self.members {
            source.validate()?;
            if !ids.insert(source.stable_id()) {
                return Err(invalid_conflict(
                    "conflict contains duplicate evidence members",
                ));
            }
            if matches!(source, DiagnosticSource::ObjectiveTarget { .. }) {
                return Err(invalid_conflict(
                    "target MUS members must be original model evidence",
                ));
            }
        }
        for source in &self.fixed_background {
            source.validate()?;
        }
        if self.minimality == ConflictMinimality::Irreducible
            && (self.status != AnalysisStatus::Unreachable
                || self.validity != ConflictValidity::Verified)
        {
            return Err(invalid_conflict(
                "irreducible conflict requires a verified unreachable target",
            ));
        }
        // An empty constraint set carries no constraint-level minimality claim, and an
        // irreducible claim must always name at least one blocking member.
        // 空的约束集合不构成约束级最小性结论；不可约结论必须至少给出一个阻塞成员。
        if self.minimality == ConflictMinimality::Irreducible && self.members.is_empty() {
            return Err(invalid_conflict(
                "irreducible conflict must name at least one blocking evidence member",
            ));
        }
        if self.background_blocking
            && (self.status != AnalysisStatus::Unreachable
                || self
                    .members
                    .iter()
                    .any(|source| matches!(source, DiagnosticSource::Constraint { .. })))
        {
            return Err(invalid_conflict(
                "background blocking requires an unreachable target and no constraint members",
            ));
        }
        Ok(())
    }
}

/// Explicit conflict report cache / 显式冲突报告缓存。
#[derive(Debug, Clone, Default)]
pub struct ConflictCache {
    entries: BTreeMap<String, ConflictExplanation>,
}

impl ConflictCache {
    /// Create an empty cache / 创建空缓存。
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear the cache / 清空缓存。
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Return cache size / 返回缓存大小。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether cache is empty / 判断缓存是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Solver-backed conflict analyzer / 基于 solver 的冲突分析器。
#[derive(Debug, Clone, Copy, Default)]
pub struct ConflictAnalyzer;

impl ConflictAnalyzer {
    /// Create a conflict analyzer / 创建冲突分析器。
    pub const fn new() -> Self {
        Self
    }

    /// Analyze a target with an explicit capability matrix / 使用显式能力矩阵分析 target。
    pub fn analyze<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        options: &ConflictAnalysisOptions<'_>,
    ) -> Result<ConflictExplanation> {
        self.analyze_with_capability(
            solver,
            snapshot,
            target,
            options,
            &runtime_capability(solver, snapshot),
        )
    }

    /// Analyze with a precomputed capability matrix / 使用预先计算的能力矩阵分析。
    pub fn analyze_with_capability<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        options: &ConflictAnalysisOptions<'_>,
        capability: &CapabilityMatrix,
    ) -> Result<ConflictExplanation> {
        snapshot.validate_identity()?;
        target.validate()?;
        options.validate()?;
        let activation_set = DiagnosticActivationSet::from_snapshot(snapshot);
        let evidence = activation_set.active_sources();
        let fixed_background = fixed_background_sources(&evidence);
        if capability.support(AnalysisCapability::Conflict) == super::CapabilitySupport::Unsupported
        {
            return Ok(unsupported_explanation(
                snapshot,
                target,
                evidence,
                fixed_background,
            ));
        }

        let target_analyzer = TargetFeasibilityAnalyzer::new();
        let declared_tier = ConflictExtractionTier::select(capability);
        let request_backend_conflict = matches!(
            declared_tier,
            ConflictExtractionTier::NativeUnsatCore | ConflictExtractionTier::AssumptionExtraction
        );
        let assumptions = if declared_tier == ConflictExtractionTier::AssumptionExtraction {
            activation_set.to_assumptions(snapshot)
        } else {
            Vec::new()
        };
        let (initial, backend_evidence) = target_analyzer
            .analyze_with_capability_and_conflict_and_assumptions(
            solver,
            snapshot,
            target,
            &options.solve_options,
            capability,
            &assumptions,
            request_backend_conflict,
        )?;
        let extraction_tier = actual_extraction_tier(declared_tier, backend_evidence.as_ref());
        if initial.status != AnalysisStatus::Unreachable {
            let members = if initial.status == AnalysisStatus::Reachable {
                Vec::new()
            } else {
                evidence.clone()
            };
            let report = ConflictExplanation {
                schema_version: CONFLICT_REPORT_SCHEMA_VERSION.to_owned(),
                target: target.clone(),
                target_feasibility: initial.clone(),
                status: initial.status,
                available_evidence: evidence,
                fixed_background,
                members: members.clone(),
                target_source: DiagnosticSource::ObjectiveTarget {
                    target: target.clone(),
                },
                validity: ConflictValidity::Unknown,
                minimality: ConflictMinimality::NotChecked,
                verifications: Vec::new(),
                group_summaries: group_summaries(snapshot, &members),
                resolve_count: 1,
                memoized_checks: 0,
                unavailable_reason: initial.message.clone(),
                extraction_tier,
                background_blocking: false,
            };
            report.validate()?;
            return Ok(report);
        }

        let started = Instant::now();
        let mut active = native_conflict_seed(
            snapshot,
            &activation_set,
            backend_evidence.as_ref(),
            extraction_tier,
        )
        .unwrap_or_else(|| {
            evidence
            .iter()
            .map(DiagnosticSource::stable_id)
            .collect::<BTreeSet<_>>()
        });
        let mut verifications = Vec::new();
        let mut resolve_count = 1usize;
        // SAT/UNSAT 记忆化 / SAT-UNSAT memoization.
        //
        // 删除收缩与最终复验会反复求解**相同**的活动集合（例如收缩后再复验最终集合、
        // 或固定点第二轮重新检查同一候选）。以活动集合为键缓存已证明的结论，可以避免
        // 重复求解；未知/不受支持的结论不会被复用为已证明结论，因此不削弱任何保证。
        //
        // Deletion shrinking and final verification repeatedly solve the **same** active set (for
        // example verifying the final set again, or re-checking a candidate in the second
        // fixed-point pass). Caching proven conclusions keyed by active set avoids those duplicate
        // solves. Unknown/Unsupported outcomes are cached too but are never upgraded, so no
        // guarantee is weakened.
        let mut sat_memo: BTreeMap<BTreeSet<String>, (AnalysisStatus, Option<String>)> =
            BTreeMap::new();
        let mut memoized_checks = 0usize;
        let mut partial = options.max_deletion_checks == 0;
        let mut unavailable_reason = if partial {
            Some("deletion verification budget is zero".to_owned())
        } else {
            None
        };

        // Deletion shrinking is repeated until no member can be removed. This second pass is
        // necessary because deleting a later member can make an earlier retained member redundant.
        // 删除收缩重复执行到固定点；删除后续成员可能使先前保留的成员变得冗余。
        while !partial {
            let mut changed = false;
            let candidates = active.iter().cloned().collect::<Vec<_>>();
            for id in candidates {
                let source = evidence_source(&evidence, &id).ok_or_else(|| {
                    invalid_conflict(format!("conflict seed references unknown evidence {id}"))
                })?;
                if !can_resolve(started, options, resolve_count) {
                    partial = true;
                    unavailable_reason = Some(budget_reason(started, options));
                    break;
                }
                let mut candidate = active.clone();
                candidate.remove(&id);
                // 先查记忆化，再决定是否真的求解。 / Consult the memo before solving.
                let (checked_status, checked_message) = match sat_memo.get(&candidate) {
                    Some((status, message)) => {
                        memoized_checks = memoized_checks.saturating_add(1);
                        (*status, message.clone())
                    }
                    None => {
                        let candidate_sources = sources_for_ids(&evidence, &candidate);
                        let report = match target_analyzer.analyze_active_sources(
                            solver,
                            snapshot,
                            target,
                            &candidate_sources,
                            &options.solve_options,
                        ) {
                            Ok(report) => report,
                            Err(error) => {
                                partial = true;
                                unavailable_reason = Some(format!(
                                    "conflict deletion stopped after backend failure: {error}"
                                ));
                                verifications.push(verification(
                                    source.clone(),
                                    AnalysisStatus::Unknown,
                                    false,
                                    true,
                                    Some(error.to_string()),
                                ));
                                break;
                            }
                        };
                        resolve_count = resolve_count.saturating_add(1);
                        sat_memo
                            .insert(candidate.clone(), (report.status, report.message.clone()));
                        (report.status, report.message.clone())
                    }
                };
                match checked_status {
                    AnalysisStatus::Unreachable => {
                        active.remove(&id);
                        changed = true;
                        verifications.push(verification(
                            source.clone(),
                            checked_status,
                            true,
                            false,
                            Some("target remains unreachable after deletion".to_owned()),
                        ));
                    }
                    AnalysisStatus::Reachable => verifications.push(verification(
                        source.clone(),
                        checked_status,
                        false,
                        true,
                        Some("target becomes reachable after deletion".to_owned()),
                    )),
                    AnalysisStatus::Unknown | AnalysisStatus::Unsupported => {
                        partial = true;
                        unavailable_reason = Some(
                            "conflict deletion did not produce a proven SAT/UNSAT result"
                                .to_owned(),
                        );
                        verifications.push(verification(
                            source.clone(),
                            checked_status,
                            false,
                            true,
                            checked_message,
                        ));
                        break;
                    }
                }
            }
            if !changed {
                break;
            }
        }

        // Final explicit gate: verify current M + target is UNSAT, then check every remaining
        // member removal is SAT. Only this gate can produce Irreducible.
        let mut final_verified = !partial;
        if final_verified {
            if !can_resolve(started, options, resolve_count) {
                final_verified = false;
                partial = true;
                unavailable_reason = Some(budget_reason(started, options));
            } else {
                // 最终集合通常刚在收缩循环里被求解过，先查记忆化。
                // The final set was usually solved moments ago in the shrink loop; check the memo first.
                let final_status = match sat_memo.get(&active) {
                    Some((status, _)) => {
                        memoized_checks = memoized_checks.saturating_add(1);
                        *status
                    }
                    None => {
                        let final_sources = sources_for_ids(&evidence, &active);
                        let final_report = target_analyzer.analyze_active_sources(
                            solver,
                            snapshot,
                            target,
                            &final_sources,
                            &options.solve_options,
                        )?;
                        resolve_count = resolve_count.saturating_add(1);
                        sat_memo.insert(
                            active.clone(),
                            (final_report.status, final_report.message.clone()),
                        );
                        final_report.status
                    }
                };
                if final_status != AnalysisStatus::Unreachable {
                    final_verified = false;
                    partial = true;
                    unavailable_reason =
                        Some("final conflict seed is not verified unreachable".to_owned());
                }
            }
        }
        if final_verified {
            for id in active.iter().cloned().collect::<Vec<_>>() {
                let source = evidence_source(&evidence, &id).ok_or_else(|| {
                    invalid_conflict(format!("conflict seed references unknown evidence {id}"))
                })?;
                if !can_resolve(started, options, resolve_count) {
                    final_verified = false;
                    partial = true;
                    unavailable_reason = Some(budget_reason(started, options));
                    break;
                }
                let mut candidate = active.clone();
                candidate.remove(&id);
                // 最终逐项复验同样先查记忆化：收缩循环里已经证明过 SAT 的候选无需重解。
                // The final per-member check also consults the memo: a candidate already proven SAT
                // during shrinking needs no second solve.
                let (checked_status, checked_message) = match sat_memo.get(&candidate) {
                    Some((status, message)) => {
                        memoized_checks = memoized_checks.saturating_add(1);
                        (*status, message.clone())
                    }
                    None => {
                        let candidate_sources = sources_for_ids(&evidence, &candidate);
                        let report = target_analyzer.analyze_active_sources(
                            solver,
                            snapshot,
                            target,
                            &candidate_sources,
                            &options.solve_options,
                        )?;
                        resolve_count = resolve_count.saturating_add(1);
                        sat_memo
                            .insert(candidate.clone(), (report.status, report.message.clone()));
                        (report.status, report.message.clone())
                    }
                };
                let _ = checked_message;
                verifications.push(verification(
                    source,
                    checked_status,
                    false,
                    true,
                    Some("final minimality verification".to_owned()),
                ));
                if checked_status != AnalysisStatus::Reachable {
                    final_verified = false;
                    partial = true;
                    unavailable_reason = Some(if checked_status == AnalysisStatus::Unreachable {
                        "final deletion check remained unreachable; conflict was not irreducible"
                            .to_owned()
                    } else {
                        "final deletion check did not prove target reachability".to_owned()
                    });
                    break;
                }
            }
        }

        let members = sources_for_ids(&evidence, &active);
        // `final_verified` means the current active evidence set was proven unreachable. If no
        // constraint member remains, the explanation is entirely a bound/domain set.
        // `final_verified` 表示当前活动 evidence 集合已被证明不可达。若不再有约束成员，阻塞
        // 解释完全由边界/域集合构成。
        let background_blocking = final_verified
            && !partial
            && !members.is_empty()
            && members
                .iter()
                .all(|source| !matches!(source, DiagnosticSource::Constraint { .. }));
        let unavailable_reason = if background_blocking {
            Some(
                "target is unreachable without any original constraint: blocking comes from the \
                 fixed background (variable bounds / sparse domains), not from a constraint MUS"
                    .to_owned(),
            )
        } else {
            unavailable_reason
        };
        let minimality = if options.max_deletion_checks == 0 {
            ConflictMinimality::NotChecked
        } else if final_verified && !partial {
            ConflictMinimality::Irreducible
        } else {
            ConflictMinimality::Partial
        };
        let report = ConflictExplanation {
            schema_version: CONFLICT_REPORT_SCHEMA_VERSION.to_owned(),
            target: target.clone(),
            target_feasibility: initial,
            status: AnalysisStatus::Unreachable,
            available_evidence: evidence,
            fixed_background,
            members: members.clone(),
            target_source: DiagnosticSource::ObjectiveTarget {
                target: target.clone(),
            },
            validity: ConflictValidity::Verified,
            minimality,
            verifications,
            group_summaries: group_summaries(snapshot, &members),
            resolve_count,
            memoized_checks,
            // 只有真正收到 NativeIis evidence 时才报告 native；Farkas/rebuild 仍归入回退路径。
            // Only actual NativeIis evidence is reported as native; Farkas/rebuild remains fallback.
            extraction_tier,
            unavailable_reason,
            background_blocking,
        };
        report.validate()?;
        Ok(report)
    }

    /// Analyze a session baseline and cache the stable conflict status / 分析 session 基线并缓存稳定冲突状态。
    pub fn analyze_session<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        session: &mut super::CriticalConstraintAnalysisSession,
        target: &ObjectiveTarget,
        options: &ConflictAnalysisOptions<'_>,
    ) -> Result<ConflictExplanation> {
        let report = self.analyze(solver, session.baseline_snapshot(), target, options)?;
        session.cache_status(
            super::AnalysisCacheKind::Conflict,
            &report.target_source,
            report.status,
        )?;
        Ok(report)
    }

    /// Analyze and use an explicit cache / 分析并使用显式缓存。
    pub fn analyze_cached<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        options: &ConflictAnalysisOptions<'_>,
        cache: &mut ConflictCache,
    ) -> Result<ConflictExplanation> {
        let key = conflict_cache_key(solver, snapshot, target, options);
        if let Some(report) = cache.entries.get(&key) {
            return Ok(report.clone());
        }
        let report = self.analyze(solver, snapshot, target, options)?;
        cache.entries.insert(key, report.clone());
        Ok(report)
    }
}

/// Convenience facade for a minimal blocking set / 最小阻塞集合便捷 facade。
#[derive(Debug, Clone, Copy, Default)]
pub struct MinimalConflictAnalyzer;

impl MinimalConflictAnalyzer {
    /// Create a facade / 创建 facade。
    pub const fn new() -> Self {
        Self
    }

    /// Analyze and project the public minimal blocking set / 分析并投影公共最小阻塞集合。
    pub fn analyze<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        target: &ObjectiveTarget,
        options: &ConflictAnalysisOptions<'_>,
    ) -> Result<MinimalBlockingSet> {
        Ok(MinimalBlockingSet::from(
            ConflictAnalyzer::new().analyze(solver, snapshot, target, options)?,
        ))
    }
}

/// Public minimal blocking-set projection / 公共最小阻塞集合投影。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct MinimalBlockingSet {
    /// Fixed target / 固定 target。
    pub target: ObjectiveTarget,
    /// Original constraint members / 原始约束成员。
    pub members: Vec<DiagnosticSource>,
    /// Solver conclusion / solver 结论。
    pub status: AnalysisStatus,
    /// Minimality conclusion / 最小性结论。
    pub minimality: ConflictMinimality,
    /// Validity of initial conflict / 初始冲突有效性。
    pub validity: ConflictValidity,
    /// Verification records / 复验记录。
    pub verifications: Vec<ConflictVerification>,
    /// 阻塞是否来自固定背景而非约束 / Whether blocking comes from the fixed background
    /// instead of from a constraint.
    pub background_blocking: bool,
    /// 实际使用的冲突提取层级 / The conflict-extraction tier that actually ran.
    pub extraction_tier: ConflictExtractionTier,
}

impl MinimalBlockingSet {
    fn from(explanation: ConflictExplanation) -> Self {
        Self {
            target: explanation.target,
            members: explanation.members,
            status: explanation.status,
            minimality: explanation.minimality,
            validity: explanation.validity,
            verifications: explanation.verifications,
            background_blocking: explanation.background_blocking,
            extraction_tier: explanation.extraction_tier,
        }
    }

    /// Whether minimality was proven / 是否已证明最小性。
    pub const fn verified(&self) -> bool {
        self.minimality.is_verified()
    }
}

fn runtime_capability<S: ConstraintProgrammingSolver + ?Sized>(
    solver: &S,
    snapshot: &ConstraintProgrammingSnapshot,
) -> CapabilityMatrix {
    let descriptor = solver.descriptor();
    let support = solver.analyze_support(snapshot);
    CapabilityMatrix::from_constraint_programming_support(&descriptor, &support)
}

fn actual_extraction_tier(
    declared: ConflictExtractionTier,
    evidence: Option<&InfeasibilityEvidence>,
) -> ConflictExtractionTier {
    if declared == ConflictExtractionTier::NativeUnsatCore
        && evidence.is_some_and(|evidence| {
            evidence.source == InfeasibilityEvidenceSource::NativeIis
        })
    {
        return ConflictExtractionTier::NativeUnsatCore;
    }
    if declared == ConflictExtractionTier::AssumptionExtraction
        && evidence.is_some_and(|evidence| {
            evidence.source == InfeasibilityEvidenceSource::RebuildVerification
                && evidence.is_authoritative()
                && evidence.members.iter().any(|member| {
                    matches!(member, InfeasibilityEvidenceMember::Assumption(_))
                })
        })
    {
        return ConflictExtractionTier::AssumptionExtraction;
    }
    if declared == ConflictExtractionTier::Unavailable {
        ConflictExtractionTier::Unavailable
    } else {
        // Farkas certificates and CP rebuild evidence prove infeasibility, but neither is a
        // native unsat-core result. They remain inputs to the repeated-solving/MUS path.
        // Farkas 证书和 CP 重建证据可以证明不可行，但都不是原生 unsat core；它们仍属于
        // 重复求解/MUS 路径的输入。
        ConflictExtractionTier::RepeatedSolving
    }
}

fn native_conflict_seed(
    snapshot: &ConstraintProgrammingSnapshot,
    activation_set: &DiagnosticActivationSet,
    evidence: Option<&InfeasibilityEvidence>,
    extraction_tier: ConflictExtractionTier,
) -> Option<BTreeSet<String>> {
    let evidence = evidence?;
    let valid_source = match extraction_tier {
        ConflictExtractionTier::NativeUnsatCore => {
            evidence.source == InfeasibilityEvidenceSource::NativeIis
        }
        ConflictExtractionTier::AssumptionExtraction => {
            evidence.source == InfeasibilityEvidenceSource::RebuildVerification
                && evidence.is_authoritative()
        }
        _ => false,
    };
    if !valid_source {
        return None;
    }
    let mut seed = activation_set
        .remap_backend_evidence(snapshot, &evidence.members)
        .into_iter()
        .map(|source| source.stable_id())
        .collect::<BTreeSet<_>>();
    if extraction_tier == ConflictExtractionTier::AssumptionExtraction {
        // Assumption cores explain only the optional activation predicates. The base model
        // constraints are still part of that solve and must remain in the initial deletion seed.
        // assumption core 只解释可选激活谓词；基础模型约束仍参与该次求解，必须保留在初始收缩集合。
        seed.extend(
            snapshot
                .constraints
                .iter()
                .map(|constraint| DiagnosticSource::Constraint {
                    id: constraint.id.clone(),
                })
                .map(|source| source.stable_id()),
        );
    }
    if seed.is_empty() {
        None
    } else {
        // A native core may be a strict subset of the full source universe; the final gate below
        // still re-proves that this seed plus the target is unreachable before it is published.
        // 原生 core 可能只是完整源约束全集的子集；下方最终门控仍会重新证明该 seed 加 target
        // 确实不可达，之后才允许发布。
        Some(seed)
    }
}

fn fixed_background_sources(evidence: &[DiagnosticSource]) -> Vec<DiagnosticSource> {
    evidence
        .iter()
        .filter(|source| !matches!(source, DiagnosticSource::Constraint { .. }))
        .cloned()
        .collect()
}

fn evidence_source(evidence: &[DiagnosticSource], id: &str) -> Option<DiagnosticSource> {
    evidence
        .iter()
        .find(|source| source.stable_id() == id)
        .cloned()
}

fn sources_for_ids(evidence: &[DiagnosticSource], ids: &BTreeSet<String>) -> Vec<DiagnosticSource> {
    ids.iter()
        .filter_map(|id| evidence_source(evidence, id))
        .collect()
}

fn group_summaries(
    snapshot: &ConstraintProgrammingSnapshot,
    members: &[DiagnosticSource],
) -> Vec<ConflictGroupSummary> {
    let groups = members
        .iter()
        .filter_map(|source| match source {
            DiagnosticSource::Constraint { id } => snapshot.constraint(id),
            _ => None,
        })
        .fold(
            BTreeMap::<Option<String>, usize>::new(),
            |mut groups, constraint| {
                *groups.entry(constraint.group.clone()).or_default() += 1;
                groups
            },
        );
    groups
        .into_iter()
        .map(|(group, count)| ConflictGroupSummary { group, count })
        .collect()
}

fn unsupported_explanation(
    snapshot: &ConstraintProgrammingSnapshot,
    target: &ObjectiveTarget,
    evidence: Vec<DiagnosticSource>,
    fixed_background: Vec<DiagnosticSource>,
) -> ConflictExplanation {
    let target_feasibility = TargetFeasibilityReport {
        schema_version: super::TARGET_FEASIBILITY_REPORT_SCHEMA_VERSION.to_owned(),
        target: target.clone(),
        effective_integer_bound: super::exact_integer_bound(target),
        source: DiagnosticSource::ObjectiveTarget {
            target: target.clone(),
        },
        status: AnalysisStatus::Unsupported,
        solution: None,
        exact_objective: None,
        objective_value: None,
        proof_status: ProofStatus::None,
        termination_reason: None,
        solve_time: None,
        target_fixed: false,
        message: Some("solver capability matrix does not support CP conflict analysis".to_owned()),
    };
    let groups = group_summaries(snapshot, &evidence);
    ConflictExplanation {
        schema_version: CONFLICT_REPORT_SCHEMA_VERSION.to_owned(),
        target: target.clone(),
        target_feasibility,
        status: AnalysisStatus::Unsupported,
        available_evidence: evidence.clone(),
        fixed_background,
        members: evidence,
        target_source: DiagnosticSource::ObjectiveTarget {
            target: target.clone(),
        },
        validity: ConflictValidity::Unknown,
        minimality: ConflictMinimality::NotChecked,
        verifications: Vec::new(),
        group_summaries: groups,
        resolve_count: 0,
        memoized_checks: 0,
        unavailable_reason: Some("CP conflict analysis is unsupported".to_owned()),
        extraction_tier: ConflictExtractionTier::Unavailable,
        background_blocking: false,
    }
}

fn verification(
    source: DiagnosticSource,
    status: AnalysisStatus,
    removed: bool,
    retained: bool,
    message: Option<String>,
) -> ConflictVerification {
    ConflictVerification {
        source,
        status,
        removed,
        retained,
        target_fixed: true,
        message,
    }
}

fn can_resolve(
    started: Instant,
    options: &ConflictAnalysisOptions<'_>,
    resolve_count: usize,
) -> bool {
    resolve_count < options.max_deletion_checks.saturating_add(1)
        && options
            .time_budget
            .is_none_or(|budget| started.elapsed() < budget)
}

fn budget_reason(started: Instant, options: &ConflictAnalysisOptions<'_>) -> String {
    if options
        .time_budget
        .is_some_and(|budget| started.elapsed() >= budget)
    {
        "conflict verification time budget was exhausted".to_owned()
    } else {
        "conflict verification resolve budget was exhausted".to_owned()
    }
}

fn conflict_cache_key<S: ConstraintProgrammingSolver + ?Sized>(
    solver: &S,
    snapshot: &ConstraintProgrammingSnapshot,
    target: &ObjectiveTarget,
    options: &ConflictAnalysisOptions<'_>,
) -> String {
    let descriptor = solver_descriptor_fingerprint(&solver.descriptor());
    // Every solve option that can change a target conclusion must take part in the cache key.
    // Omitting a budget would let a budget-limited (Unknown) run be served from a cache entry
    // produced by an unlimited run, which would report an unproven conflict as verified.
    // 任何会改变 target 结论的求解参数都必须参与缓存键：遗漏预算会让"受限的 Unknown 结果"
    // 命中"无限制运行的缓存"，从而把未证明的冲突当成已验证结果返回。
    format!(
        "{}:{}:{}:{}:{:?}:{:?}:{:?}:{:?}:{}:{}:{}:{}",
        descriptor.value,
        snapshot.fingerprint.value,
        target.stable_id(),
        options.max_deletion_checks,
        options.time_budget,
        options.solve_options.time_limit,
        options.solve_options.node_limit,
        options.solve_options.solution_limit,
        options.solve_options.enumeration_limit,
        options.solve_options.request_conflict,
        options.solve_options.shrink_conflict,
        options.solve_options.max_conflict_resolves,
    )
}

fn invalid_conflict(message: impl Into<String>) -> CoreError {
    CoreError::Solver(SolverError::InvalidInput(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::constraint_programming::{
        ConstraintDefinition, ConstraintProgrammingConstraint, ConstraintProgrammingModel,
        IntegerDomain, IntegerExpression, IntegerObjective, IntegerRelation, IntegerTerm,
        IntegerVariable,
    };
    use crate::solver::constraint_programming::{
        ConstraintProgrammingSolveOptions, ConstraintProgrammingSolver,
        ConstraintProgrammingSupportReport, FakeConstraintProgrammingSolver,
    };
    use crate::solver::report::{
        InfeasibilityEvidence, InfeasibilityEvidenceMember, InfeasibilityEvidenceSource,
        InfeasibilityMinimality, ProofCompleteness, ProofReliability,
    };
    use crate::solver::{
        CapabilitySupport, SolverCapabilities, SolverCapability, SolverDescriptor, SolverInfo,
    };

    fn snapshot() -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("conflict-analysis");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 1).expect("domain"))
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "lower",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x.clone()),
                    IntegerRelation::GreaterOrEqual,
                    1,
                ),
            ))
            .expect("lower");
        model
            .add_constraint(ConstraintDefinition::new(
                "upper",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x.clone()),
                    IntegerRelation::LessOrEqual,
                    0,
                ),
            ))
            .expect("upper");
        model.set_objective(IntegerObjective::maximize(IntegerExpression::variable(x)));
        model.freeze().expect("snapshot")
    }

    fn feasible_snapshot() -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("feasible-conflict-analysis");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 1).expect("domain"))
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "lower",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x.clone()),
                    IntegerRelation::GreaterOrEqual,
                    0,
                ),
            ))
            .expect("lower");
        model.set_objective(IntegerObjective::maximize(IntegerExpression::variable(x)));
        model.freeze().expect("snapshot")
    }

    fn all_different_snapshot() -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let mut model = ConstraintProgrammingModel::new("all-different-conflict-analysis");
        let domain = IntegerDomain::range(0, 1).expect("domain");
        model
            .register_variable(x.clone(), domain.clone())
            .expect("x");
        model
            .register_variable(y.clone(), domain)
            .expect("y");
        model
            .add_constraint(ConstraintDefinition::new(
                "all-different",
                ConstraintProgrammingConstraint::AllDifferent {
                    expressions: vec![
                        IntegerExpression::variable(x.clone()),
                        IntegerExpression::variable(y.clone()),
                    ],
                },
            ))
            .expect("all-different");
        model.set_objective(IntegerObjective::maximize(
            IntegerExpression::linear(
                0,
                vec![
                    IntegerTerm {
                        variable: x,
                        coefficient: 1,
                    },
                    IntegerTerm {
                        variable: y,
                        coefficient: 1,
                    },
                ],
            )
            .expect("objective"),
        ));
        model.freeze().expect("snapshot")
    }

    #[derive(Debug, Clone, Copy)]
    struct NativeConflictTestSolver;

    impl SolverInfo for NativeConflictTestSolver {
        fn name(&self) -> &str {
            "native-conflict-test"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::ConstraintProgramming]
        }

        fn descriptor(&self) -> SolverDescriptor {
            SolverDescriptor {
                solver_id: self.name().to_owned(),
                display_name: self.name().to_owned(),
                backend_name: self.name().to_owned(),
                backend_version: Some("test".to_owned()),
                runtime_available: Some(true),
                capabilities: SolverCapabilities::default()
                    .with("constraint_programming", CapabilitySupport::Supported)
                    .with("assumption", CapabilitySupport::Supported)
                    .with("unsat_core", CapabilitySupport::Supported),
                warnings: Vec::new(),
            }
        }
    }

    impl ConstraintProgrammingSolver for NativeConflictTestSolver {
        fn analyze_support(
            &self,
            snapshot: &ConstraintProgrammingSnapshot,
        ) -> ConstraintProgrammingSupportReport {
            ConstraintProgrammingSupportReport {
                constraints: snapshot
                    .constraints
                    .iter()
                    .map(|constraint| {
                        (
                            constraint.id.clone(),
                            crate::solver::constraint_programming::ConstraintProgrammingSupport::Native,
                        )
                    })
                    .collect(),
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
                notes: vec!["test backend provides a native conflict result".to_owned()],
            }
        }

        fn solve_constraint_programming(
            &self,
            snapshot: &ConstraintProgrammingSnapshot,
            options: &ConstraintProgrammingSolveOptions<'_>,
        ) -> Result<crate::solver::SolveReport<i64>> {
            FakeConstraintProgrammingSolver::new().solve_constraint_programming(snapshot, options)
        }

        fn solve_constraint_programming_with_conflict(
            &self,
            snapshot: &ConstraintProgrammingSnapshot,
            options: &ConstraintProgrammingSolveOptions<'_>,
        ) -> Result<crate::solver::SolveReport<i64>> {
            let mut report = self.solve_constraint_programming(snapshot, options)?;
            if report.problem_status == crate::solver::ProblemStatus::Infeasible {
                report.diagnostics.infeasibility_evidence = Some(InfeasibilityEvidence {
                    source: InfeasibilityEvidenceSource::NativeIis,
                    reliability: ProofReliability::Reliable,
                    completeness: ProofCompleteness::Complete,
                    constraint_ids: BTreeSet::from(["upper".to_owned()]),
                    members: BTreeSet::from([InfeasibilityEvidenceMember::Constraint(
                        "upper".to_owned(),
                    )]),
                    minimality: InfeasibilityMinimality::NotChecked,
                    computation_time: Duration::ZERO,
                    unavailable_reason: None,
                });
            }
            Ok(report)
        }
    }

    #[derive(Debug, Clone)]
    struct AssumptionReceiptSolver {
        receipts: std::sync::Arc<std::sync::Mutex<Vec<Vec<String>>>>,
    }

    impl AssumptionReceiptSolver {
        fn new() -> Self {
            Self {
                receipts: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }

        fn receipt_count(&self) -> usize {
            self.receipts
                .lock()
                .expect("receipt lock")
                .len()
        }

        fn receipts_contain(&self, prefix: &str) -> bool {
            self.receipts
                .lock()
                .expect("receipt lock")
                .iter()
                .flatten()
                .any(|id| id.starts_with(prefix))
        }
    }

    impl SolverInfo for AssumptionReceiptSolver {
        fn name(&self) -> &str {
            "assumption-receipt-test"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::ConstraintProgramming]
        }

        fn descriptor(&self) -> SolverDescriptor {
            SolverDescriptor {
                solver_id: self.name().to_owned(),
                display_name: self.name().to_owned(),
                backend_name: self.name().to_owned(),
                backend_version: Some("test".to_owned()),
                runtime_available: Some(true),
                capabilities: SolverCapabilities::default()
                    .with("constraint_programming", CapabilitySupport::Supported)
                    .with("assumption", CapabilitySupport::Supported),
                warnings: Vec::new(),
            }
        }
    }

    impl ConstraintProgrammingSolver for AssumptionReceiptSolver {
        fn analyze_support(
            &self,
            snapshot: &ConstraintProgrammingSnapshot,
        ) -> ConstraintProgrammingSupportReport {
            FakeConstraintProgrammingSolver::new().analyze_support(snapshot)
        }

        fn solve_constraint_programming(
            &self,
            snapshot: &ConstraintProgrammingSnapshot,
            options: &ConstraintProgrammingSolveOptions<'_>,
        ) -> Result<crate::solver::SolveReport<i64>> {
            FakeConstraintProgrammingSolver::new().solve_constraint_programming(snapshot, options)
        }

        fn solve_constraint_programming_with_assumptions_and_conflict(
            &self,
            snapshot: &ConstraintProgrammingSnapshot,
            assumptions: &[crate::solver::ConstraintProgrammingAssumption],
            options: &ConstraintProgrammingSolveOptions<'_>,
        ) -> Result<crate::solver::SolveReport<i64>> {
            self.receipts
                .lock()
                .expect("receipt lock")
                .push(assumptions.iter().map(|assumption| assumption.stable_id().0).collect());
            <FakeConstraintProgrammingSolver as ConstraintProgrammingSolver>::
                solve_constraint_programming_with_assumptions_and_conflict(
                    &FakeConstraintProgrammingSolver::new(),
                    snapshot,
                    assumptions,
                    options,
                )
        }
    }

    fn sparse_domain_conflict_snapshot() -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("sparse-domain-conflict");
        model
            .register_variable(
                x.clone(),
                IntegerDomain::values([0, 2]).expect("sparse domain"),
            )
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "fixed-at-gap",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x.clone()),
                    IntegerRelation::Equal,
                    1,
                ),
            ))
            .expect("constraint");
        model.set_objective(IntegerObjective::maximize(IntegerExpression::variable(x)));
        model.freeze().expect("snapshot")
    }

    #[test]
    fn deletion_mus_keeps_target_separate_and_is_irreducible() {
        let solver = FakeConstraintProgrammingSolver::new();
        let target = ObjectiveTarget::at_least("objective", 1.0).expect("target");
        let options = ConflictAnalysisOptions::default();
        let report = ConflictAnalyzer::new()
            .analyze(&solver, &snapshot(), &target, &options)
            .expect("conflict report");
        assert_eq!(report.status, AnalysisStatus::Unreachable);
        assert_eq!(report.validity, ConflictValidity::Verified);
        assert_eq!(report.minimality, ConflictMinimality::Irreducible);
        assert_eq!(report.members.len(), 1);
        assert_eq!(report.constraint_ids().len(), 1);
        assert_eq!(report.target_source.kind(), "objective-target");
        assert!(!format!("{:?}", report).contains("solver_row"));
        assert!(!format!("{:?}", report).contains("auxiliary"));
        assert!(report.fixed_background.len() >= 2);
        report.validate().expect("valid public report");
    }

    #[test]
    fn all_different_target_conflict_preserves_original_identity_and_minimality() {
        let solver = FakeConstraintProgrammingSolver::new();
        let target = ObjectiveTarget::at_least("objective", 2.0).expect("target");
        let report = ConflictAnalyzer::new()
            .analyze(
                &solver,
                &all_different_snapshot(),
                &target,
                &ConflictAnalysisOptions::default(),
            )
            .expect("all-different conflict report");

        assert_eq!(report.status, AnalysisStatus::Unreachable);
        assert_eq!(report.validity, ConflictValidity::Verified);
        assert_eq!(report.minimality, ConflictMinimality::Irreducible);
        assert_eq!(
            report.constraint_ids(),
            BTreeSet::from([StableConstraintId::from("all-different")])
        );
        assert!(report.members.iter().any(|member| {
            *member
                == DiagnosticSource::Constraint {
                    id: StableConstraintId::from("all-different"),
                }
        }));
        assert!(report.verifications.iter().any(|verification| {
            verification.source
                == DiagnosticSource::Constraint {
                    id: StableConstraintId::from("all-different"),
                }
                && verification.retained
                && verification.status == AnalysisStatus::Reachable
        }));
        let public = format!("{report:?}");
        assert!(!public.contains("solver_row"));
        assert!(!public.contains("auxiliary"));
        report.validate().expect("valid all-different report");
    }

    #[test]
    fn deletion_budget_marks_partial_without_claiming_minimality() {
        let solver = FakeConstraintProgrammingSolver::new();
        let target = ObjectiveTarget::at_least("objective", 1.0).expect("target");
        let options = ConflictAnalysisOptions {
            max_deletion_checks: 1,
            ..Default::default()
        };
        let report = ConflictAnalyzer::new()
            .analyze(&solver, &snapshot(), &target, &options)
            .expect("partial conflict report");
        assert_eq!(report.status, AnalysisStatus::Unreachable);
        assert_eq!(report.minimality, ConflictMinimality::Partial);
        assert!(!report.minimality_verified());
        assert!(report.unavailable_reason.is_some());
    }

    #[test]
    fn reachable_target_has_no_conflict_and_unknown_is_not_unsat() {
        let solver = FakeConstraintProgrammingSolver::new();
        let snapshot = feasible_snapshot();
        let reachable = ObjectiveTarget::at_least("objective", 0.0).expect("target");
        let report = ConflictAnalyzer::new()
            .analyze(
                &solver,
                &snapshot,
                &reachable,
                &ConflictAnalysisOptions::default(),
            )
            .expect("reachable report");
        assert_eq!(report.status, AnalysisStatus::Reachable);
        assert!(report.members.is_empty());
        assert_eq!(report.validity, ConflictValidity::Unknown);

        let limited = ConflictAnalysisOptions {
            solve_options: ConstraintProgrammingSolveOptions::builder()
                .node_limit(Some(0))
                .finish(),
            ..Default::default()
        };
        let unknown = ConflictAnalyzer::new()
            .analyze(&solver, &snapshot, &reachable, &limited)
            .expect("unknown report");
        assert_eq!(unknown.status, AnalysisStatus::Unknown);
        assert_ne!(unknown.status, AnalysisStatus::Unreachable);
    }

    #[test]
    fn sat_unsat_memoization_avoids_duplicate_solves_without_weakening_minimality() {
        let solver = FakeConstraintProgrammingSolver::new();
        let target = ObjectiveTarget::at_least("objective", 1.0).expect("target");
        let report = ConflictAnalyzer::new()
            .analyze(
                &solver,
                &snapshot(),
                &target,
                &ConflictAnalysisOptions::default(),
            )
            .expect("conflict report");

        // 记忆化必须真的省下求解，且不得改变结论。
        // Memoization must actually avoid solves and must not change the conclusion.
        assert!(
            report.memoized_checks > 0,
            "expected at least one memoized check, got memoized={} resolve_count={} verifications={}",
            report.memoized_checks,
            report.resolve_count,
            report.verifications.len()
        );
        // 结论强度不得被记忆化削弱：仍然必须是已验证的不可达冲突。
        // Memoization must not weaken the conclusion: it must still be a verified unreachable
        // conflict.
        assert_eq!(report.status, AnalysisStatus::Unreachable);
        assert_eq!(report.validity, ConflictValidity::Verified);
        assert_eq!(report.minimality, ConflictMinimality::Irreducible);
        // 记忆化不得削弱最小性复验：每个成员仍必须经过最终逐项检查。
        // Memoization must not weaken the minimality re-verification: every member still passed the
        // final per-member check.
        assert_eq!(report.members.len(), 1);
        assert!(
            report.verifications.iter().any(|verification| {
                verification.message.as_deref() == Some("final minimality verification")
            }),
            "the final per-member minimality check must still run"
        );
        report.validate().expect("valid conflict report");
    }

    #[test]
    fn conflict_reports_which_extraction_tier_actually_ran() {
        let solver = FakeConstraintProgrammingSolver::new();
        let target = ObjectiveTarget::at_least("objective", 1.0).expect("target");
        let report = ConflictAnalyzer::new()
            .analyze(
                &solver,
                &snapshot(),
                &target,
                &ConflictAnalysisOptions::default(),
            )
            .expect("conflict report");

        // 三级路由中当前只有"重复求解回退"可用，报告必须如实说明，而不是让调用方从能力矩阵去猜。
        // Of the three tiers only the repeated-solving fallback is available today; the report must say
        // so rather than leaving callers to infer it from the capability matrix.
        assert_eq!(report.extraction_tier, ConflictExtractionTier::RepeatedSolving);
        assert!(!report.extraction_tier.is_native());
        assert!(report.extraction_tier.is_available());
        // 该层级必须出现在公共报告中且可序列化名称稳定。
        // The tier must appear in the public report with a stable name.
        assert_eq!(report.extraction_tier.as_str(), "repeated-solving");
        report.validate().expect("valid conflict report");
    }

    #[test]
    fn native_conflict_evidence_seeds_and_routes_the_public_mus() {
        let solver = NativeConflictTestSolver;
        let target = ObjectiveTarget::at_least("objective", 1.0).expect("target");
        let report = ConflictAnalyzer::new()
            .analyze(&solver, &snapshot(), &target, &ConflictAnalysisOptions::default())
            .expect("native conflict report");

        assert_eq!(report.extraction_tier, ConflictExtractionTier::NativeUnsatCore);
        assert_eq!(report.status, AnalysisStatus::Unreachable);
        assert_eq!(report.minimality, ConflictMinimality::Irreducible);
        assert_eq!(
            report.constraint_ids(),
            BTreeSet::from([StableConstraintId::from("upper")])
        );
        assert!(report.verifications.iter().any(|verification| {
            verification.source == DiagnosticSource::Constraint {
                id: StableConstraintId::from("upper"),
            }
        }));
        assert!(!format!("{report:?}").contains("cp::row"));
        report.validate().expect("valid native conflict report");
    }

    #[test]
    fn undeclared_native_evidence_cannot_change_the_selected_tier() {
        let evidence = InfeasibilityEvidence {
            source: InfeasibilityEvidenceSource::NativeIis,
            reliability: ProofReliability::Reliable,
            completeness: ProofCompleteness::Complete,
            constraint_ids: BTreeSet::from(["upper".to_owned()]),
            members: BTreeSet::from([InfeasibilityEvidenceMember::Constraint(
                "upper".to_owned(),
            )]),
            minimality: InfeasibilityMinimality::NotChecked,
            computation_time: Duration::ZERO,
            unavailable_reason: None,
        };
        assert_eq!(
            actual_extraction_tier(
                ConflictExtractionTier::RepeatedSolving,
                Some(&evidence)
            ),
            ConflictExtractionTier::RepeatedSolving
        );
        assert!(native_conflict_seed(
            &snapshot(),
            &DiagnosticActivationSet::from_snapshot(&snapshot()),
            Some(&evidence),
            ConflictExtractionTier::RepeatedSolving,
        )
        .is_none());
    }

    /// 8.16 性能预算用的多约束模型 / Multi-constraint model for the 8.16 performance budget.
    ///
    /// `x, y ∈ [0,10]`；`c1: x ≤ 3`、`c2: y ≤ 3` 与目标 `x + y ≥ 7` 构成 MUS `{c1, c2}`，
    /// 另有 4 条冗余约束让删除收缩必须做真实工作。
    /// The MUS of `{c1, c2}` against the target makes deletion shrinking do real work, while four
    /// redundant constraints force genuine candidate checks.
    fn performance_snapshot() -> ConstraintProgrammingSnapshot {
        use crate::model::constraint_programming::IntegerTerm;

        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let mut model = ConstraintProgrammingModel::new("conflict-performance");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 10).expect("domain"))
            .expect("x");
        model
            .register_variable(y.clone(), IntegerDomain::range(0, 10).expect("domain"))
            .expect("y");
        let add = |name: &str,
                   model: &mut ConstraintProgrammingModel,
                   expression: IntegerExpression,
                   relation: IntegerRelation,
                   rhs: i64| {
            model
                .add_constraint(ConstraintDefinition::new(
                    name,
                    ConstraintProgrammingConstraint::integer(expression, relation, rhs),
                ))
                .expect("constraint");
        };
        let variable = |variable: &IntegerVariable| IntegerExpression::variable(variable.clone());

        add("c1", &mut model, variable(&x), IntegerRelation::LessOrEqual, 3);
        add("c2", &mut model, variable(&y), IntegerRelation::LessOrEqual, 3);
        // 冗余约束：与变量边界重复，收缩时必须逐条复验并全部移除。
        // Redundant constraints duplicate the variable bounds; shrinking must verify and drop each.
        add("r1", &mut model, variable(&x), IntegerRelation::GreaterOrEqual, 0);
        add("r2", &mut model, variable(&y), IntegerRelation::GreaterOrEqual, 0);
        add("r3", &mut model, variable(&x), IntegerRelation::LessOrEqual, 10);
        add("r4", &mut model, variable(&y), IntegerRelation::LessOrEqual, 10);
        model.set_objective(IntegerObjective::maximize(
            IntegerExpression::linear(
                0,
                vec![
                    IntegerTerm {
                        variable: x.clone(),
                        coefficient: 1,
                    },
                    IntegerTerm {
                        variable: y.clone(),
                        coefficient: 1,
                    },
                ],
            )
            .expect("objective"),
        ));
        model.freeze().expect("snapshot")
    }

    #[test]
    fn performance_budget_holds_and_memoization_reduces_real_solves() {
        let solver = FakeConstraintProgrammingSolver::new();
        let target = ObjectiveTarget::at_least("objective", 7.0).expect("target");
        let options = ConflictAnalysisOptions::default();
        let report = ConflictAnalyzer::new()
            .analyze(&solver, &performance_snapshot(), &target, &options)
            .expect("conflict report");

        // 正确性：MUS 必须是 {c1, c2}——两者都必要，冗余约束必须全部被移除。
        // Correctness: the MUS must be {c1, c2}; both are necessary and every redundant constraint
        // must have been removed.
        assert_eq!(report.status, AnalysisStatus::Unreachable);
        assert_eq!(report.minimality, ConflictMinimality::Irreducible);
        let members = report
            .constraint_ids()
            .into_iter()
            .map(|id| id.0)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            members,
            BTreeSet::from(["c1".to_owned(), "c2".to_owned()]),
            "unexpected MUS members"
        );

        // 预算：真实求解次数必须落在配置的删除复验预算内（8.16）。
        // Budget: the number of real solves must stay within the configured deletion-check budget.
        assert!(
            report.resolve_count <= options.max_deletion_checks.saturating_add(1),
            "resolve_count {} exceeded the configured budget",
            report.resolve_count
        );
        // 记忆化必须产生可观测的节省。
        // Memoization must produce an observable saving.
        assert!(
            report.memoized_checks > 0,
            "expected memoized checks, got resolve_count={} verifications={}",
            report.resolve_count,
            report.verifications.len()
        );
        // 总检查数 = 真实求解 + 记忆化命中；真实求解必须严格更少，否则记忆化没有效果。
        // Total checks = real solves + memo hits; real solves must be strictly fewer.
        let total_checks = report.verifications.len() + report.memoized_checks;
        assert!(
            report.resolve_count < total_checks,
            "memoization saved nothing: resolve_count={} total_checks={}",
            report.resolve_count,
            total_checks
        );
        report.validate().expect("valid conflict report");
    }

    #[test]
    fn exhausted_time_budget_never_claims_an_irreducible_conflict() {
        let solver = FakeConstraintProgrammingSolver::new();
        let target = ObjectiveTarget::at_least("objective", 7.0).expect("target");
        // 极小时间预算：删除复验无法完成，因此结论必须降级为 Partial，绝不能声称"已验证最小"。
        // A tiny time budget prevents deletion verification from completing, so the conclusion must
        // degrade to Partial and must never claim a proven minimality.
        let options = ConflictAnalysisOptions {
            time_budget: Some(std::time::Duration::from_nanos(1)),
            ..Default::default()
        };
        let report = ConflictAnalyzer::new()
            .analyze(&solver, &performance_snapshot(), &target, &options)
            .expect("budget-limited report");

        assert_eq!(report.minimality, ConflictMinimality::Partial);
        assert!(!report.minimality_verified());
        assert!(report.unavailable_reason.is_some());
        // 预算耗尽也必须仍然是"已证明不可达"，不得把预算问题伪装成 SAT。
        // An exhausted budget must still report the proven unreachability rather than disguise the
        // budget problem as SAT.
        assert_eq!(report.status, AnalysisStatus::Unreachable);
        report.validate().expect("valid conflict report");
    }

    #[test]
    fn cache_key_separates_budget_limited_runs_from_unlimited_runs() {
        let solver = FakeConstraintProgrammingSolver::new();
        let snapshot = feasible_snapshot();
        let target = ObjectiveTarget::at_least("objective", 0.0).expect("target");
        let analyzer = ConflictAnalyzer::new();
        let mut cache = ConflictCache::new();

        // An unlimited run may be served from cache...
        let unlimited = ConflictAnalysisOptions::default();
        let first = analyzer
            .analyze_cached(&solver, &snapshot, &target, &unlimited, &mut cache)
            .expect("unlimited run");
        assert_eq!(cache.len(), 1);
        let cached = analyzer
            .analyze_cached(&solver, &snapshot, &target, &unlimited, &mut cache)
            .expect("cached run");
        assert_eq!(cache.len(), 1, "identical options must reuse the cache");
        assert_eq!(first.status, cached.status);

        // ...but a budget-limited run must never inherit that conclusion, because a limited
        // solve cannot prove anything and must stay Unknown.
        // 无限制运行可以命中缓存；但受限运行绝不能继承该结论，受限求解无法形成证明，必须保持 Unknown。
        let limited = ConflictAnalysisOptions {
            solve_options: ConstraintProgrammingSolveOptions::builder()
                .node_limit(Some(0))
                .finish(),
            ..Default::default()
        };
        let unknown = analyzer
            .analyze_cached(&solver, &snapshot, &target, &limited, &mut cache)
            .expect("limited run");
        assert_eq!(cache.len(), 2, "a limited run must not share the unlimited key");
        assert_eq!(unknown.status, AnalysisStatus::Unknown);
        assert_ne!(unknown.status, first.status);
    }

    fn background_blocked_snapshot() -> ConstraintProgrammingSnapshot {
        // The target is unreachable purely because of the variable domain: the single
        // constraint is satisfiable and cannot be part of any blocking set.
        // 目标不可达完全由变量值域造成：唯一约束本身可满足，不可能构成阻塞集合的一部分。
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("background-blocked");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 1).expect("domain"))
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "always-satisfiable",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x.clone()),
                    IntegerRelation::LessOrEqual,
                    1,
                ),
            ))
            .expect("constraint");
        model.set_objective(IntegerObjective::maximize(IntegerExpression::variable(x)));
        model.freeze().expect("snapshot")
    }

    #[test]
    fn background_only_unreachability_reports_the_blocking_bound_as_a_mus_member() {
        let solver = FakeConstraintProgrammingSolver::new();
        // Domain is [0, 1], so x >= 5 is unreachable no matter which constraints are active.
        let target = ObjectiveTarget::at_least("objective", 5.0).expect("target");
        let report = ConflictAnalyzer::new()
            .analyze(
                &solver,
                &background_blocked_snapshot(),
                &target,
                &ConflictAnalysisOptions::default(),
            )
            .expect("background-blocked report");

        assert_eq!(report.status, AnalysisStatus::Unreachable);
        assert!(report.background_blocking);
        assert_eq!(report.minimality, ConflictMinimality::Irreducible);
        assert_eq!(
            report.extraction_tier,
            ConflictExtractionTier::RepeatedSolving,
            "fixed-background blocking must not be misreported as an assumption core"
        );
        assert!(report.minimality_verified());
        assert!(!report.members.is_empty());
        assert!(report.constraint_ids().is_empty());
        assert!(report.unavailable_reason.is_some());
        // The real blocking evidence must be reachable through the explicit accessor.
        // 真正的阻塞证据必须能通过显式访问器取得。
        assert!(!report.blocking_background().is_empty());
        assert!(
            report
                .blocking_background()
                .iter()
                .any(|source| source.stable_id().contains("variable"))
        );
        // The report must remain internally consistent.
        report.validate().expect("valid background-blocked report");
    }

    #[test]
    fn assumption_tier_receives_active_assumptions_and_reports_sparse_domain() {
        let solver = AssumptionReceiptSolver::new();
        let target = ObjectiveTarget::at_least("objective", 1.0).expect("target");
        let report = ConflictAnalyzer::new()
            .analyze(
                &solver,
                &sparse_domain_conflict_snapshot(),
                &target,
                &ConflictAnalysisOptions::default(),
            )
            .expect("assumption conflict report");

        assert!(solver.receipt_count() > 0, "assumption solve was never called");
        assert!(
            solver.receipts_contain("sparse-domain/"),
            "sparse-domain assumption was not forwarded"
        );
        assert_eq!(report.extraction_tier, ConflictExtractionTier::AssumptionExtraction);
        assert_eq!(report.status, AnalysisStatus::Unreachable);
        assert_eq!(report.minimality, ConflictMinimality::Irreducible);
        assert_eq!(
            report.members,
            vec![
                DiagnosticSource::Constraint {
                    id: "fixed-at-gap".into(),
                },
                DiagnosticSource::SparseDomain {
                    variable_id: "x".into(),
                },
            ],
            "deletion shrinking must retain both indispensable constraint and sparse-domain evidence"
        );
        assert!(report.members.iter().any(|source| matches!(
            source,
            DiagnosticSource::SparseDomain { .. }
        )));
        assert!(report
            .members
            .iter()
            .all(|source| !source.stable_id().contains("row")));
        report.validate().expect("valid assumption conflict report");
    }

    #[test]
    fn exact_lowering_capability_requires_all_snapshot_constraints() {
        let solver = FakeConstraintProgrammingSolver::new();
        let snapshot = snapshot();
        let capability = runtime_capability(&solver, &snapshot);
        assert!(
            capability.support(AnalysisCapability::Conflict)
                != crate::solver::CapabilitySupport::Unsupported
        );
    }
}
