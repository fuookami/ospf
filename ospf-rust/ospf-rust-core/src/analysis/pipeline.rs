//! Candidate funnel, effectiveness ranking, and the unified analysis report.
//! 候选漏斗、有效性排序与统一分析报告。
//!
//! 本模块实现计划事项 G、Phase 3 的 `EffectivenessRanking`、事项 M 的 ConstraintGroup 聚合，
//! 以及 Phase 6 的 `CriticalConstraintAnalysisReport`。
//! This module implements plan item G, the Phase 3 `EffectivenessRanking`, the item M
//! ConstraintGroup aggregation, and the Phase 6 `CriticalConstraintAnalysisReport`.
//!
//! 贯穿本模块的两条语义红线（计划 3.5 / 7.7）：
//! 1. `inactive ≠ ineffective`、`dual = 0 ≠ globally ineffective`、
//!    `removal 无收益 ≠ 不参与组合瓶颈`，因此排序结果**不含**任何"全局无效"判决，
//!    只并列展示各层各自能证明的结论；
//! 2. Tier C 候选只被降权，**永不删除**。
//! Two semantic invariants run through this module (plan 3.5 / 7.7):
//! 1. `inactive ≠ ineffective`, `dual = 0 ≠ globally ineffective`, and
//!    "no removal improvement ≠ not part of a combined bottleneck", so a ranking never contains
//!    a "globally ineffective" verdict; it only reports what each layer actually proved;
//! 2. Tier C candidates are only deprioritised, never removed.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::Result;
use crate::model::ObjectiveCategory;
use crate::model::constraint_programming::ConstraintProgrammingSnapshot;
use crate::solver::constraint_programming::{
    ConstraintProgrammingSolveOptions, ConstraintProgrammingSolver,
};
use crate::solver::report::{ProblemStatus, SolveReport};
use crate::solver::{LinearSolver, StableVariableId};

use super::{
    ActivityStatus, AnalysisCacheKind, AnalysisCapability, AnalysisStatus, CapabilityMatrix,
    ConflictAnalysisOptions, ConflictAnalyzer, ConflictExplanation, ConstraintActivityAnalyzer,
    ConstraintActivityReport, ConstraintId,
    ConstraintPerturbationPolicy, ConstraintPerturbationReport, CriticalConstraintAnalysisSession,
    DiagnosticSource, FixedIntegerLpSensitivityAnalyzer, FixedIntegerLpSensitivityConfig,
    LocalConstraintSensitivityReport, ObjectiveTarget,
    TargetFeasibilityAnalyzer, TargetFeasibilityReport, invalid_analysis,
};

/// 统一报告 schema / Unified report schema.
pub const CRITICAL_ANALYSIS_REPORT_SCHEMA_VERSION: &str = "1.0";

/// 候选分层 / Candidate tier.
///
/// 分层只表达"做昂贵分析的优先级"，不表达约束是否重要。Tier C 依然保留在结果中。
/// The tier expresses the priority for expensive analysis only, never whether a constraint
/// matters. Tier C is still retained in the result.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum CandidateTier {
    /// active 且 |dual| 超过阈值 / Active with a dual above the threshold.
    TierA,
    /// active 但 dual 约为 0 / Active with an approximately zero dual.
    TierB,
    /// 非 active / Not active.
    TierC,
    /// 证据不足，无法分层 / Insufficient evidence to assign a tier.
    #[default]
    Unclassified,
}

impl CandidateTier {
    /// 计划中的层级名称 / Tier name used by the plan.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TierA => "A",
            Self::TierB => "B",
            Self::TierC => "C",
            Self::Unclassified => "UNCLASSIFIED",
        }
    }

    /// 该层是否默认进入昂贵分析短名单 / Whether this tier enters the expensive shortlist.
    pub const fn is_shortlist_default(self) -> bool {
        matches!(self, Self::TierA | Self::TierB)
    }
}

/// 漏斗配置 / Funnel configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandidateFunnelConfig {
    /// |dual| 超过该值才算强候选 / A dual above this value marks a strong candidate.
    pub dual_threshold: f64,
    /// 昂贵分析的默认候选上限 / Default candidate limit for expensive analysis.
    pub default_candidate_limit: usize,
}

impl Default for CandidateFunnelConfig {
    fn default() -> Self {
        Self {
            dual_threshold: 1e-9,
            default_candidate_limit: 32,
        }
    }
}

impl CandidateFunnelConfig {
    /// 校验配置 / Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        if !self.dual_threshold.is_finite() || self.dual_threshold < 0.0 {
            return Err(invalid_analysis(
                "candidate funnel dual threshold must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

/// 一个候选约束 / One candidate constraint.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintCandidate {
    /// 原始约束稳定身份 / Stable original-constraint identity.
    pub constraint_id: ConstraintId,
    /// 约束组 / Constraint group.
    pub group: Option<String>,
    /// 分层 / Tier.
    pub tier: CandidateTier,
    /// 活动性状态 / Activity status.
    pub activity: ActivityStatus,
    /// signed slack / Signed slack.
    pub slack: Option<f64>,
    /// 固定整数 LP 对偶值 / Fixed-integer LP dual value.
    pub dual_value: Option<f64>,
    /// 局部有效性 / Local effectiveness.
    pub local_effective: Option<bool>,
    /// 排序位置，0 表示优先级最高 / Rank position; 0 is the highest priority.
    pub priority: usize,
}

impl ConstraintCandidate {
    /// 原始证据来源 / Original evidence source.
    pub fn source(&self) -> DiagnosticSource {
        DiagnosticSource::Constraint {
            id: self.constraint_id.clone(),
        }
    }
}

/// 候选漏斗排序结果 / Candidate funnel ranking.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateFunnelRanking {
    /// 报告 schema / Report schema.
    pub schema_version: String,
    /// 使用的对偶阈值 / Dual threshold used.
    pub dual_threshold: f64,
    /// 全部候选，按优先级排序 / Every candidate, ordered by priority.
    pub candidates: Vec<ConstraintCandidate>,
    /// Tier A 数量 / Tier A count.
    pub tier_a: usize,
    /// Tier B 数量 / Tier B count.
    pub tier_b: usize,
    /// Tier C 数量 / Tier C count.
    pub tier_c: usize,
    /// 无法分层的数量 / Unclassified count.
    pub unclassified: usize,
}

impl CandidateFunnelRanking {
    /// 全部候选，包括 Tier C / Every candidate, Tier C included.
    pub fn candidates(&self) -> &[ConstraintCandidate] {
        &self.candidates
    }

    /// 昂贵分析短名单 / Shortlist for expensive analysis.
    ///
    /// 短名单默认包含强候选与退化候选；Tier C 及未分层候选只有在短名单仍有名额时才被纳入，
    /// 因此它们被降权而不会被永久丢弃。
    /// The shortlist prefers strong and degenerate candidates. Tier C and unclassified
    /// candidates are appended only while slots remain, so they are deprioritised rather than
    /// dropped.
    pub fn shortlist(&self, limit: usize) -> Vec<&ConstraintCandidate> {
        let mut preferred: Vec<&ConstraintCandidate> = self
            .candidates
            .iter()
            .filter(|candidate| candidate.tier.is_shortlist_default())
            .collect();
        if preferred.len() < limit {
            preferred.extend(
                self.candidates
                    .iter()
                    .filter(|candidate| !candidate.tier.is_shortlist_default())
                    .take(limit - preferred.len()),
            );
        }
        preferred.truncate(limit);
        preferred
    }

    /// 校验报告不变量 / Validate report invariants.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version.trim().is_empty()
            || !self.dual_threshold.is_finite()
            || self.dual_threshold < 0.0
        {
            return Err(invalid_analysis("candidate funnel ranking is invalid"));
        }
        let mut seen = BTreeSet::new();
        let mut counts = (0usize, 0usize, 0usize, 0usize);
        for (position, candidate) in self.candidates.iter().enumerate() {
            if candidate.constraint_id.0.trim().is_empty() || candidate.priority != position {
                return Err(invalid_analysis(
                    "candidate funnel priorities must be dense and match stable identities",
                ));
            }
            if !seen.insert(candidate.constraint_id.clone()) {
                return Err(invalid_analysis(
                    "candidate funnel contains duplicate constraints",
                ));
            }
            match candidate.tier {
                CandidateTier::TierA => counts.0 += 1,
                CandidateTier::TierB => counts.1 += 1,
                CandidateTier::TierC => counts.2 += 1,
                CandidateTier::Unclassified => counts.3 += 1,
            }
        }
        if counts != (self.tier_a, self.tier_b, self.tier_c, self.unclassified) {
            return Err(invalid_analysis(
                "candidate funnel tier counts do not match the candidate list",
            ));
        }
        // Tier A/B require a usable dual; Tier A requires it to exceed the threshold.
        for candidate in &self.candidates {
            match candidate.tier {
                CandidateTier::TierA => {
                    let dual = candidate.dual_value.ok_or_else(|| {
                        invalid_analysis("tier A candidate requires a dual value")
                    })?;
                    if !(dual.abs() > self.dual_threshold) {
                        return Err(invalid_analysis(
                            "tier A candidate requires a dual above the threshold",
                        ));
                    }
                }
                CandidateTier::TierB => {
                    let dual = candidate.dual_value.ok_or_else(|| {
                        invalid_analysis("tier B candidate requires a dual value")
                    })?;
                    if dual.abs() > self.dual_threshold {
                        return Err(invalid_analysis(
                            "tier B candidate requires a dual at or below the threshold",
                        ));
                    }
                }
                CandidateTier::TierC => {
                    if candidate.activity.is_active() {
                        return Err(invalid_analysis(
                            "tier C candidate must not be active",
                        ));
                    }
                }
                CandidateTier::Unclassified => {}
            }
        }
        Ok(())
    }
}

/// 分层一条候选 / Classify one candidate.
///
/// 只有同时具备活动性与对偶证据时才能给出分层；否则返回 [`CandidateTier::Unclassified`]，
/// 以免把"未知"当成"不重要"。
/// A tier requires both activity and dual evidence; otherwise the candidate is
/// [`CandidateTier::Unclassified`], so "unknown" can never be read as "unimportant".
pub fn classify_candidate(
    activity: ActivityStatus,
    dual_value: Option<f64>,
    dual_threshold: f64,
) -> CandidateTier {
    if activity.is_active() || activity.is_nearly_active() {
        return match dual_value {
            Some(dual) if dual.abs() > dual_threshold => CandidateTier::TierA,
            Some(_) => CandidateTier::TierB,
            None => CandidateTier::Unclassified,
        };
    }
    if matches!(
        activity,
        ActivityStatus::Inactive | ActivityStatus::SatisfiedWithoutSlackMetric
    ) {
        return CandidateTier::TierC;
    }
    CandidateTier::Unclassified
}

