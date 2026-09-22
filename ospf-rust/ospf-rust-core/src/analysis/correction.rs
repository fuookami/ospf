//! 已验证 conflict 上的最小修正集与松弛建议协议 / Minimal correction and relaxation recommendation protocol for verified conflicts.

use std::collections::{BTreeMap, BTreeSet};

use super::{ConflictExplanation, DiagnosticSource, ObjectiveTarget};
use crate::error::Result;

/// Policy controlling which correction candidates may be recommended.
/// 控制可被推荐的 correction candidates 的策略。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct RelaxabilityPolicy {
    /// Maximum number of evidence sources in one correction set / 单个 correction set 允许的最大证据来源数。
    pub max_candidates: usize,
    /// Maximum number of ranked recommendations / 允许返回的最大排序建议数。
    pub max_plans: usize,
    /// Whether candidate weights must be strictly positive / candidate 权重是否必须严格为正。
    pub require_positive_weight: bool,
}

impl Default for RelaxabilityPolicy {
    fn default() -> Self {
        Self {
            max_candidates: 32,
            max_plans: 8,
            require_positive_weight: true,
        }
    }
}

impl RelaxabilityPolicy {
    pub fn validate(&self) -> Result<()> {
        if self.max_candidates == 0 || self.max_plans == 0 {
            return Err(super::invalid_analysis(
                "relaxability limits must be positive",
            ));
        }
        Ok(())
    }
}

/// A source that can be relaxed and its caller-provided business cost.
/// 可被放宽的证据来源及调用方提供的业务成本。
///
/// `relaxation` is a suggested amount only. The analysis layer does not apply it to a model or
/// solve the resulting model, so callers must revalidate target reachability after applying it.
/// `relaxation` 仅是建议的松弛幅度。分析层不会将其应用到模型或重新求解，因此调用方在应用
/// 该幅度后必须重新验证 target 是否可达。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct CorrectionCandidate {
    /// Original diagnostic evidence source / 原始诊断证据来源。
    pub source: DiagnosticSource,
    /// Business cost multiplier / 业务成本权重。
    pub weight: f64,
    /// Caller-provided suggested relaxation amount / 调用方提供的建议松弛幅度。
    pub relaxation: f64,
}

/// Business cost and suggested relaxation amount for one evidence source.
/// 一个证据来源的业务成本与建议松弛幅度。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelaxationCost {
    /// Cost multiplier used for deterministic plan ranking / 用于确定性方案排序的成本权重。
    pub weight: f64,
    /// Positive relaxation amount suggested for the source / 针对来源建议的正松弛幅度。
    pub relaxation: f64,
}

impl Default for RelaxationCost {
    fn default() -> Self {
        Self {
            weight: 1.0,
            relaxation: 1.0,
        }
    }
}

impl RelaxationCost {
    /// Validate a cost under the supplied policy / 按给定策略校验成本。
    pub fn validate(&self, policy: &RelaxabilityPolicy) -> Result<()> {
        policy.validate()?;
        if !self.weight.is_finite() || !self.relaxation.is_finite() || self.relaxation <= 0.0 {
            return Err(super::invalid_analysis(
                "correction weight/relaxation must be finite and positive",
            ));
        }
        if policy.require_positive_weight && self.weight <= 0.0 {
            return Err(super::invalid_analysis(
                "correction weight must be positive",
            ));
        }
        Ok(())
    }
}

impl CorrectionCandidate {
    pub fn validate(&self, policy: &RelaxabilityPolicy) -> Result<()> {
        self.source.validate()?;
        if !self.weight.is_finite() || !self.relaxation.is_finite() || self.relaxation <= 0.0 {
            return Err(super::invalid_analysis(
                "correction weight/relaxation must be finite and positive",
            ));
        }
        if policy.require_positive_weight && self.weight <= 0.0 {
            return Err(super::invalid_analysis(
                "correction weight must be positive",
            ));
        }
        Ok(())
    }
}

/// A deletion-level correction candidate or weighted relaxation recommendation.
/// 删除层面的 correction candidate 或带权松弛建议。
///
/// `minimal` records only deletion minimality for the evidence set that was actually checked. It
/// is deliberately not a claim that this set is a complete model-wide MCS: this module receives a
/// single conflict/MUS, and does not enumerate all conflicts or re-solve the full model after a
/// numeric relaxation. The numeric `relaxation` amounts are suggestions only.
/// `minimal` 只表示实际复验的 evidence 集合在删除语义下具备最小性。它有意不宣称自己是完整
/// 模型范围的 MCS：本模块只接收单个 conflict/MUS，不枚举全部冲突，也不会在数值松弛后重新
/// 求解完整模型。`relaxation` 数值仅为建议。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct CorrectionSet {
    /// Candidate evidence sources / 候选证据来源。
    pub members: Vec<CorrectionCandidate>,
    /// Weighted recommendation cost / 带权建议成本。
    pub total_cost: f64,
    /// Whether deletion minimality was verified / 是否已验证删除层面的最小性。
    pub minimal: bool,
}

impl CorrectionSet {
    /// Whether deletion minimality was verified for this candidate / 是否验证了该 candidate 的删除最小性。
    pub const fn deletion_minimality_verified(&self) -> bool {
        self.minimal
    }

    /// This protocol never proves a complete model-wide MCS / 本协议不会证明完整模型范围的 MCS。
    ///
    /// A `CorrectionSet` is built from one conflict or caller-provided candidate list. Without
    /// enumerating the complete conflict family and checking the full model, even a deletion-
    /// minimal set is only a candidate recommendation.
    /// `CorrectionSet` 来自单个 conflict 或调用方候选列表；未枚举完整 conflict 家族并复验完整
    /// 模型时，即使删除层面最小，也只能作为 candidate recommendation。
    pub const fn is_complete_mcs(&self) -> bool {
        false
    }

    /// Numeric relaxation still requires full-model revalidation / 数值松弛仍需完整模型复验。
    ///
    /// This remains true even when `minimal` is true because deletion minimality is scoped to the
    /// checked conflict/evidence set, not the full model, and does not prove sufficiency of a
    /// caller-selected numeric relaxation amount.
    /// 即使 `minimal` 为 true 也仍然返回 true，因为删除最小性只针对已复验的 conflict/evidence
    /// 集合，不覆盖完整模型，也不证明调用方选择的数值松弛幅度足以达到 target。
    pub const fn requires_revalidation(&self) -> bool {
        true
    }

    pub fn validate(&self, policy: &RelaxabilityPolicy) -> Result<()> {
        policy.validate()?;
        if self.members.is_empty() || self.members.len() > policy.max_candidates {
            return Err(super::invalid_analysis(
                "correction set size is outside policy",
            ));
        }
        let mut seen = BTreeSet::new();
        let mut cost = 0.0;
        for member in &self.members {
            member.validate(policy)?;
            if !seen.insert(member.source.stable_id()) {
                return Err(super::invalid_analysis(
                    "correction set contains duplicate evidence",
                ));
            }
            cost += member.weight * member.relaxation;
        }
        if !self.total_cost.is_finite() || (self.total_cost - cost).abs() > 1e-9 {
            return Err(super::invalid_analysis(
                "correction set total cost is inconsistent",
            ));
        }
        Ok(())
    }
}

/// A target together with one candidate improvement recommendation.
/// 一个 target 及一个候选改进建议。
///
/// The recommendation is derived from a verified deletion conflict, but applying its numeric
/// relaxation still requires caller-side model reconstruction and target revalidation.
/// 该建议源自已验证的删除 conflict，但应用数值松弛仍需调用方重建模型并重新验证 target。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct AlternativeImprovementPlan {
    /// Target associated with this recommendation / 此建议关联的 target。
    pub target: ObjectiveTarget,
    /// Candidate correction or relaxation set / 候选修正或松弛集合。
    pub correction_set: CorrectionSet,
    /// Deterministic zero-based rank / 确定性的从零开始排序名次。
    pub rank: usize,
}