/// 构建候选漏斗 / Build the candidate funnel.
///
/// `sensitivity` 为 `None` 时，所有候选都只能停在 `Unclassified`：没有对偶证据就不允许
/// 声称分层。Tier C 依然按活动性给出，并保留在结果中。
/// When `sensitivity` is `None` every candidate stays `Unclassified`: without dual evidence no
/// tier may be claimed. Tier C is still derived from activity and retained in the result.
pub fn build_candidate_funnel(
    activity: &ConstraintActivityReport,
    sensitivity: Option<&LocalConstraintSensitivityReport>,
    config: &CandidateFunnelConfig,
) -> Result<CandidateFunnelRanking> {
    config.validate()?;
    activity.validate()?;
    if let Some(report) = sensitivity {
        report.validate()?;
    }
    let duals: BTreeMap<ConstraintId, Option<f64>> = sensitivity
        .map(|report| {
            report
                .constraints
                .iter()
                .map(|entry| (entry.constraint_id.clone(), entry.dual_value))
                .collect()
        })
        .unwrap_or_default();

    let mut candidates: Vec<ConstraintCandidate> = activity
        .constraints
        .iter()
        .map(|record| {
            let dual = duals.get(&record.constraint_id).copied().flatten();
            let tier = classify_candidate(record.status, dual, config.dual_threshold);
            ConstraintCandidate {
                constraint_id: record.constraint_id.clone(),
                group: record.group.clone(),
                tier,
                activity: record.status,
                slack: record.slack,
                dual_value: dual,
                local_effective: dual.map(|value| value.abs() > config.dual_threshold),
                priority: 0,
            }
        })
        .collect();

    // Priority: tier order first, then larger |dual|, then stable identity for determinism.
    // 优先级：先按层级，再按 |dual| 降序，最后按稳定身份保证确定性。
    candidates.sort_by(|left, right| {
        left.tier
            .cmp(&right.tier)
            .then_with(|| {
                let left_dual = left.dual_value.map(f64::abs).unwrap_or(0.0);
                let right_dual = right.dual_value.map(f64::abs).unwrap_or(0.0);
                right_dual
                    .partial_cmp(&left_dual)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.constraint_id.0.cmp(&right.constraint_id.0))
    });
    for (position, candidate) in candidates.iter_mut().enumerate() {
        candidate.priority = position;
    }

    let count = |tier: CandidateTier| {
        candidates
            .iter()
            .filter(|candidate| candidate.tier == tier)
            .count()
    };
    let ranking = CandidateFunnelRanking {
        schema_version: CRITICAL_ANALYSIS_REPORT_SCHEMA_VERSION.to_owned(),
        dual_threshold: config.dual_threshold,
        tier_a: count(CandidateTier::TierA),
        tier_b: count(CandidateTier::TierB),
        tier_c: count(CandidateTier::TierC),
        unclassified: count(CandidateTier::Unclassified),
        candidates,
    };
    ranking.validate()?;
    Ok(ranking)
}

/// 一条约束的有效性证据 / Effectiveness evidence for one constraint.
///
/// 各层结论并列保存，绝不互相覆盖，也不合成单一的"是否临界"判决。
/// Each layer's conclusion is stored side by side; they never overwrite one another and are
/// never collapsed into a single "is critical" verdict.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct EffectivenessEntry {
    /// 原始约束稳定身份 / Stable original-constraint identity.
    pub constraint_id: ConstraintId,
    /// 约束组 / Constraint group.
    pub group: Option<String>,
    /// 漏斗分层 / Funnel tier.
    pub tier: CandidateTier,
    /// 活动性状态 / Activity status.
    pub activity: ActivityStatus,
    /// 固定整数结构下的局部对偶值 / Local dual under the fixed integer pattern.
    pub dual_value: Option<f64>,
    /// 局部有效性 / Local effectiveness.
    pub local_effective: Option<bool>,
    /// 单删是否改善目标 / Whether single removal improved the objective.
    pub removal_effective: Option<bool>,
    /// 首次观察到改善的 delta / First delta that produced an improvement.
    pub first_effective_delta: Option<f64>,
    /// 已证明的最大目标改善 / Largest proven objective improvement.
    pub max_improvement: Option<f64>,
    /// 每单位 delta 的边际效应 / Marginal effect per unit of delta.
    pub marginal_effect: Option<f64>,
    /// 该约束的分析状态 / Analysis status for this constraint.
    pub status: AnalysisStatus,
    /// 未形成结论的原因 / Why no conclusion was formed.
    pub unavailable_reason: Option<String>,
}

impl EffectivenessEntry {
    /// 是否观察到全局改善 / Whether a global improvement was observed.
    pub fn is_globally_effective(&self) -> bool {
        self.max_improvement.is_some_and(|value| value > 0.0)
    }

    /// 原始证据来源 / Original evidence source.
    pub fn source(&self) -> DiagnosticSource {
        DiagnosticSource::Constraint {
            id: self.constraint_id.clone(),
        }
    }
}

/// 有效性排序 / Effectiveness ranking.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct EffectivenessRanking {
    /// 报告 schema / Report schema.
    pub schema_version: String,
    /// baseline 目标值 / Baseline objective value.
    pub baseline_objective: f64,
    /// 按已证明的最大改善降序排列 / Entries ordered by proven maximum improvement.
    pub entries: Vec<EffectivenessEntry>,
}

impl EffectivenessRanking {
    /// 观察到全局改善的约束 / Constraints with an observed global improvement.
    ///
    /// 这只是"已证明有收益"的子集；不在其中的约束**不**表示无关，因为单独移除无收益也可能是
    /// 组合瓶颈的一部分（计划 8.7）。
    /// This is only the subset with proven benefit. Absence from it does **not** mean
    /// irrelevance: a constraint whose single removal shows no benefit can still be part of a
    /// combined bottleneck (plan 8.7).
    pub fn globally_effective(&self) -> Vec<&EffectivenessEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.is_globally_effective())
            .collect()
    }

    /// 局部有效的约束 / Constraints that are locally effective.
    pub fn locally_effective(&self) -> Vec<&EffectivenessEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.local_effective == Some(true))
            .collect()
    }

    /// 校验报告不变量 / Validate report invariants.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version.trim().is_empty() || !self.baseline_objective.is_finite() {
            return Err(invalid_analysis("effectiveness ranking is invalid"));
        }
        let mut seen = BTreeSet::new();
        for entry in &self.entries {
            if entry.constraint_id.0.trim().is_empty() {
                return Err(invalid_analysis(
                    "effectiveness entry requires a stable constraint identity",
                ));
            }
            if !seen.insert(entry.constraint_id.clone()) {
                return Err(invalid_analysis(
                    "effectiveness ranking contains duplicate constraints",
                ));
            }
            if entry
                .dual_value
                .is_some_and(|value| !value.is_finite())
                || entry
                    .max_improvement
                    .is_some_and(|value| !value.is_finite())
                || entry
                    .marginal_effect
                    .is_some_and(|value| !value.is_finite())
                || entry
                    .first_effective_delta
                    .is_some_and(|value| !value.is_finite() || value <= 0.0)
            {
                return Err(invalid_analysis(
                    "effectiveness entry contains a non-finite or non-positive value",
                ));
            }
            // A local verdict may only exist when a dual was actually available.
            if entry.local_effective.is_some() && entry.dual_value.is_none() {
                return Err(invalid_analysis(
                    "local effectiveness requires an available dual value",
                ));
            }
            // A removal verdict may only exist when the removal test ran.
            if entry.removal_effective.is_some() && entry.status == AnalysisStatus::Unsupported {
                return Err(invalid_analysis(
                    "removal effectiveness requires a supported removal test",
                ));
            }
        }
        Ok(())
    }
}

/// 从扰动报告构建有效性排序 / Build an effectiveness ranking from perturbation reports.
pub fn build_effectiveness_ranking(
    baseline_objective: f64,
    funnel: Option<&CandidateFunnelRanking>,
    perturbation_reports: &[ConstraintPerturbationReport],
) -> Result<EffectivenessRanking> {
    if !baseline_objective.is_finite() {
        return Err(invalid_analysis("baseline objective must be finite"));
    }
    let funnel_by_id: BTreeMap<&ConstraintId, &ConstraintCandidate> = funnel
        .map(|ranking| {
            ranking
                .candidates
                .iter()
                .map(|candidate| (&candidate.constraint_id, candidate))
                .collect()
        })
        .unwrap_or_default();

    let mut entries = Vec::with_capacity(perturbation_reports.len());
    for report in perturbation_reports {
        report.validate()?;
        let candidate = funnel_by_id.get(&report.constraint_id).copied();
        // Only proven observations may feed the ranking; an Unknown solve never becomes an
        // improvement, and a non-proven observation is never counted.
        // 只有已证明的观测才能进入排序；Unknown 求解绝不转化为改善，未证明的观测不计入。
        let proven: Vec<&super::ConstraintPerturbationObservation> = report
            .observations
            .iter()
            .filter(|observation| observation.objective_proven)
            .collect();
        let max_improvement = proven
            .iter()
            .filter_map(|observation| observation.improvement)
            .filter(|value| value.is_finite())
            .fold(None::<f64>, |best, value| {
                Some(best.map_or(value, |current| current.max(value)))
            });
        let first_effective_delta = proven
            .iter()
            .filter(|observation| observation.improved == Some(true))
            .map(|observation| observation.delta)
            .fold(None::<f64>, |best, delta| {
                Some(best.map_or(delta, |current| current.min(delta)))
            });
        let marginal_effect = match (max_improvement, first_effective_delta) {
            (Some(improvement), Some(delta)) if delta > 0.0 => Some(improvement / delta),
            _ => None,
        };
        entries.push(EffectivenessEntry {
            constraint_id: report.constraint_id.clone(),
            group: candidate.and_then(|candidate| candidate.group.clone()),
            tier: candidate
                .map(|candidate| candidate.tier)
                .unwrap_or(CandidateTier::Unclassified),
            activity: candidate
                .map(|candidate| candidate.activity)
                .unwrap_or(ActivityStatus::Unknown),
            dual_value: candidate.and_then(|candidate| candidate.dual_value),
            local_effective: candidate.and_then(|candidate| candidate.local_effective),
            removal_effective: report.removal.as_ref().and_then(|removal| removal.effective),
            first_effective_delta,
            max_improvement,
            marginal_effect,
            status: report.status,
            unavailable_reason: report.unavailable_reason.clone(),
        });
    }

    // Order by proven improvement, then by |dual|, then by stable identity.
    // 排序：已证明改善降序，其次 |dual| 降序，最后稳定身份。
    entries.sort_by(|left, right| {
        let left_gain = left.max_improvement.unwrap_or(f64::NEG_INFINITY);
        let right_gain = right.max_improvement.unwrap_or(f64::NEG_INFINITY);
        right_gain
            .partial_cmp(&left_gain)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let left_dual = left.dual_value.map(f64::abs).unwrap_or(0.0);
                let right_dual = right.dual_value.map(f64::abs).unwrap_or(0.0);
                right_dual
                    .partial_cmp(&left_dual)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.constraint_id.0.cmp(&right.constraint_id.0))
    });

    let ranking = EffectivenessRanking {
        schema_version: CRITICAL_ANALYSIS_REPORT_SCHEMA_VERSION.to_owned(),
        baseline_objective,
        entries,
    };
    ranking.validate()?;
    Ok(ranking)
}

/// 业务级约束组汇总 / Business-level constraint-group summary.
///
/// 对应计划事项 M：默认展示组级统计，并支持下钻到实例。
/// Corresponds to plan item M: group-level statistics by default, with drill-down to instances.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisGroupSummary {
    /// 组名称；`None` 表示未分组 / Group name; `None` means ungrouped.
    pub group: Option<String>,
    /// 该组涉及的约束实例数 / Number of involved constraint instances.
    pub involved_instances: usize,
    /// 其中处于 active 的实例数 / Instances that are active.
    pub active_instances: usize,
    /// 其中进入当前 MUS 的实例数 / Instances in the current MUS.
    pub blocking_instances: usize,
    /// 组内观察到的已证明最大改善 / Largest proven improvement observed in this group.
    pub max_improvement: Option<f64>,
}

/// 跨阶段汇总约束组 / Aggregate constraint groups across stages.
pub fn summarize_groups(
    activity: Option<&ConstraintActivityReport>,
    effectiveness: Option<&EffectivenessRanking>,
    conflict: Option<&ConflictExplanation>,
) -> Vec<AnalysisGroupSummary> {
    let mut groups: BTreeMap<Option<String>, AnalysisGroupSummary> = BTreeMap::new();
    // Materialise every referenced group first, then mutate by key. This keeps the borrow
    // checker satisfied without giving up the deterministic `BTreeMap` ordering.
    // 先物化所有被引用的组，再按键修改；既满足借用检查，又保留 `BTreeMap` 的确定性顺序。
    let mut ensure = |groups: &mut BTreeMap<Option<String>, AnalysisGroupSummary>,
                      group: Option<String>| {
        groups
            .entry(group.clone())
            .or_insert_with(|| AnalysisGroupSummary {
                group,
                involved_instances: 0,
                active_instances: 0,
                blocking_instances: 0,
                max_improvement: None,
            });
    };

    if let Some(report) = activity {
        for record in &report.constraints {
            ensure(&mut groups, record.group.clone());
            if let Some(summary) = groups.get_mut(&record.group) {
                summary.involved_instances += 1;
                if record.status.is_active() {
                    summary.active_instances += 1;
                }
            }
        }
    }
    if let Some(ranking) = effectiveness {
        for record in &ranking.entries {
            if let Some(improvement) = record.max_improvement {
                ensure(&mut groups, record.group.clone());
                if let Some(summary) = groups.get_mut(&record.group) {
                    summary.max_improvement = Some(
                        summary
                            .max_improvement
                            .map_or(improvement, |current| current.max(improvement)),
                    );
                }
            }
        }
    }
    if let Some(explanation) = conflict {
        // Group membership of a blocking constraint comes from the activity report, because the
        // conflict report deliberately stores only stable identities.
        // 阻塞约束的组归属取自活动性报告，因为冲突报告刻意只保存稳定身份。
        let group_of: BTreeMap<ConstraintId, Option<String>> = activity
            .map(|report| {
                report
                    .constraints
                    .iter()
                    .map(|record| (record.constraint_id.clone(), record.group.clone()))
                    .collect()
            })
            .unwrap_or_default();
        for id in explanation.constraint_ids() {
            let group = group_of.get(&id).cloned().flatten();
            ensure(&mut groups, group.clone());
            if let Some(summary) = groups.get_mut(&group) {
                summary.blocking_instances += 1;
            }
        }
    }

    let mut summaries: Vec<AnalysisGroupSummary> = groups.into_values().collect();
    summaries.sort_by(|left, right| {
        right
            .blocking_instances
            .cmp(&left.blocking_instances)
            .then_with(|| right.active_instances.cmp(&left.active_instances))
            .then_with(|| left.group.cmp(&right.group))
    });
    summaries
}

/// 基线摘要 / Baseline summary.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisBaselineSummary {
    /// 目标身份 / Objective identity.
    pub objective_id: String,
    /// 优化方向 / Optimization direction.
    pub sense: String,
    /// 已证明的基线目标值；未证明最优时为空 / Proven baseline objective; absent when not proven.
    pub optimal_value: Option<f64>,
    /// 求解结论 / Solve conclusion.
    pub status: AnalysisStatus,
    /// 求解是否是已证明最优 / Whether the solve proved optimality.
    pub proven_optimal: bool,
}

/// Phase 6 统一报告 / Phase 6 unified report.
///
/// 该报告把 baseline → activity → fixed-integer LP → perturbation → target → conflict → MUS
/// 各阶段结果聚合到一个公共结构中。所有字段只承载原始模型证据；任何阶段缺失都显式表示为
/// `None` 或 `Unsupported`，绝不推断为"无效"。
/// This report aggregates baseline → activity → fixed-integer LP → perturbation → target →
/// conflict → MUS into one public structure. Every field carries original-model evidence only;
/// a missing stage is expressed as `None` or `Unsupported` and is never inferred to mean
/// "ineffective".
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct CriticalConstraintAnalysisReport {
    /// 报告 schema / Report schema.
    pub schema_version: String,
    /// 基线 / Baseline.
    pub baseline: AnalysisBaselineSummary,
    /// 活动性 / Activity.
    pub activity: Option<ConstraintActivityReport>,
    /// 固定整数 LP 局部有效性 / Fixed-integer LP local sensitivity.
    pub local_sensitivity: Option<LocalConstraintSensitivityReport>,
    /// 候选漏斗 / Candidate funnel.
    pub funnel: Option<CandidateFunnelRanking>,
    /// 有效性排序 / Effectiveness ranking.
    pub effectiveness: Option<EffectivenessRanking>,
    /// 目标可行性 / Target feasibility.
    pub target: Option<TargetFeasibilityReport>,
    /// 冲突解释 / Conflict explanation.
    pub conflict: Option<ConflictExplanation>,
    /// 按约束组聚合的业务级汇总 / Business-level group aggregation.
    pub group_summaries: Vec<AnalysisGroupSummary>,
    /// 整条链路的总体结论 / Overall conclusion for the whole chain.
    pub status: AnalysisStatus,
    /// 未完成阶段的原因 / Reasons stages did not complete.
    pub unavailable_reasons: Vec<String>,
}