impl AlternativeImprovementPlan {
    /// Whether this plan is a complete model-wide MCS / 该方案是否为完整模型范围的 MCS。
    ///
    /// The answer is always false for this solver-neutral recommendation protocol: a plan is
    /// derived from one conflict and carries no full-model revalidation result.
    /// 对本 solver-neutral recommendation 协议始终为 false：方案来自单个 conflict，且不携带
    /// 完整模型复验结果。
    pub const fn is_complete_mcs(&self) -> bool {
        self.correction_set.is_complete_mcs()
    }

    /// Numeric relaxation still requires full-model revalidation / 数值松弛仍需完整模型复验。
    pub const fn requires_revalidation(&self) -> bool {
        self.correction_set.requires_revalidation()
    }

    pub fn validate(&self, policy: &RelaxabilityPolicy) -> Result<()> {
        self.target.validate()?;
        self.correction_set.validate(policy)?;
        if self.rank >= policy.max_plans {
            return Err(super::invalid_analysis(
                "improvement plan rank exceeds policy",
            ));
        }
        Ok(())
    }
}

/// Build a deterministic weighted recommendation from candidates without solver revalidation.
/// 在不重新调用 solver 验证的前提下，从 candidates 构建确定性的带权建议。
pub fn weighted_correction_set(
    mut candidates: Vec<CorrectionCandidate>,
    policy: &RelaxabilityPolicy,
) -> Result<CorrectionSet> {
    policy.validate()?;
    for candidate in &candidates {
        candidate.validate(policy)?;
    }
    candidates.sort_by(|a, b| {
        (a.weight * a.relaxation)
            .partial_cmp(&(b.weight * b.relaxation))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.source.stable_id().cmp(&b.source.stable_id()))
    });
    candidates.truncate(policy.max_candidates);
    let total_cost = candidates.iter().map(|c| c.weight * c.relaxation).sum();
    let set = CorrectionSet {
        members: candidates,
        total_cost,
        minimal: false,
    };
    set.validate(policy)?;
    Ok(set)
}

/// Mark a caller-verified deletion-shrunk set as deletion-minimal.
/// 将调用方已验证删除收缩的集合标记为删除层面的最小集合。
///
/// The `verified` flag covers deletion minimality only; it does not validate numeric relaxation
/// amounts against the target.
/// `verified` 仅覆盖删除层面的最小性，不会针对 target 验证数值松弛幅度。
pub fn minimal_correction_set(
    candidates: Vec<CorrectionCandidate>,
    policy: &RelaxabilityPolicy,
    verified: bool,
) -> Result<CorrectionSet> {
    if !verified {
        return Err(super::invalid_analysis(
            "minimal correction set requires deletion verification",
        ));
    }
    let was_truncated = candidates.len() > policy.max_candidates;
    let mut set = weighted_correction_set(candidates, policy)?;
    // The caller's verification applies to the complete input candidate set. Once policy
    // truncation drops members, the returned set no longer has that proof and must remain a
    // candidate recommendation rather than being mislabeled as minimal.
    set.minimal = !was_truncated;
    Ok(set)
}

/// Derive deletion-only singleton correction candidates from a deletion-verified MUS.
/// 从已验证删除 MUS 派生仅支持删除语义的 singleton correction candidates。
///
/// These alternatives inherit deletion proof relative to this one MUS. They are not complete
/// model-wide MCSes, and are not proof that a numeric relaxation of the default amount is
/// sufficient to reach target reachability.
/// 这些 alternatives 继承相对于当前单个 MUS 的删除证明，但不是完整模型范围的 MCS，也不证明
/// 默认数值松弛幅度足以使 target 可达。
pub fn correction_sets_from_conflict(conflict: &ConflictExplanation) -> Vec<CorrectionSet> {
    if !verified_irreducible_conflict(conflict) {
        return Vec::new();
    }
    conflict
        .members
        .iter()
        .cloned()
        .map(|source| CorrectionSet {
            members: vec![CorrectionCandidate {
                source,
                weight: 1.0,
                relaxation: 1.0,
            }],
            total_cost: 1.0,
            minimal: true,
        })
        .collect()
}