impl CriticalConstraintAnalysisReport {
    /// 校验报告不变量 / Validate report invariants.
    ///
    /// 这里强制两条计划红线：报告不得包含 solver 生成的辅助证据；某个阶段缺失或未形成结论
    /// 时，总体结论不得升格为已证明。
    /// Two plan invariants are enforced here: the report must not carry solver-generated
    /// auxiliary evidence, and the overall conclusion must not be upgraded to proven while any
    /// stage is missing or unproven.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version.trim().is_empty() {
            return Err(invalid_analysis("analysis report schema version must not be blank"));
        }
        if self.baseline.objective_id.trim().is_empty() || self.baseline.sense.trim().is_empty() {
            return Err(invalid_analysis(
                "analysis report baseline requires an objective identity and sense",
            ));
        }
        if self.baseline.proven_optimal && self.baseline.optimal_value.is_none() {
            return Err(invalid_analysis(
                "a proven baseline requires an objective value",
            ));
        }
        if let Some(activity) = &self.activity {
            activity.validate()?;
        }
        if let Some(report) = &self.local_sensitivity {
            report.validate()?;
        }
        if let Some(funnel) = &self.funnel {
            funnel.validate()?;
        }
        if let Some(ranking) = &self.effectiveness {
            ranking.validate()?;
        }
        if let Some(target) = &self.target {
            target.validate()?;
        }
        if let Some(explanation) = &self.conflict {
            explanation.validate()?;
        }
        // The public report must never mention a solver artifact kind.
        // 公开报告绝不能出现 solver 生成物的痕迹。
        for forbidden in ["solver_row", "solver_column", "auxiliary_variable", "native_handle"] {
            let mention = format!("{self:?}");
            if mention.contains(forbidden) {
                return Err(invalid_analysis(format!(
                    "analysis report must not expose solver-generated evidence: {forbidden}"
                )));
            }
        }
        Ok(())
    }

    /// 是否存在已证明的最小阻塞集合 / Whether a proven minimal blocking set exists.
    pub fn has_minimal_blocking_set(&self) -> bool {
        self.conflict
            .as_ref()
            .is_some_and(|explanation| explanation.minimality_verified())
    }

    /// 业务可读的阻塞摘要 / Business-readable blocking summary.
    ///
    /// 当阻塞来自固定背景时明确说明，避免把空约束集合读成"没有阻塞约束"。
    /// When blocking comes from the fixed background this is stated explicitly, so an empty
    /// constraint set is never read as "no blocking constraints".
    pub fn blocking_summary(&self) -> String {
        match &self.conflict {
            None => "阻塞分析未执行 / Blocking analysis was not run".to_owned(),
            Some(explanation) if explanation.background_blocking => format!(
                "目标不可达由固定背景造成（变量边界/稀疏域 {} 项），不存在约束级最小阻塞集合 / \
                 Target is unreachable because of the fixed background ({} bound/domain items); \
                 no constraint-level minimal blocking set exists",
                explanation.fixed_background.len(),
                explanation.fixed_background.len()
            ),
            Some(explanation) => {
                let groups = explanation
                    .group_summaries
                    .iter()
                    .map(|summary| {
                        format!(
                            "{}: {}",
                            summary.group.as_deref().unwrap_or("<ungrouped>"),
                            summary.count
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let minimality = if explanation.minimality_verified() {
                    "已验证最小"
                } else {
                    "未证明最小"
                };
                format!(
                    "最小阻塞集合（{minimality}）：{} 个原始约束 [{groups}] / \
                     Minimal blocking set ({minimality}): {} original constraints [{groups}]",
                    explanation.members.len(),
                    explanation.members.len()
                )
            }
        }
    }
}

/// 统一报告构建器 / Unified report builder.
///
/// 各阶段由调用方执行并传入；构建器只负责聚合、派生 group 汇总与总体状态，因此它不依赖
/// 任何 backend，也不会隐式触发求解。
/// Each stage is executed by the caller and passed in; the builder only aggregates, derives
/// group summaries, and computes the overall status, so it depends on no backend and never
/// triggers a solve implicitly.
#[derive(Debug, Clone, Default)]
pub struct CriticalConstraintAnalysisReportBuilder {
    baseline: Option<AnalysisBaselineSummary>,
    activity: Option<ConstraintActivityReport>,
    local_sensitivity: Option<LocalConstraintSensitivityReport>,
    perturbation: Vec<ConstraintPerturbationReport>,
    target: Option<TargetFeasibilityReport>,
    conflict: Option<ConflictExplanation>,
    unavailable_reasons: Vec<String>,
    has_unsupported_stage: bool,
}

impl CriticalConstraintAnalysisReportBuilder {
    /// 创建空构建器 / Create an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置基线 / Set the baseline.
    pub fn with_baseline(mut self, baseline: AnalysisBaselineSummary) -> Self {
        self.baseline = Some(baseline);
        self
    }

    /// 设置活动性报告 / Set the activity report.
    pub fn with_activity(mut self, activity: ConstraintActivityReport) -> Self {
        self.activity = Some(activity);
        self
    }

    /// 设置局部有效性报告 / Set the local sensitivity report.
    pub fn with_local_sensitivity(
        mut self,
        report: LocalConstraintSensitivityReport,
    ) -> Self {
        self.local_sensitivity = Some(report);
        self
    }

    /// 追加扰动报告 / Append perturbation reports.
    pub fn with_perturbation(mut self, reports: Vec<ConstraintPerturbationReport>) -> Self {
        self.perturbation = reports;
        self
    }

    /// 设置目标可行性报告 / Set the target-feasibility report.
    pub fn with_target(mut self, target: TargetFeasibilityReport) -> Self {
        self.target = Some(target);
        self
    }

    /// 设置冲突解释 / Set the conflict explanation.
    pub fn with_conflict(mut self, conflict: ConflictExplanation) -> Self {
        self.conflict = Some(conflict);
        self
    }

    /// 记录一个阶段未完成的原因 / Record why a stage did not complete.
    pub fn note_unavailable(mut self, reason: impl Into<String>) -> Self {
        self.unavailable_reasons.push(reason.into());
        self.has_unsupported_stage = true;
        self
    }

    /// 构建报告 / Build the report.
    pub fn build(self, config: &CandidateFunnelConfig) -> Result<CriticalConstraintAnalysisReport> {
        let baseline = self.baseline.ok_or_else(|| {
            invalid_analysis("critical constraint analysis requires a baseline summary")
        })?;

        let funnel = match &self.activity {
            Some(activity) => Some(build_candidate_funnel(
                activity,
                self.local_sensitivity.as_ref(),
                config,
            )?),
            None => None,
        };
        let effectiveness = if self.perturbation.is_empty() {
            None
        } else {
            let baseline_objective = baseline.optimal_value.ok_or_else(|| {
                invalid_analysis(
                    "effectiveness ranking requires a proven baseline objective value",
                )
            })?;
            Some(build_effectiveness_ranking(
                baseline_objective,
                funnel.as_ref(),
                &self.perturbation,
            )?)
        };
        let group_summaries = summarize_groups(
            self.activity.as_ref(),
            effectiveness.as_ref(),
            self.conflict.as_ref(),
        );

        // The overall status is the weakest link: any Unknown or Unsupported stage dominates,
        // and a missing stage is reported as Unsupported rather than silently ignored.
        // 总体结论取最弱环节：任一阶段为 Unknown 或 Unsupported 即主导，
        // 缺失阶段按 Unsupported 表示，而不是被静默忽略。
        let mut status = if baseline.proven_optimal {
            AnalysisStatus::Reachable
        } else {
            AnalysisStatus::Unknown
        };
        if self.has_unsupported_stage {
            status = combine_status(status, Some(AnalysisStatus::Unsupported));
        }
        status = combine_status(status, self.target.as_ref().map(|t| t.status));
        status = combine_status(
            status,
            self.conflict.as_ref().map(|conflict| conflict.status),
        );
        for report in &self.perturbation {
            status = combine_status(status, Some(report.status));
        }
        if let Some(report) = &self.local_sensitivity {
            status = combine_status(status, Some(report.status));
        }
        let mut unavailable_reasons = self.unavailable_reasons;
        if self.activity.is_none() {
            unavailable_reasons.push("activity stage was not run".to_owned());
            status = combine_status(status, Some(AnalysisStatus::Unsupported));
        }
        if funnel
            .as_ref()
            .is_some_and(|ranking| ranking.unclassified > 0)
        {
            unavailable_reasons.push(format!(
                "{} candidate(s) could not be tiered because dual evidence was unavailable",
                funnel.as_ref().map(|r| r.unclassified).unwrap_or(0)
            ));
        }

        let report = CriticalConstraintAnalysisReport {
            schema_version: CRITICAL_ANALYSIS_REPORT_SCHEMA_VERSION.to_owned(),
            baseline,
            activity: self.activity,
            local_sensitivity: self.local_sensitivity,
            funnel,
            effectiveness,
            target: self.target,
            conflict: self.conflict,
            group_summaries,
            status,
            unavailable_reasons,
        };
        report.validate()?;
        Ok(report)
    }
}

/// Options for the public critical-constraint analysis pipeline.
///
/// Every expensive stage is independently switchable.  A disabled stage is recorded as
/// `Unsupported` by the pipeline builder; it is never silently omitted and never converted into
/// an effectiveness claim.  The CP and conflict options are borrowed only for the duration of a
/// call, so callers can safely provide cancellation handles and progress reporters.
///
/// 公共临界约束分析流水线的配置。每个昂贵阶段都可以独立关闭；关闭阶段由流水线通过
/// `Unsupported` 和原因显式记录，绝不会静默跳过或转化成有效性结论。
#[derive(Clone)]
pub struct CriticalConstraintAnalysisOptions<'a> {
    /// 候选漏斗配置 / Candidate-funnel configuration.
    pub funnel: CandidateFunnelConfig,
    /// 固定整数 LP 灵敏度配置 / Fixed-integer LP sensitivity configuration.
    pub fixed_integer: FixedIntegerLpSensitivityConfig,
    /// RHS 扰动与删除策略 / RHS perturbation/removal policy.
    pub perturbation: ConstraintPerturbationPolicy,
    /// baseline 与目标可行性共用的 CP 求解配置 / CP solve options for baseline and target feasibility.
    pub solve_options: ConstraintProgrammingSolveOptions<'a>,
    /// conflict/MUS 提取配置 / Options used by conflict/MUS extraction.
    pub conflict: ConflictAnalysisOptions<'a>,
    /// 送入扰动 adapter 的候选上限 / Maximum candidates sent to the perturbation adapter.
    pub candidate_limit: usize,
    /// adapter 存在时是否执行固定整数 LP / Whether to run fixed-integer LP when an adapter exists.
    pub run_fixed_integer: bool,
    /// adapter 存在时是否执行扰动与删除 / Whether to run perturbation/removal when an adapter exists.
    pub run_perturbation: bool,
    /// target 存在时是否检查其可行性 / Whether to check target feasibility when a target is supplied.
    pub run_target_analysis: bool,
    /// target 确认不可达后是否执行 conflict/MUS / Whether to run conflict/MUS after a proven unreachable target.
    pub run_conflict_analysis: bool,
}

impl<'a> Default for CriticalConstraintAnalysisOptions<'a> {
    fn default() -> Self {
        Self {
            funnel: CandidateFunnelConfig::default(),
            fixed_integer: FixedIntegerLpSensitivityConfig::default(),
            perturbation: ConstraintPerturbationPolicy::default(),
            solve_options: ConstraintProgrammingSolveOptions::new(),
            conflict: ConflictAnalysisOptions::default(),
            candidate_limit: CandidateFunnelConfig::default().default_candidate_limit,
            run_fixed_integer: true,
            run_perturbation: true,
            run_target_analysis: true,
            run_conflict_analysis: true,
        }
    }
}

impl<'a> CriticalConstraintAnalysisOptions<'a> {
    /// 在调用 backend 前校验所有阶段配置 / Validate all stage options before any backend is called.
    pub fn validate(&self) -> Result<()> {
        self.funnel.validate()?;
        self.fixed_integer.validate()?;
        self.perturbation.validate()?;
        self.solve_options.validate()?;
        self.conflict.validate()?;
        Ok(())
    }

    /// 设置固定整数 LP 配置 / Set the fixed-integer LP configuration.
    pub const fn with_fixed_integer(
        mut self,
        config: FixedIntegerLpSensitivityConfig,
    ) -> Self {
        self.fixed_integer = config;
        self
    }

    /// 设置扰动策略 / Set the perturbation policy.
    pub fn with_perturbation(mut self, policy: ConstraintPerturbationPolicy) -> Self {
        self.perturbation = policy;
        self
    }

    /// 设置候选上限；零表示只做筛选 / Set the candidate limit; zero intentionally means screening only.
    pub const fn with_candidate_limit(mut self, limit: usize) -> Self {
        self.candidate_limit = limit;
        self
    }

    /// 启用或关闭固定整数 LP 分析 / Enable or disable fixed-integer LP analysis.
    pub const fn with_fixed_integer_analysis(mut self, enabled: bool) -> Self {
        self.run_fixed_integer = enabled;
        self
    }

    /// 启用或关闭扰动分析 / Enable or disable perturbation analysis.
    pub const fn with_perturbation_analysis(mut self, enabled: bool) -> Self {
        self.run_perturbation = enabled;
        self
    }

    /// 启用或关闭目标可行性分析 / Enable or disable target feasibility analysis.
    pub const fn with_target_analysis(mut self, enabled: bool) -> Self {
        self.run_target_analysis = enabled;
        self
    }

    /// 启用或关闭 conflict/MUS 分析 / Enable or disable conflict/MUS analysis.
    pub const fn with_conflict_analysis(mut self, enabled: bool) -> Self {
        self.run_conflict_analysis = enabled;
        self
    }
}

/// 固定整数 LP adapter 请求 / Request passed to a fixed-integer LP adapter.
///
/// The request exposes only the immutable source snapshot and stable baseline values.  An
/// adapter may lower these values internally, but solver rows, columns, and handles must not be
/// placed in the returned report.
#[derive(Debug, Clone, Copy)]
pub struct FixedIntegerLpAnalysisRequest<'a> {
    /// 不可变原始 CP 快照 / Immutable source CP snapshot.
    pub snapshot: &'a ConstraintProgrammingSnapshot,
    /// 以稳定原始变量身份为键的完整 baseline 赋值 / Complete baseline assignment keyed by stable source variable identity.
    pub baseline_values: &'a BTreeMap<StableVariableId, i64>,
    /// baseline 目标值 / Baseline objective value.
    pub baseline_objective: f64,
    /// 阶段配置 / Stage configuration.
    pub config: &'a FixedIntegerLpSensitivityConfig,
}

/// 与 backend 无关的固定整数 LP adapter / Backend-neutral fixed-integer LP adapter.
pub trait FixedIntegerLpAnalysisAdapter {
    /// 针对不可变原始快照执行固定整数灵敏度分析 / Run fixed-integer sensitivity against the immutable source snapshot.
    fn analyze(&self, request: FixedIntegerLpAnalysisRequest<'_>)
        -> Result<LocalConstraintSensitivityReport>;
}

/// 固定整数 LP adapter 的兼容名称 / Compatibility name for fixed-integer LP adapters.
pub use FixedIntegerLpAnalysisAdapter as FixedIntegerLpAdapter;

impl<F> FixedIntegerLpAnalysisAdapter for F
where
    F: for<'a> Fn(FixedIntegerLpAnalysisRequest<'a>) -> Result<LocalConstraintSensitivityReport>,
{
    fn analyze(&self, request: FixedIntegerLpAnalysisRequest<'_>)
        -> Result<LocalConstraintSensitivityReport> {
        self(request)
    }
}

/// 扰动与删除 adapter 请求 / Request passed to a perturbation/removal adapter.
#[derive(Debug, Clone, Copy)]
pub struct ConstraintPerturbationAnalysisRequest<'a> {
    /// 不可变原始 CP 快照 / Immutable source CP snapshot.
    pub snapshot: &'a ConstraintProgrammingSnapshot,
    /// 以稳定原始变量身份为键的完整 baseline 赋值 / Complete baseline assignment keyed by stable source variable identity.
    pub baseline_values: &'a BTreeMap<StableVariableId, i64>,
    /// baseline 目标值 / Baseline objective value.
    pub baseline_objective: f64,
    /// 候选原始约束身份 / Candidate original constraint identity.
    pub constraint_id: &'a ConstraintId,
    /// 阶段配置 / Stage configuration.
    pub policy: &'a ConstraintPerturbationPolicy,
}

/// 与 backend 无关的 RHS 扰动与删除 adapter / Backend-neutral RHS perturbation/removal adapter.
pub trait ConstraintPerturbationAnalysisAdapter {
    /// 针对一个原始约束执行扰动与删除 / Run perturbation/removal for one original constraint.
    fn analyze(&self, request: ConstraintPerturbationAnalysisRequest<'_>)
        -> Result<ConstraintPerturbationReport>;
}

/// 扰动 adapter 的兼容名称 / Compatibility name for perturbation adapters.
pub use ConstraintPerturbationAnalysisAdapter as ConstraintPerturbationAdapter;

impl<F> ConstraintPerturbationAnalysisAdapter for F
where
    F: for<'a> Fn(ConstraintPerturbationAnalysisRequest<'a>) -> Result<ConstraintPerturbationReport>,
{
    fn analyze(&self, request: ConstraintPerturbationAnalysisRequest<'_>)
        -> Result<ConstraintPerturbationReport> {
        self(request)
    }
}

/// A real CP-to-LP adapter for the existing fixed-integer analyzer.
///
/// The adapter deliberately delegates to [`FixedIntegerLpSensitivityAnalyzer`] instead of
/// manufacturing duals.  For pure integer CP snapshots the existing analyzer may return an
/// explicit `Unsupported` report when no original row has a defensible LP interpretation.
///
/// 使用现有固定整数分析器的真实 CP→LP adapter。它不会伪造对偶；纯整数 CP 模型在没有可
/// 辩护的原始 LP 行时会由已有 analyzer 明确返回 `Unsupported`。
pub struct ConstraintProgrammingFixedIntegerLpAdapter<'a> {
    solver: &'a dyn LinearSolver,
}

impl<'a> ConstraintProgrammingFixedIntegerLpAdapter<'a> {
    /// Create an adapter around an existing linear solver.
    pub fn new<S>(solver: &'a S) -> Self
    where
        S: LinearSolver,
    {
        Self { solver }
    }
}

impl FixedIntegerLpAnalysisAdapter for ConstraintProgrammingFixedIntegerLpAdapter<'_> {
    fn analyze(&self, request: FixedIntegerLpAnalysisRequest<'_>)
        -> Result<LocalConstraintSensitivityReport> {
        FixedIntegerLpSensitivityAnalyzer::new(*request.config)?.analyze_constraint_programming(
            self.solver,
            request.snapshot,
            request.baseline_values,
            request.baseline_objective,
        )
    }
}

/// Public facade that orchestrates the existing analysis stages.
///
/// The CP solver is mandatory because target and conflict analysis require a real solver.  LP
/// sensitivity and RHS perturbation are represented by explicit optional adapters; a missing
/// adapter is recorded as `Unsupported` and never replaced with a fake solve.  The facade owns no
/// solver and therefore does not change solver lifetime or shutdown semantics.
///
/// 公共分析 facade。CP solver 是必需的，因为 target 和 conflict 需要真实求解器；LP 对偶与
/// RHS 扰动通过显式可选 adapter 接入，缺失时记录 `Unsupported`，绝不以假求解替代。
pub struct CriticalConstraintAnalysisPipeline<'a> {
    solver: &'a dyn ConstraintProgrammingSolver,
    fixed_integer_adapter: Option<Box<dyn FixedIntegerLpAnalysisAdapter + 'a>>,
    perturbation_adapter: Option<Box<dyn ConstraintPerturbationAnalysisAdapter + 'a>>,
    activity_analyzer: ConstraintActivityAnalyzer,
    capability_override: Option<CapabilityMatrix>,
}

impl<'a> CriticalConstraintAnalysisPipeline<'a> {
    /// 使用 solver 运行时 CP 能力报告创建 pipeline / Create a pipeline using the solver's runtime CP capability report.
    pub fn new<S>(solver: &'a S) -> Self
    where
        S: ConstraintProgrammingSolver,
    {
        Self {
            solver,
            fixed_integer_adapter: None,
            perturbation_adapter: None,
            activity_analyzer: ConstraintActivityAnalyzer::with_defaults(),
            capability_override: None,
        }
    }

    /// 接入固定整数 LP adapter / Attach a fixed-integer LP adapter.
    pub fn with_fixed_integer_adapter<A>(mut self, adapter: A) -> Self
    where
        A: FixedIntegerLpAnalysisAdapter + 'a,
    {
        self.fixed_integer_adapter = Some(Box::new(adapter));
        self
    }

    /// 使用现有线性 solver 接入 CP→LP adapter / Attach the CP-to-LP adapter to an existing linear solver.
    ///
    /// 该便捷入口只包装真实的 [`FixedIntegerLpSensitivityAnalyzer`]；后端仍负责提供 LP
    /// 求解能力，Rust 不会据此宣称 Gurobi 或 SCIP 已自动接线。
    /// This convenience method only wraps the real [`FixedIntegerLpSensitivityAnalyzer`]. The
    /// backend still supplies LP solving, and Rust does not claim that Gurobi or SCIP is wired
    /// automatically by this method.
    pub fn with_constraint_programming_fixed_integer_solver<S>(
        self,
        solver: &'a S,
    ) -> Self
    where
        S: LinearSolver + 'a,
    {
        self.with_fixed_integer_adapter(ConstraintProgrammingFixedIntegerLpAdapter::new(solver))
    }

    /// 接入 RHS 扰动与删除 adapter / Attach a RHS perturbation/removal adapter.
    pub fn with_perturbation_adapter<A>(mut self, adapter: A) -> Self
    where
        A: ConstraintPerturbationAnalysisAdapter + 'a,
    {
        self.perturbation_adapter = Some(Box::new(adapter));
        self
    }

    /// 覆盖指定能力项，同时保留 solver 运行时能力 / Override selected capability entries while retaining runtime solver capabilities.
    ///
    /// Only entries explicitly present in the supplied matrix override runtime inference.  This
    /// lets `CapabilityMatrix::default()` remain a useful empty override while still allowing a
    /// caller to force a particular stage to `Unsupported`.
    pub fn with_capability_matrix(mut self, matrix: CapabilityMatrix) -> Self {
        self.capability_override = Some(matrix);
        self
    }

    /// 替换活动性分析器及其容差配置 / Replace the activity analyzer and its tolerance configuration.
    pub fn with_activity_analyzer(mut self, analyzer: ConstraintActivityAnalyzer) -> Self {
        self.activity_analyzer = analyzer;
        self
    }

    /// 是否已配置固定整数 adapter / Return the configured fixed-integer adapter state.
    pub fn has_fixed_integer_adapter(&self) -> bool {
        self.fixed_integer_adapter.is_some()
    }

    /// 是否已配置扰动 adapter / Return the configured perturbation adapter state.
    pub fn has_perturbation_adapter(&self) -> bool {
        self.perturbation_adapter.is_some()
    }

    /// 基于已校验的 baseline 求解报告运行后续完整分析 / Run the complete post-baseline pipeline from a validated solve report.
    ///
    /// The baseline report is validated and copied into a fresh analysis session.  Its stable
    /// incumbent is the only source used for activity and later adapters.  A feasible incumbent
    /// without `SolutionPresence::Optimal` is retained as evidence but cannot produce a proven
    /// overall `Reachable` conclusion.
    pub fn analyze<'b>(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        baseline_report: &SolveReport<i64>,
        target: Option<&ObjectiveTarget>,
        options: &CriticalConstraintAnalysisOptions<'b>,
    ) -> Result<CriticalConstraintAnalysisReport> {
        options.validate()?;
        snapshot.validate_identity()?;
        baseline_report.validate()?;
        let capability = self.capability_for(snapshot);
        let mut session = CriticalConstraintAnalysisSession::from_baseline_report(
            snapshot.clone(),
            baseline_report,
            Some(self.solver.descriptor()),
            Some(capability),
        )?;
        let proven_optimal = baseline_report.problem_status == ProblemStatus::Feasible
            && baseline_report.is_optimal();
        let baseline_status = if proven_optimal {
            AnalysisStatus::Reachable
        } else if baseline_report.problem_status == ProblemStatus::Infeasible
            && crate::solver::require_infeasibility_certificate(baseline_report).is_ok()
        {
            AnalysisStatus::Unreachable
        } else {
            AnalysisStatus::from_problem_status(baseline_report.problem_status)
        };
        let result = self.run_session(
            &mut session,
            baseline_status,
            proven_optimal,
            target,
            options,
        );
        session.close();
        result
    }