/// Build weighted singleton correction candidates from a verified conflict.
/// 从已验证 conflict 构建带权 singleton correction candidates。
///
/// An irreducible conflict proves that deleting any one remaining member restores target
/// reachability relative to that conflict's active evidence. The conversion therefore creates one
/// deletion-minimal candidate per member; it does not claim a complete model-wide MCS, does not
/// accept an unverified or partial conflict, and does not validate caller-provided numeric
/// relaxation amounts.
/// 不可约 conflict 已证明相对于该 conflict 的活动 evidence 删除任一剩余成员即可恢复 target
/// 可达性，因此每个成员生成一个删除层面最小 candidate；这不是完整模型范围的 MCS，不接受未
/// 验证或部分 conflict，也不会在此处验证调用方提供的数值松弛幅度。
pub fn weighted_correction_sets_from_conflict(
    conflict: &ConflictExplanation,
    costs: &BTreeMap<String, RelaxationCost>,
    policy: &RelaxabilityPolicy,
) -> Result<Vec<CorrectionSet>> {
    policy.validate()?;
    if !verified_irreducible_conflict(conflict) {
        return Ok(Vec::new());
    }

    let mut sets = conflict
        .members
        .iter()
        .map(|source| {
            let cost = costs.get(&source.stable_id()).copied().unwrap_or_default();
            let candidate = CorrectionCandidate {
                source: source.clone(),
                weight: cost.weight,
                relaxation: cost.relaxation,
            };
            candidate.validate(policy)?;
            let total_cost = candidate.weight * candidate.relaxation;
            let set = CorrectionSet {
                members: vec![candidate],
                total_cost,
                minimal: true,
            };
            set.validate(policy)?;
            Ok(set)
        })
        .collect::<Result<Vec<_>>>()?;

    sets.sort_by(|left, right| {
        left.total_cost.total_cmp(&right.total_cost).then_with(|| {
            left.members[0]
                .source
                .stable_id()
                .cmp(&right.members[0].source.stable_id())
        })
    });
    sets.truncate(policy.max_plans);
    Ok(sets)
}

/// Build ranked weighted alternative-improvement plans from a verified conflict.
/// 从已验证 conflict 构建排序后的带权替代改进方案。
///
/// Returned plans are recommendations only. Applying a numeric relaxation requires caller-side
/// model reconstruction and target revalidation.
/// 返回的方案仅是建议。应用数值松弛需要调用方重建模型并重新验证 target。
pub fn alternative_improvement_plans_from_conflict(
    conflict: &ConflictExplanation,
    costs: &BTreeMap<String, RelaxationCost>,
    policy: &RelaxabilityPolicy,
) -> Result<Vec<AlternativeImprovementPlan>> {
    let sets = weighted_correction_sets_from_conflict(conflict, costs, policy)?;
    sets.into_iter()
        .enumerate()
        .map(|(rank, correction_set)| {
            let plan = AlternativeImprovementPlan {
                target: conflict.target.clone(),
                correction_set,
                rank,
            };
            plan.validate(policy)?;
            Ok(plan)
        })
        .collect()
}

/// Alias emphasizing that the result is a relaxation recommendation.
/// 强调返回松弛建议的别名入口。
///
/// This function does not apply or verify the suggested relaxation amount.
/// 此函数不会应用或验证建议的松弛幅度。
pub fn relaxation_recommendations_from_conflict(
    conflict: &ConflictExplanation,
    costs: &BTreeMap<String, RelaxationCost>,
    policy: &RelaxabilityPolicy,
) -> Result<Vec<AlternativeImprovementPlan>> {
    alternative_improvement_plans_from_conflict(conflict, costs, policy)
}