    /// 求解 baseline 后运行完整分析 / Solve the baseline and run the complete analysis pipeline.
    ///
    /// 此入口使用 `solve_options` 只求解原始 snapshot；目标条件只在后续 target 阶段加入派生
    /// snapshot。这样 baseline、activity 和 adapter 始终共享同一原始证据边界。
    /// This entry uses `solve_options` only for the source snapshot; the target is added later to
    /// a derived snapshot, keeping baseline, activity, and adapters on the same evidence boundary.
    pub fn solve_and_analyze<'b>(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        target: Option<&ObjectiveTarget>,
        options: &CriticalConstraintAnalysisOptions<'b>,
    ) -> Result<CriticalConstraintAnalysisReport> {
        options.validate()?;
        snapshot.validate_identity()?;
        let baseline = self
            .solver
            .solve_constraint_programming(snapshot, &options.solve_options)?;
        self.analyze(snapshot, &baseline, target, options)
    }

    /// 显式命名 baseline 参数的兼容别名 / Compatibility alias that names the baseline argument explicitly.
    pub fn analyze_from_baseline_report<'b>(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        baseline_report: &SolveReport<i64>,
        target: Option<&ObjectiveTarget>,
        options: &CriticalConstraintAnalysisOptions<'b>,
    ) -> Result<CriticalConstraintAnalysisReport> {
        self.analyze(snapshot, baseline_report, target, options)
    }

    /// 基于调用方提供的稳定 baseline 赋值运行 pipeline / Run the pipeline from caller-supplied stable baseline values.
    ///
    /// This entry point is useful when the caller owns a CP solve report in another layer.  The
    /// `baseline_proven_optimal` flag is an explicit assertion supplied by that layer; the
    /// pipeline still validates the assignment and all downstream reports.
    pub fn analyze_with_baseline_values<'b>(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        baseline_values: BTreeMap<StableVariableId, i64>,
        baseline_objective: Option<f64>,
        baseline_proven_optimal: bool,
        target: Option<&ObjectiveTarget>,
        options: &CriticalConstraintAnalysisOptions<'b>,
    ) -> Result<CriticalConstraintAnalysisReport> {
        options.validate()?;
        snapshot.validate_identity()?;
        snapshot.validate_assignment(&baseline_values)?;
        let capability = self.capability_for(snapshot);
        let mut session = CriticalConstraintAnalysisSession::from_snapshot_with_options(
            snapshot.clone(),
            Some(baseline_values),
            baseline_objective,
            Some(self.solver.descriptor()),
            Some(capability),
        )?;
        let baseline_status = if baseline_proven_optimal {
            AnalysisStatus::Reachable
        } else {
            AnalysisStatus::Unknown
        };
        let result = self.run_session(
            &mut session,
            baseline_status,
            baseline_proven_optimal,
            target,
            options,
        );
        session.close();
        result
    }

    /// 在既有 session 上运行 pipeline，且不取得其所有权 / Run the pipeline over an existing session without taking ownership of it.
    ///
    /// The session's baseline is retained and its stage status cache is populated.  This method
    /// does not close the session; the caller remains responsible for its lifecycle.
    pub fn analyze_session<'b>(
        &self,
        session: &mut CriticalConstraintAnalysisSession,
        baseline_status: AnalysisStatus,
        baseline_proven_optimal: bool,
        target: Option<&ObjectiveTarget>,
        options: &CriticalConstraintAnalysisOptions<'b>,
    ) -> Result<CriticalConstraintAnalysisReport> {
        options.validate()?;
        self.run_session(
            session,
            baseline_status,
            baseline_proven_optimal,
            target,
            options,
        )
    }

    fn run_session<'b>(
        &self,
        session: &mut CriticalConstraintAnalysisSession,
        baseline_status: AnalysisStatus,
        baseline_proven_optimal: bool,
        target: Option<&ObjectiveTarget>,
        options: &CriticalConstraintAnalysisOptions<'b>,
    ) -> Result<CriticalConstraintAnalysisReport> {
        if session.is_closed() {
            return Err(invalid_analysis("analysis session is closed"));
        }
        // Keep a local immutable snapshot copy so stage caches can be updated on the mutable
        // session without holding an immutable borrow across adapter calls.
        let snapshot_owned = session.baseline_snapshot().clone();
        let snapshot = &snapshot_owned;
        let objective = snapshot.objective.as_ref().ok_or_else(|| {
            invalid_analysis("critical constraint analysis requires a CP objective")
        })?;
        let baseline_objective = session.baseline_objective_value();
        let baseline = AnalysisBaselineSummary {
            objective_id: objective.id.clone(),
            sense: objective_sense(objective.category).to_owned(),
            optimal_value: baseline_proven_optimal.then_some(baseline_objective).flatten(),
            status: baseline_status,
            proven_optimal: baseline_proven_optimal,
        };
        let mut builder = CriticalConstraintAnalysisReportBuilder::new().with_baseline(baseline);

        let baseline_values = session.baseline_solution().cloned();
        let activity = match baseline_values.as_ref() {
            Some(values) => {
                snapshot.validate_assignment(values)?;
                let report = self.activity_analyzer.analyze_cp(snapshot, values)?;
                builder = builder.with_activity(report.clone());
                Some(report)
            }
            None => {
                builder = builder.note_unavailable(
                    "baseline has no complete incumbent; activity stage is Unsupported",
                );
                None
            }
        };

        let capability = session.capability_matrix().clone();
        let mut local_sensitivity = None;
        if !options.run_fixed_integer {
            builder = builder.note_unavailable("fixed-integer LP stage was disabled");
        } else if self.fixed_integer_adapter.is_none() {
            builder = builder.note_unavailable(
                "no fixed-integer LP adapter was supplied; local sensitivity is Unsupported",
            );
        } else if capability.support(AnalysisCapability::FixedIntegerLpSensitivity)
            == crate::solver::CapabilitySupport::Unsupported
        {
            builder = builder.note_unavailable(
                "capability matrix marks fixed-integer LP sensitivity Unsupported",
            );
        } else {
            if let (Some(values), Some(objective_value)) =
                (baseline_values.as_ref(), baseline_objective)
            {
                let adapter = self
                    .fixed_integer_adapter
                    .as_ref()
                    .expect("adapter presence checked above");
                let report = adapter.analyze(FixedIntegerLpAnalysisRequest {
                    snapshot,
                    baseline_values: values,
                    baseline_objective: objective_value,
                    config: &options.fixed_integer,
                })?;
                report.validate()?;
                cache_local_sensitivity(session, &report)?;
                local_sensitivity = Some(report.clone());
                builder = builder.with_local_sensitivity(report);
            } else {
                if baseline_values.is_none() {
                    builder = builder.note_unavailable(
                        "fixed-integer LP requires a complete baseline incumbent",
                    );
                }
                if baseline_objective.is_none() {
                    builder = builder.note_unavailable(
                        "fixed-integer LP requires a finite baseline objective",
                    );
                }
            }
        }

        let funnel = activity
            .as_ref()
            .map(|report| build_candidate_funnel(report, local_sensitivity.as_ref(), &options.funnel))
            .transpose()?;

        let mut perturbation_reports = Vec::new();
        if !options.run_perturbation {
            builder = builder.note_unavailable("perturbation stage was disabled");
        } else if self.perturbation_adapter.is_none() {
            builder = builder.note_unavailable(
                "no perturbation adapter was supplied; effectiveness is Unsupported",
            );
        } else if capability.support(AnalysisCapability::RhsPerturbation)
            == crate::solver::CapabilitySupport::Unsupported
        {
            builder = builder.note_unavailable(
                "capability matrix marks RHS perturbation Unsupported",
            );
        } else if baseline_objective.is_none() || !baseline_proven_optimal {
            builder = builder.note_unavailable(
                "perturbation ranking requires a proven finite baseline objective",
            );
        } else if let (Some(values), Some(funnel)) = (baseline_values.as_ref(), funnel.as_ref()) {
            let candidate_ids = funnel
                .shortlist(options.candidate_limit)
                .into_iter()
                .map(|candidate| candidate.constraint_id.clone())
                .collect::<Vec<_>>();
            let adapter = self
                .perturbation_adapter
                .as_ref()
                .expect("adapter presence checked above");
            for constraint_id in &candidate_ids {
                let report = adapter.analyze(ConstraintPerturbationAnalysisRequest {
                    snapshot,
                    baseline_values: values,
                    baseline_objective: baseline_objective
                        .expect("baseline objective checked above"),
                    constraint_id,
                    policy: &options.perturbation,
                })?;
                report.validate()?;
                if report.constraint_id != *constraint_id {
                    return Err(invalid_analysis(
                        "perturbation adapter returned a report for a different constraint",
                    ));
                }
                session.cache_status(
                    AnalysisCacheKind::Perturbation,
                    &DiagnosticSource::Constraint {
                        id: constraint_id.clone(),
                    },
                    report.status,
                )?;
                perturbation_reports.push(report);
            }
            if candidate_ids.is_empty() && !funnel.candidates.is_empty() {
                builder = builder.note_unavailable(
                    "perturbation shortlist is empty; no perturbation was executed",
                );
            }
        } else if baseline_values.is_none() {
            builder = builder.note_unavailable(
                "perturbation requires a complete baseline incumbent",
            );
        }
        if !perturbation_reports.is_empty() {
            builder = builder.with_perturbation(perturbation_reports);
        }

        self.append_target_and_conflict(
            builder,
            session,
            target,
            options,
            capability,
        )
    }

    fn append_target_and_conflict<'b>(
        &self,
        mut builder: CriticalConstraintAnalysisReportBuilder,
        session: &mut CriticalConstraintAnalysisSession,
        target: Option<&ObjectiveTarget>,
        options: &CriticalConstraintAnalysisOptions<'b>,
        capability: CapabilityMatrix,
    ) -> Result<CriticalConstraintAnalysisReport> {
        if !options.run_target_analysis {
            builder = builder.note_unavailable("target-feasibility stage was disabled");
            return builder.build(&options.funnel);
        }
        let Some(target) = target else {
            builder = builder.note_unavailable(
                "no objective target was supplied; target/conflict analysis did not run",
            );
            return builder.build(&options.funnel);
        };
        let target_report = TargetFeasibilityAnalyzer::new().analyze_with_capability(
            self.solver,
            session.baseline_snapshot(),
            target,
            &options.solve_options,
            &capability,
        )?;
        target_report.validate()?;
        session.cache_status(AnalysisCacheKind::Target, &target_report.source, target_report.status)?;
        builder = builder.with_target(target_report.clone());

        if target_report.status != AnalysisStatus::Unreachable {
            return builder.build(&options.funnel);
        }
        if !options.run_conflict_analysis {
            builder = builder.note_unavailable("conflict/MUS stage was disabled");
            return builder.build(&options.funnel);
        }
        let conflict = ConflictAnalyzer::new().analyze_with_capability(
            self.solver,
            session.baseline_snapshot(),
            target,
            &options.conflict,
            &capability,
        )?;
        conflict.validate()?;
        session.cache_status(AnalysisCacheKind::Conflict, &conflict.target_source, conflict.status)?;
        builder.with_conflict(conflict).build(&options.funnel)
    }

    fn capability_for(&self, snapshot: &ConstraintProgrammingSnapshot) -> CapabilityMatrix {
        let support = self.solver.analyze_support(snapshot);
        let descriptor = self.solver.descriptor();
        let mut capability = CapabilityMatrix::from_constraint_programming_support(&descriptor, &support);
        if let Some(override_matrix) = &self.capability_override {
            for (kind, level) in &override_matrix.analysis_capabilities {
                capability.analysis_capabilities.insert(*kind, *level);
            }
            for (feature, level) in &override_matrix.constraint_programming_features {
                capability.constraint_programming_features.insert(*feature, *level);
            }
            if !override_matrix.model_types.is_empty() {
                capability.model_types = override_matrix.model_types.clone();
            }
            capability.native_assumption_solving |= override_matrix.native_assumption_solving;
            capability.native_unsat_core |= override_matrix.native_unsat_core;
            capability.fallback_satisfaction_only |= override_matrix.fallback_satisfaction_only;
            capability.dual |= override_matrix.dual;
            capability.warm_start |= override_matrix.warm_start;
            capability.exact_cp_lowering |= override_matrix.exact_cp_lowering;
        }
        if self.fixed_integer_adapter.is_some()
            && capability
                .analysis_capabilities
                .get(&AnalysisCapability::FixedIntegerLpSensitivity)
                != Some(&crate::solver::CapabilitySupport::Unsupported)
        {
            capability.analysis_capabilities.insert(
                AnalysisCapability::FixedIntegerLpSensitivity,
                crate::solver::CapabilitySupport::Supported,
            );
        }
        if self.perturbation_adapter.is_some()
            && capability
                .analysis_capabilities
                .get(&AnalysisCapability::RhsPerturbation)
                != Some(&crate::solver::CapabilitySupport::Unsupported)
        {
            capability.analysis_capabilities.insert(
                AnalysisCapability::RhsPerturbation,
                crate::solver::CapabilitySupport::Supported,
            );
            capability.analysis_capabilities.insert(
                AnalysisCapability::RemovalTest,
                crate::solver::CapabilitySupport::Supported,
            );
            capability.analysis_capabilities.insert(
                AnalysisCapability::AdaptivePerturbation,
                crate::solver::CapabilitySupport::Supported,
            );
        }
        capability
    }
}

fn objective_sense(category: ObjectiveCategory) -> &'static str {
    match category {
        ObjectiveCategory::Minimum => "minimize",
        ObjectiveCategory::Maximum => "maximize",
    }
}

fn cache_local_sensitivity(
    session: &mut CriticalConstraintAnalysisSession,
    report: &LocalConstraintSensitivityReport,
) -> Result<()> {
    for sensitivity in &report.constraints {
        session.cache_status(
            AnalysisCacheKind::FixedIntegerLp,
            &DiagnosticSource::Constraint {
                id: sensitivity.constraint_id.clone(),
            },
            sensitivity.status,
        )?;
    }
    Ok(())
}

/// 合并两个分析状态，取更弱的一方 / Combine two analysis statuses, keeping the weaker one.
///
/// 顺序由强到弱：`Unsupported` 最弱，其次 `Unknown`，再次 `Unreachable`，`Reachable` 最强。
/// 这与计划 8.14 一致：任何未证明的阶段都不得被更强的结论掩盖。
/// Order from strongest to weakest: `Reachable`, `Unreachable`, `Unknown`, `Unsupported`. This
/// matches plan 8.14: no unproven stage may be masked by a stronger conclusion.
pub fn combine_status(current: AnalysisStatus, next: Option<AnalysisStatus>) -> AnalysisStatus {
    let Some(next) = next else {
        return current;
    };
    let rank = |status: AnalysisStatus| match status {
        AnalysisStatus::Reachable => 3,
        AnalysisStatus::Unreachable => 2,
        AnalysisStatus::Unknown => 1,
        AnalysisStatus::Unsupported => 0,
    };
    if rank(next) < rank(current) {
        next
    } else {
        current
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::analysis::{
        ActivitySummary, ConstraintActivity, ConstraintPerturbationObservation,
        LocalConstraintSensitivity, SensitivityScope, PERTURBATION_REPORT_SCHEMA_VERSION,
    };
    use crate::model::constraint_programming::{
        ConstraintDefinition, ConstraintProgrammingConstraint, ConstraintProgrammingModel,
        IntegerDomain, IntegerExpression, IntegerObjective, IntegerRelation, IntegerVariable,
    };
    use crate::solver::constraint_programming::{
        ConstraintProgrammingSolver, FakeConstraintProgrammingSolver,
    };
    use crate::solver::{SolverInfo, SolverOutput, SolverStatus};

    #[derive(Debug, Default)]
    struct CountingLinearSolver {
        calls: AtomicUsize,
    }

    impl SolverInfo for CountingLinearSolver {
        fn name(&self) -> &str {
            "counting-analysis-linear"
        }

        fn capabilities(&self) -> Vec<crate::solver::SolverCapability> {
            vec![
                crate::solver::SolverCapability::Linear,
                crate::solver::SolverCapability::Mip,
            ]
        }
    }

    impl crate::solver::LinearSolver for CountingLinearSolver {
        fn solve_linear(
            &self,
            _model: &crate::model::intermediate::LinearTriadModel,
        ) -> crate::error::Result<SolverOutput> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(SolverOutput::new(SolverStatus::Unknown))
        }
    }

    fn pipeline_snapshot() -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("critical-analysis-pipeline");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 20).expect("domain"))
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
        model.set_objective(IntegerObjective::maximize_with_id(
            "payload",
            IntegerExpression::variable(x),
        ));
        model.freeze().expect("snapshot")
    }

    fn activity_record(
        id: &str,
        group: Option<&str>,
        status: ActivityStatus,
    ) -> ConstraintActivity {
        ConstraintActivity {
            constraint_id: id.into(),
            group: group.map(str::to_owned),
            status,
            lhs: Some(1.0),
            rhs: Some(1.0),
            slack: Some(0.0),
            normalized_slack: Some(0.0),
            tolerance: Some(1e-7),
            satisfied: Some(true),
            evidence: crate::analysis::ActivityEvidence::IntegerComparison {
                relation: crate::model::constraint_programming::IntegerRelation::LessOrEqual,
                lhs: 1,
                rhs: 1,
            },
        }
    }

    fn activity_report(records: Vec<ConstraintActivity>) -> ConstraintActivityReport {
        // Derive summaries from the records so the report satisfies its own count invariants.
        // 从记录派生汇总，使报告满足自身的计数不变量。
        let mut summary = ActivitySummary::default();
        let mut grouped = BTreeMap::<Option<String>, ActivitySummary>::new();
        for record in &records {
            summary.add(record.status);
            grouped
                .entry(record.group.clone())
                .or_default()
                .add(record.status);
        }
        let groups = grouped
            .into_iter()
            .map(|(group, summary)| crate::analysis::ActivityGroupSummary { group, summary })
            .collect();
        ConstraintActivityReport {
            schema_version: "1.0".to_owned(),
            constraints: records,
            variable_bounds: Vec::new(),
            summary,
            variable_bound_summary: ActivitySummary::default(),
            groups,
            tolerance: 1e-7,
            nearly_active_tolerance: 1e-5,
        }
    }

    fn sensitivity_report(entries: Vec<(&str, Option<f64>)>) -> LocalConstraintSensitivityReport {
        LocalConstraintSensitivityReport {
            schema_version: "1.0".to_owned(),
            scope: SensitivityScope::FixedIntegerIncumbent,
            baseline_objective: 10.0,
            lp_objective: Some(10.0),
            fixed_integer_count: 1,
            constraints: entries
                .into_iter()
                .map(|(id, dual)| LocalConstraintSensitivity {
                    constraint_id: id.into(),
                    dual_value: dual,
                    local_effective: dual.map(|value| value.abs() > 1e-9),
                    status: AnalysisStatus::Reachable,
                    scope: SensitivityScope::FixedIntegerIncumbent,
                    activity: None,
                })
                .collect(),
            status: AnalysisStatus::Reachable,
            termination_reason: None,
            unavailable_reason: None,
        }
    }

    #[test]
    fn pipeline_continues_cp_target_and_conflict_when_lp_adapters_are_unavailable() {
        let solver = FakeConstraintProgrammingSolver::new();
        let snapshot = pipeline_snapshot();
        let target = ObjectiveTarget::at_least("payload", 11.0).expect("target");
        let report = CriticalConstraintAnalysisPipeline::new(&solver)
            .solve_and_analyze(
                &snapshot,
                Some(&target),
                &CriticalConstraintAnalysisOptions::default(),
            )
            .expect("pipeline report");

        assert!(report.activity.is_some());
        assert!(report.local_sensitivity.is_none());
        assert_eq!(
            report.target.as_ref().map(|entry| entry.status),
            Some(AnalysisStatus::Unreachable)
        );
        let conflict = report.conflict.as_ref().expect("conflict after unreachable target");
        assert!(conflict.members.iter().any(|source| {
            matches!(source, DiagnosticSource::Constraint { id } if id.0 == "capacity")
        }));
        assert_eq!(report.status, AnalysisStatus::Unsupported);
        assert!(report
            .unavailable_reasons
            .iter()
            .any(|reason| reason.contains("fixed-integer LP adapter")));
        assert!(report
            .unavailable_reasons
            .iter()
            .any(|reason| reason.contains("perturbation adapter")));
        report.validate().expect("valid pipeline report");
    }

    #[test]
    fn pipeline_passes_source_snapshot_and_incumbent_to_fixed_integer_adapter() {
        let solver = FakeConstraintProgrammingSolver::new();
        let snapshot = pipeline_snapshot();
        let baseline = solver
            .solve_constraint_programming(&snapshot, &ConstraintProgrammingSolveOptions::new())
            .expect("baseline");
        let calls = Cell::new(0usize);
        let adapter = |request: FixedIntegerLpAnalysisRequest<'_>| {
            calls.set(calls.get() + 1);
            assert_eq!(request.snapshot.fingerprint, snapshot.fingerprint);
            assert_eq!(
                request.baseline_values.get(&StableVariableId::from("x")),
                Some(&10)
            );
            assert_eq!(request.baseline_objective, 10.0);
            Ok(sensitivity_report(vec![("capacity", Some(2.0))]))
        };
        let options = CriticalConstraintAnalysisOptions::default()
            .with_perturbation_analysis(false)
            .with_target_analysis(false);
        let report = CriticalConstraintAnalysisPipeline::new(&solver)
            .with_fixed_integer_adapter(&adapter)
            .analyze(&snapshot, &baseline, None, &options)
            .expect("pipeline report");

        assert_eq!(calls.get(), 1);
        assert!(report.local_sensitivity.is_some());
        assert_eq!(report.funnel.as_ref().map(|funnel| funnel.tier_a), Some(1));
        report.validate().expect("valid pipeline report");
    }

    #[test]
    fn pipeline_fixed_integer_solver_convenience_uses_real_adapter() {
        let cp_solver = FakeConstraintProgrammingSolver::new();
        let lp_solver = CountingLinearSolver::default();
        let snapshot = pipeline_snapshot();
        let options = CriticalConstraintAnalysisOptions::default()
            .with_perturbation_analysis(false)
            .with_target_analysis(false);
        let report = CriticalConstraintAnalysisPipeline::new(&cp_solver)
            .with_constraint_programming_fixed_integer_solver(&lp_solver)
            .analyze_with_baseline_values(
                &snapshot,
                BTreeMap::from([(StableVariableId::from("x"), 10)]),
                Some(10.0),
                true,
                None,
                &options,
            )
            .expect("pipeline report");

        assert_eq!(lp_solver.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            report
                .local_sensitivity
                .as_ref()
                .map(|entry| entry.status),
            Some(AnalysisStatus::Unsupported)
        );
        let sensitivity = report
            .local_sensitivity
            .as_ref()
            .expect("local sensitivity report");
        assert!(sensitivity.constraints.is_empty());
        assert!(sensitivity
            .unavailable_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("no original CP constraint maps")));
        report.validate().expect("valid pipeline report");
    }

    #[test]
    fn pipeline_perturbation_adapter_is_called_and_missing_adapter_is_explicit() {
        let cp_solver = FakeConstraintProgrammingSolver::new();
        let snapshot = pipeline_snapshot();
        let calls = Cell::new(0usize);
        let adapter = |request: ConstraintPerturbationAnalysisRequest<'_>| {
            calls.set(calls.get() + 1);
            assert_eq!(request.snapshot.fingerprint, snapshot.fingerprint);
            assert_eq!(request.baseline_objective, 10.0);
            Ok(ConstraintPerturbationReport {
                schema_version: PERTURBATION_REPORT_SCHEMA_VERSION.to_owned(),
                constraint_id: request.constraint_id.clone(),
                baseline_objective: request.baseline_objective,
                observations: Vec::new(),
                removal: None,
                adaptive_outcome: None,
                status: AnalysisStatus::Unsupported,
                solve_count: 0,
                unavailable_reason: Some("test adapter did not provide a backend".to_owned()),
            })
        };
        let options = CriticalConstraintAnalysisOptions::default()
            .with_fixed_integer_analysis(false)
            .with_target_analysis(false);
        let report = CriticalConstraintAnalysisPipeline::new(&cp_solver)
            .with_perturbation_adapter(&adapter)
            .analyze_with_baseline_values(
                &snapshot,
                BTreeMap::from([(StableVariableId::from("x"), 10)]),
                Some(10.0),
                true,
                None,
                &options,
            )
            .expect("pipeline report");
        assert_eq!(calls.get(), 1);
        assert_eq!(
            report
                .effectiveness
                .as_ref()
                .and_then(|entry| entry.entries.first())
                .map(|entry| entry.status),
            Some(AnalysisStatus::Unsupported)
        );
        report.validate().expect("valid pipeline report");

        let without_adapter = CriticalConstraintAnalysisPipeline::new(&cp_solver)
            .analyze_with_baseline_values(
                &snapshot,
                BTreeMap::from([(StableVariableId::from("x"), 10)]),
                Some(10.0),
                true,
                None,
                &options,
            )
            .expect("pipeline report without adapter");
        assert!(without_adapter
            .unavailable_reasons
            .iter()
            .any(|reason| reason.contains("no perturbation adapter")));
        without_adapter.validate().expect("valid pipeline report");
    }

    #[test]
    fn pipeline_rejects_baseline_objective_mismatch_before_adapters_run() {
        let cp_solver = FakeConstraintProgrammingSolver::new();
        let lp_solver = CountingLinearSolver::default();
        let snapshot = pipeline_snapshot();
        let options = CriticalConstraintAnalysisOptions::default()
            .with_perturbation_analysis(false)
            .with_target_analysis(false);
        let error = CriticalConstraintAnalysisPipeline::new(&cp_solver)
            .with_constraint_programming_fixed_integer_solver(&lp_solver)
            .analyze_with_baseline_values(
                &snapshot,
                BTreeMap::from([(StableVariableId::from("x"), 10)]),
                Some(9.0),
                true,
                None,
                &options,
            )
            .expect_err("mismatched baseline objective");
        assert!(format!("{error}").contains("baseline objective does not match"));
        assert_eq!(lp_solver.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn funnel_tiers_follow_activity_and_dual_and_keep_every_candidate() {
        let activity = activity_report(vec![
            activity_record("strong", Some("Capacity"), ActivityStatus::Active),
            activity_record("degenerate", Some("Capacity"), ActivityStatus::Active),
            activity_record("slack", Some("Slot"), ActivityStatus::Inactive),
            activity_record("semantic", Some("Slot"), ActivityStatus::SatisfiedWithoutSlackMetric),
        ]);
        let sensitivity = sensitivity_report(vec![
            ("strong", Some(12.5)),
            ("degenerate", Some(0.0)),
            ("slack", Some(0.0)),
            ("semantic", None),
        ]);
        let ranking = build_candidate_funnel(
            &activity,
            Some(&sensitivity),
            &CandidateFunnelConfig::default(),
        )
        .expect("funnel");

        assert_eq!(ranking.candidates.len(), 4, "Tier C must never be dropped");
        assert_eq!(ranking.tier_a, 1);
        assert_eq!(ranking.tier_b, 1);
        assert_eq!(ranking.tier_c, 2);
        assert_eq!(ranking.unclassified, 0);
        assert_eq!(ranking.candidates[0].constraint_id.0, "strong");
        assert_eq!(ranking.candidates[0].tier, CandidateTier::TierA);

        // A saturated shortlist still falls back to the lower tiers rather than losing them.
        // 短名单即使已被强候选占满，也必须回落到低层级而不是丢弃它们。
        assert_eq!(ranking.shortlist(1).len(), 1);
        assert_eq!(ranking.shortlist(10).len(), 4);
        ranking.validate().expect("valid ranking");
    }

    #[test]
    fn missing_dual_evidence_never_becomes_a_tier() {
        let activity = activity_report(vec![activity_record(
            "x",
            None,
            ActivityStatus::Active,
        )]);
        // No sensitivity report at all: tiering is impossible and must stay unclassified.
        // 完全没有灵敏度报告：无法分层，必须保持未分层。
        let ranking =
            build_candidate_funnel(&activity, None, &CandidateFunnelConfig::default())
                .expect("funnel");
        assert_eq!(ranking.unclassified, 1);
        assert_eq!(ranking.tier_a, 0);
        assert_eq!(ranking.candidates[0].tier, CandidateTier::Unclassified);
        assert!(!ranking.candidates[0].tier.is_shortlist_default());

        // A candidate whose dual is unknown stays unclassified even when it is active.
        // 对偶未知的候选即使 active 也保持未分层。
        let sensitivity = sensitivity_report(vec![("x", None)]);
        let ranking = build_candidate_funnel(
            &activity,
            Some(&sensitivity),
            &CandidateFunnelConfig::default(),
        )
        .expect("funnel");
        assert_eq!(ranking.candidates[0].tier, CandidateTier::Unclassified);
    }

    fn perturbation_report(
        id: &str,
        baseline: f64,
        points: Vec<(f64, Option<f64>, bool, bool)>,
    ) -> ConstraintPerturbationReport {
        ConstraintPerturbationReport {
            schema_version: "1.0".to_owned(),
            constraint_id: id.into(),
            baseline_objective: baseline,
            observations: points
                .into_iter()
                .map(|(delta, objective, proven, improved)| {
                    ConstraintPerturbationObservation {
                        delta,
                        perturbed_rhs: 1.0 + delta,
                        status: if proven {
                            AnalysisStatus::Reachable
                        } else {
                            AnalysisStatus::Unknown
                        },
                        termination_reason: crate::solver::report::TerminationReason::Completed,
                        objective,
                        objective_change: objective.map(|value| value - baseline),
                        improvement: objective.map(|value| value - baseline),
                        improved: proven.then_some(improved),
                        integer_pattern_changed: None,
                        solve_time: std::time::Duration::ZERO,
                        objective_proven: proven,
                    }
                })
                .collect(),
            removal: None,
            adaptive_outcome: None,
            status: AnalysisStatus::Reachable,
            solve_count: 1,
            unavailable_reason: None,
        }
    }

    #[test]
    fn effectiveness_ranking_uses_only_proven_observations() {
        let reports = vec![
            perturbation_report("a", 10.0, vec![(1.0, Some(12.0), true, true)]),
            // An unproven observation must not contribute an improvement.
            // 未证明的观测不得贡献改善值。
            perturbation_report("b", 10.0, vec![(1.0, Some(99.0), false, true)]),
        ];
        let ranking = build_effectiveness_ranking(10.0, None, &reports).expect("ranking");

        assert_eq!(ranking.entries.len(), 2);
        let a = ranking
            .entries
            .iter()
            .find(|entry| entry.constraint_id.0 == "a")
            .expect("a");
        assert_eq!(a.max_improvement, Some(2.0));
        assert_eq!(a.first_effective_delta, Some(1.0));
        assert_eq!(a.marginal_effect, Some(2.0));
        assert!(a.is_globally_effective());

        let b = ranking
            .entries
            .iter()
            .find(|entry| entry.constraint_id.0 == "b")
            .expect("b");
        assert_eq!(b.max_improvement, None);
        assert_eq!(b.first_effective_delta, None);
        assert!(!b.is_globally_effective());

        // Ranking order puts the proven improvement first.
        assert_eq!(ranking.entries[0].constraint_id.0, "a");
        assert_eq!(ranking.globally_effective().len(), 1);
        ranking.validate().expect("valid ranking");
    }

    #[test]
    fn effectiveness_ranking_never_claims_global_irrelevance() {
        // A constraint that is neither locally effective nor removal-effective must not be
        // labelled irrelevant anywhere in the public structure.
        // 既不局部有效、单删也无收益的约束，不得在公共结构中的任何位置被标记为无关。
        let reports = vec![perturbation_report("quiet", 10.0, vec![(1.0, Some(10.0), true, false)])];
        let ranking = build_effectiveness_ranking(10.0, None, &reports).expect("ranking");
        let entry = &ranking.entries[0];
        assert_eq!(entry.max_improvement, Some(0.0));
        assert!(!entry.is_globally_effective());
        assert!(ranking.globally_effective().is_empty());
        let debug = format!("{entry:?}");
        assert!(!debug.to_lowercase().contains("irrelevant"));
        assert!(!debug.to_lowercase().contains("globallyineffective"));
    }

    #[test]
    fn group_summary_reports_involved_active_and_blocking_instances() {
        let activity = activity_report(vec![
            activity_record("c1", Some("Cargo Transfer Time"), ActivityStatus::Active),
            activity_record("c2", Some("Cargo Transfer Time"), ActivityStatus::Inactive),
            activity_record("c3", Some("Cargo Transfer Time"), ActivityStatus::Active),
            activity_record("c4", Some("Airport Slot"), ActivityStatus::Active),
        ]);
        let ranking = build_candidate_funnel(
            &activity,
            Some(&sensitivity_report(vec![("c1", Some(3.0))])),
            &CandidateFunnelConfig::default(),
        )
        .expect("funnel");
        let summaries = summarize_groups(Some(&activity), None, None);
        assert_eq!(summaries.len(), 2);
        let cargo = summaries
            .iter()
            .find(|summary| summary.group.as_deref() == Some("Cargo Transfer Time"))
            .expect("cargo group");
        assert_eq!(cargo.involved_instances, 3);
        assert_eq!(cargo.active_instances, 2);
        assert_eq!(cargo.blocking_instances, 0);
        assert_eq!(ranking.tier_a, 1);
    }

    #[test]
    fn combined_status_keeps_the_weakest_stage() {
        assert_eq!(
            combine_status(AnalysisStatus::Reachable, Some(AnalysisStatus::Unknown)),
            AnalysisStatus::Unknown
        );
        assert_eq!(
            combine_status(AnalysisStatus::Unknown, Some(AnalysisStatus::Unsupported)),
            AnalysisStatus::Unsupported
        );
        // A missing stage must not strengthen the conclusion.
        assert_eq!(
            combine_status(AnalysisStatus::Reachable, None),
            AnalysisStatus::Reachable
        );
        assert_eq!(
            combine_status(AnalysisStatus::Unsupported, Some(AnalysisStatus::Reachable)),
            AnalysisStatus::Unsupported
        );
    }
}