/// Whether a conflict carries all proof gates required by recommendation generation.
/// 判断 conflict 是否通过 recommendation 生成所需的全部证明门槛。
fn verified_irreducible_conflict(conflict: &ConflictExplanation) -> bool {
    conflict.status == super::AnalysisStatus::Unreachable
        && conflict.validity == super::ConflictValidity::Verified
        && conflict.minimality_verified()
        && !conflict.members.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{StableConstraintId, StableVariableId};

    #[test]
    fn weighted_set_is_sorted_and_validated() {
        let policy = RelaxabilityPolicy::default();
        let candidates = vec![
            CorrectionCandidate {
                source: DiagnosticSource::constraint(StableConstraintId("b".into())).unwrap(),
                weight: 2.0,
                relaxation: 2.0,
            },
            CorrectionCandidate {
                source: DiagnosticSource::variable_lower_bound(StableVariableId("x".into()))
                    .unwrap(),
                weight: 1.0,
                relaxation: 1.0,
            },
        ];
        let set = weighted_correction_set(candidates, &policy).unwrap();
        assert_eq!(set.members[0].source.stable_id(), "variable:x:lower");
        assert!((set.total_cost - 5.0).abs() < 1e-9);
        set.validate(&policy).unwrap();
    }

    #[test]
    fn policy_rejects_non_positive_weight() {
        let policy = RelaxabilityPolicy::default();
        let candidate = CorrectionCandidate {
            source: DiagnosticSource::constraint(StableConstraintId("c".into())).unwrap(),
            weight: 0.0,
            relaxation: 1.0,
        };
        assert!(candidate.validate(&policy).is_err());
    }

    #[test]
    fn weighted_conflict_alternatives_are_ranked_and_require_verified_mus() {
        let target = ObjectiveTarget::at_least("objective", 1.0).unwrap();
        let source_a = DiagnosticSource::constraint(StableConstraintId("a".into())).unwrap();
        let source_b = DiagnosticSource::constraint(StableConstraintId("b".into())).unwrap();
        let conflict = ConflictExplanation {
            schema_version: super::super::CONFLICT_REPORT_SCHEMA_VERSION.to_owned(),
            target: target.clone(),
            target_feasibility: unreachable_target_report(&target),
            status: super::super::AnalysisStatus::Unreachable,
            available_evidence: vec![source_a.clone(), source_b.clone()],
            fixed_background: Vec::new(),
            members: vec![source_a.clone(), source_b.clone()],
            target_source: DiagnosticSource::ObjectiveTarget { target },
            validity: super::super::ConflictValidity::Verified,
            minimality: super::super::ConflictMinimality::Irreducible,
            verifications: Vec::new(),
            group_summaries: Vec::new(),
            resolve_count: 1,
            memoized_checks: 0,
            extraction_tier: super::super::ConflictExtractionTier::RepeatedSolving,
            unavailable_reason: None,
            background_blocking: false,
        };
        conflict.validate().unwrap();

        let costs = BTreeMap::from([
            (
                source_a.stable_id(),
                RelaxationCost {
                    weight: 3.0,
                    relaxation: 1.0,
                },
            ),
            (
                source_b.stable_id(),
                RelaxationCost {
                    weight: 1.0,
                    relaxation: 2.0,
                },
            ),
        ]);
        let plans = alternative_improvement_plans_from_conflict(
            &conflict,
            &costs,
            &RelaxabilityPolicy::default(),
        )
        .unwrap();
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].rank, 0);
        assert_eq!(
            plans[0].correction_set.members[0].source.stable_id(),
            "constraint:b"
        );
        assert_eq!(
            plans[1].correction_set.members[0].source.stable_id(),
            "constraint:a"
        );
        assert!(plans
            .iter()
            .all(AlternativeImprovementPlan::requires_revalidation));
        assert!(plans
            .iter()
            .all(|plan| !plan.correction_set.members.is_empty()));
        assert!(plans.iter().all(|plan| !plan.is_complete_mcs()));
    }

    #[test]
    fn truncated_verified_candidates_are_not_marked_minimal() {
        let policy = RelaxabilityPolicy {
            max_candidates: 1,
            ..RelaxabilityPolicy::default()
        };
        let candidates = vec![
            CorrectionCandidate {
                source: DiagnosticSource::constraint("a").unwrap(),
                weight: 1.0,
                relaxation: 1.0,
            },
            CorrectionCandidate {
                source: DiagnosticSource::constraint("b").unwrap(),
                weight: 2.0,
                relaxation: 1.0,
            },
        ];

        let set = minimal_correction_set(candidates, &policy, true).unwrap();
        assert_eq!(set.members.len(), 1);
        assert!(!set.minimal);
        assert!(!set.deletion_minimality_verified());
        assert!(!set.is_complete_mcs());
        assert!(set.requires_revalidation());
    }

    #[test]
    fn partial_or_unverified_conflicts_do_not_produce_correction_candidates() {
        let partial = conflict_with(
            super::super::ConflictMinimality::Partial,
            super::super::ConflictValidity::Verified,
            super::super::AnalysisStatus::Unreachable,
        );
        assert!(correction_sets_from_conflict(&partial).is_empty());
        assert!(weighted_correction_sets_from_conflict(
            &partial,
            &BTreeMap::new(),
            &RelaxabilityPolicy::default(),
        )
        .unwrap()
        .is_empty());

        let wrong_status = conflict_with(
            super::super::ConflictMinimality::Irreducible,
            super::super::ConflictValidity::Verified,
            super::super::AnalysisStatus::Reachable,
        );
        assert!(correction_sets_from_conflict(&wrong_status).is_empty());
        assert!(alternative_improvement_plans_from_conflict(
            &wrong_status,
            &BTreeMap::new(),
            &RelaxabilityPolicy::default(),
        )
        .unwrap()
        .is_empty());
    }

    fn unreachable_target_report(
        target: &ObjectiveTarget,
    ) -> super::super::TargetFeasibilityReport {
        super::super::TargetFeasibilityReport {
            schema_version: super::super::TARGET_FEASIBILITY_REPORT_SCHEMA_VERSION.to_owned(),
            target: target.clone(),
            effective_integer_bound: Some(1),
            source: DiagnosticSource::ObjectiveTarget {
                target: target.clone(),
            },
            status: super::super::AnalysisStatus::Unreachable,
            solution: None,
            exact_objective: None,
            objective_value: None,
            proof_status: crate::solver::report::ProofStatus::Verified,
            termination_reason: Some(crate::solver::TerminationReason::Completed),
            solve_time: None,
            target_fixed: true,
            message: None,
        }
    }

    fn conflict_with(
        minimality: super::super::ConflictMinimality,
        validity: super::super::ConflictValidity,
        status: super::super::AnalysisStatus,
    ) -> super::super::ConflictExplanation {
        let target = ObjectiveTarget::at_least("objective", 1.0).unwrap();
        let source_a = DiagnosticSource::constraint("a").unwrap();
        let source_b = DiagnosticSource::constraint("b").unwrap();
        super::super::ConflictExplanation {
            schema_version: super::super::CONFLICT_REPORT_SCHEMA_VERSION.to_owned(),
            target: target.clone(),
            target_feasibility: unreachable_target_report(&target),
            status,
            available_evidence: vec![source_a.clone(), source_b.clone()],
            fixed_background: Vec::new(),
            members: vec![source_a, source_b],
            target_source: DiagnosticSource::ObjectiveTarget { target },
            validity,
            minimality,
            verifications: Vec::new(),
            group_summaries: Vec::new(),
            resolve_count: 1,
            memoized_checks: 0,
            extraction_tier: super::super::ConflictExtractionTier::RepeatedSolving,
            unavailable_reason: None,
            background_blocking: false,
        }
    }
}
