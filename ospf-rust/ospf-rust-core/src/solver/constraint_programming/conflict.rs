//! CP 假设冲突诊断 / CP assumption conflict diagnostics.
//!
//! 本模块只消费统一 `SolveReport`，不会引入 CP 专属终态。所有 conflict 成员都必须经过
//! 完整 CP snapshot 的不可行性复验；删除收缩无法完成时保留最后一个 verified seed。
//! This module consumes only the unified `SolveReport` and introduces no CP-specific terminal.
//! Every conflict member is checked against the complete CP snapshot, and the last verified seed
//! is retained when deletion shrinking cannot finish.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use crate::error::{CoreError, Result, SolverError};
use crate::model::constraint_programming::ConstraintProgrammingSnapshot;
use crate::solver::{
    ConstraintProgrammingAssumption, ConstraintProgrammingSession,
    ConstraintProgrammingSolveOptions, InfeasibilityEvidence, InfeasibilityEvidenceMember,
    InfeasibilityEvidenceSource, InfeasibilityMinimality, ProblemStatus, ProofCompleteness,
    ProofReliability, SolveIssue, SolveReport,
};

/// 返回一个 CP snapshot 可以贡献给冲突解释的全部稳定成员 / Return all stable members that a
/// CP snapshot can contribute to a conflict explanation.
///
/// 该集合是完整模型证据，而不是最小冲突集合。只有删除收缩完成后，调用方才可以把
/// `minimality` 标记为 `Irreducible`。
/// This is complete-model evidence rather than a minimal conflict. Callers may use
/// `Irreducible` only after deletion shrinking has completed.
pub(crate) fn snapshot_members(
    snapshot: &ConstraintProgrammingSnapshot,
) -> BTreeSet<InfeasibilityEvidenceMember> {
    let mut members = BTreeSet::new();
    for constraint in &snapshot.constraints {
        members.insert(InfeasibilityEvidenceMember::Constraint(
            constraint.id.0.clone(),
        ));
    }
    for variable in &snapshot.variables {
        let id = variable.variable.stable_id.0.clone();
        members.insert(InfeasibilityEvidenceMember::Domain(id.clone()));
        members.insert(InfeasibilityEvidenceMember::LowerBound(id.clone()));
        members.insert(InfeasibilityEvidenceMember::UpperBound(id));
    }
    for interval in &snapshot.intervals {
        members.insert(InfeasibilityEvidenceMember::Interval(
            interval.interval.id.0.clone(),
        ));
    }
    members
}

fn member_constraint_ids(members: &BTreeSet<InfeasibilityEvidenceMember>) -> BTreeSet<String> {
    members
        .iter()
        .map(|member| match member {
            InfeasibilityEvidenceMember::Constraint(id) => format!("constraint/{id}"),
            InfeasibilityEvidenceMember::LowerBound(id) => format!("lower-bound/{id}"),
            InfeasibilityEvidenceMember::UpperBound(id) => format!("upper-bound/{id}"),
            InfeasibilityEvidenceMember::Domain(id) => format!("domain/{id}"),
            InfeasibilityEvidenceMember::Interval(id) => format!("interval/{id}"),
            InfeasibilityEvidenceMember::Assumption(id) => format!("assumption/{id}"),
        })
        .collect()
}

fn base_model_conflict(
    snapshot: &ConstraintProgrammingSnapshot,
    computation_time: Duration,
) -> ConstraintProgrammingConflict {
    let members = snapshot_members(snapshot);
    ConstraintProgrammingConflict {
        source: InfeasibilityEvidenceSource::RebuildVerification,
        reliability: ProofReliability::Exact,
        completeness: ProofCompleteness::Complete,
        constraint_ids: member_constraint_ids(&members),
        members,
        minimality: InfeasibilityMinimality::NotChecked,
        computation_time,
        incomplete_reason: None,
    }
}

/// CP conflict 诊断结果 / CP conflict diagnostic result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintProgrammingConflict {
    /// 证据来源 / Evidence source.
    pub source: InfeasibilityEvidenceSource,
    /// 证据可靠性 / Evidence reliability.
    pub reliability: ProofReliability,
    /// 证据完整度 / Evidence completeness.
    pub completeness: ProofCompleteness,
    /// 带类型的 conflict 成员 / Typed conflict members.
    pub members: BTreeSet<InfeasibilityEvidenceMember>,
    /// 兼容旧字段的稳定成员 ID / Stable member IDs for the compatibility field.
    pub constraint_ids: BTreeSet<String>,
    /// 最小性状态 / Minimality state.
    pub minimality: InfeasibilityMinimality,
    /// 诊断耗时 / Diagnostic computation time.
    pub computation_time: Duration,
    /// 未完成原因 / Reason why shrinking was incomplete.
    pub incomplete_reason: Option<String>,
}

impl ConstraintProgrammingConflict {
    /// 附加到统一报告 / Attach to a unified report.
    pub fn attach_to_report(self, report: &mut SolveReport<i64>) {
        if report
            .diagnostics
            .infeasibility_evidence
            .as_ref()
            .is_some_and(InfeasibilityEvidence::is_authoritative)
        {
            report.diagnostics.issues.push(SolveIssue::new(
                "CpConflictEvidencePreserved",
                "authoritative infeasibility evidence was preserved; CP fallback members were not merged",
            ));
            return;
        }
        if report.diagnostics.infeasibility_evidence.is_some() {
            report.diagnostics.issues.push(SolveIssue::new(
                "CpConflictEvidenceReplaced",
                "non-authoritative infeasibility evidence was replaced by verified CP conflict evidence",
            ));
        }
        report.diagnostics.infeasibility_evidence = Some(InfeasibilityEvidence {
            source: self.source,
            reliability: self.reliability,
            completeness: self.completeness,
            constraint_ids: self.constraint_ids,
            members: self.members,
            minimality: self.minimality,
            computation_time: self.computation_time,
            unavailable_reason: self.incomplete_reason,
        });
    }
}

/// 计算 CP conflict / Compute a CP conflict.
///
/// `full_report` 必须是完整 assumptions 的结果。只有 base model 和完整 assumptions
/// 都有 verified infeasibility proof 时，函数才会生成 conflict seed。/
/// `full_report` must be the result for the complete assumption set. A conflict seed is generated
/// only when both the base model and the complete assumptions have verified infeasibility proofs.
pub(crate) fn compute_constraint_programming_conflict(
    session: &mut dyn ConstraintProgrammingSession,
    snapshot: &ConstraintProgrammingSnapshot,
    assumptions: &[ConstraintProgrammingAssumption],
    full_report: &SolveReport<i64>,
    solve_options: &ConstraintProgrammingSolveOptions<'_>,
    conflict_options: &ConstraintProgrammingSolveOptions<'_>,
) -> Result<ConstraintProgrammingConflict> {
    snapshot.validate_identity()?;
    let started = Instant::now();
    if assumptions.is_empty() && full_report.problem_status == ProblemStatus::Infeasible {
        crate::solver::require_infeasibility_certificate(full_report)?;
        return Ok(base_model_conflict(snapshot, started.elapsed()));
    }

    if !assumptions.is_empty() {
        let base_report = session.solve_with_assumptions(&[], solve_options)?;
        if base_report.problem_status == ProblemStatus::Infeasible {
            crate::solver::require_infeasibility_certificate(&base_report)?;
            return Ok(base_model_conflict(snapshot, started.elapsed()));
        }
    }

    crate::solver::require_infeasibility_certificate(full_report)?;
    let mut ordered = assumptions
        .iter()
        .cloned()
        .map(|assumption| (assumption.stable_id(), assumption))
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.0.cmp(&right.0));
    let mut active = ordered;
    let mut resolves = 0usize;
    let mut minimality = if conflict_options.shrink_conflict {
        InfeasibilityMinimality::Partial
    } else {
        InfeasibilityMinimality::NotChecked
    };
    let mut incomplete_reason = None;

    if conflict_options.shrink_conflict {
        let mut index = 0usize;
        while index < active.len() {
            if conflict_options
                .cancellation_handle
                .is_some_and(crate::solver::SolveHandle::is_cancelled)
            {
                incomplete_reason = Some("conflict shrinking was cancelled".to_owned());
                break;
            }
            if resolves >= conflict_options.max_conflict_resolves {
                incomplete_reason =
                    Some("conflict shrinking resolve budget was exhausted".to_owned());
                break;
            }
            let candidate = active
                .iter()
                .enumerate()
                .filter(|(candidate_index, _)| *candidate_index != index)
                .map(|(_, (_, assumption))| assumption.clone())
                .collect::<Vec<_>>();
            resolves = resolves.saturating_add(1);
            let candidate_report = match session.solve_with_assumptions(&candidate, solve_options) {
                Ok(report) => report,
                Err(error) => {
                    incomplete_reason = Some(format!(
                        "conflict shrinking stopped after backend failure: {error}"
                    ));
                    break;
                }
            };
            if candidate_report.problem_status == ProblemStatus::Infeasible
                && crate::solver::require_infeasibility_certificate(&candidate_report).is_ok()
            {
                active.remove(index);
            } else {
                index = index.saturating_add(1);
            }
        }
        if incomplete_reason.is_none() {
            minimality = InfeasibilityMinimality::Irreducible;
        }
    }

    let members = active
        .iter()
        .map(|(id, _)| InfeasibilityEvidenceMember::Assumption(id.0.clone()))
        .collect::<BTreeSet<_>>();
    let constraint_ids = active
        .iter()
        .map(|(id, _)| format!("assumption/{}", id.0))
        .collect::<BTreeSet<_>>();
    Ok(ConstraintProgrammingConflict {
        source: InfeasibilityEvidenceSource::RebuildVerification,
        reliability: ProofReliability::Exact,
        completeness: if incomplete_reason.is_some() {
            ProofCompleteness::Partial
        } else {
            ProofCompleteness::Complete
        },
        members,
        constraint_ids,
        minimality,
        computation_time: started.elapsed(),
        incomplete_reason,
    })
}

/// 记录 conflict 诊断不可用 / Record unavailable conflict diagnostics.
pub(crate) fn attach_unavailable_conflict(report: &mut SolveReport<i64>, reason: String) {
    if report.diagnostics.infeasibility_evidence.is_none() {
        report.diagnostics.infeasibility_evidence = Some(InfeasibilityEvidence {
            source: InfeasibilityEvidenceSource::Unavailable,
            reliability: ProofReliability::Unknown,
            completeness: ProofCompleteness::Unavailable,
            constraint_ids: BTreeSet::new(),
            members: BTreeSet::new(),
            minimality: InfeasibilityMinimality::NotChecked,
            computation_time: Duration::ZERO,
            unavailable_reason: Some(reason.clone()),
        });
    }
    report
        .diagnostics
        .issues
        .push(SolveIssue::new("CpConflictDiagnosticsUnavailable", reason));
}

/// 检查冲突输入的稳定身份 / Check stable identities of conflict inputs.
pub(crate) fn validate_conflict_assumption_ids(
    assumptions: &[ConstraintProgrammingAssumption],
) -> Result<()> {
    let mut ids = BTreeSet::new();
    for assumption in assumptions {
        if !ids.insert(assumption.stable_id()) {
            return Err(CoreError::Solver(SolverError::InvalidInput(
                "duplicate CP assumption identity".to_owned(),
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{ProofCompleteness, SolveProof, TerminationReason};

    fn conflict() -> ConstraintProgrammingConflict {
        ConstraintProgrammingConflict {
            source: InfeasibilityEvidenceSource::RebuildVerification,
            reliability: ProofReliability::Exact,
            completeness: ProofCompleteness::Complete,
            members: BTreeSet::from([InfeasibilityEvidenceMember::Assumption("a".to_owned())]),
            constraint_ids: BTreeSet::from(["assumption/a".to_owned()]),
            minimality: InfeasibilityMinimality::Irreducible,
            computation_time: Duration::ZERO,
            incomplete_reason: None,
        }
    }

    fn infeasible_report() -> SolveReport<i64> {
        SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
            .proof(SolveProof::infeasibility())
            .build()
            .expect("infeasible report")
    }

    #[test]
    fn authoritative_native_evidence_is_not_overwritten_by_cp_fallback() {
        let mut report = infeasible_report();
        report.diagnostics.infeasibility_evidence = Some(InfeasibilityEvidence {
            source: InfeasibilityEvidenceSource::NativeIis,
            reliability: ProofReliability::Reliable,
            completeness: ProofCompleteness::Complete,
            constraint_ids: BTreeSet::from(["native/c1".to_owned()]),
            members: BTreeSet::new(),
            minimality: InfeasibilityMinimality::Irreducible,
            computation_time: Duration::ZERO,
            unavailable_reason: None,
        });

        conflict().attach_to_report(&mut report);

        let evidence = report
            .diagnostics
            .infeasibility_evidence
            .expect("native evidence");
        assert_eq!(evidence.source, InfeasibilityEvidenceSource::NativeIis);
        assert!(
            report
                .diagnostics
                .issues
                .iter()
                .any(|issue| { issue.code == "CpConflictEvidencePreserved" })
        );
    }

    #[test]
    fn unavailable_evidence_is_replaced_by_verified_cp_fallback() {
        let mut report = infeasible_report();
        report.diagnostics.infeasibility_evidence = Some(InfeasibilityEvidence {
            source: InfeasibilityEvidenceSource::Unavailable,
            reliability: ProofReliability::Unknown,
            completeness: ProofCompleteness::Unavailable,
            constraint_ids: BTreeSet::new(),
            members: BTreeSet::new(),
            minimality: InfeasibilityMinimality::NotChecked,
            computation_time: Duration::ZERO,
            unavailable_reason: Some("native conflict unavailable".to_owned()),
        });

        conflict().attach_to_report(&mut report);

        let evidence = report
            .diagnostics
            .infeasibility_evidence
            .expect("verified fallback evidence");
        assert_eq!(
            evidence.source,
            InfeasibilityEvidenceSource::RebuildVerification
        );
        assert_eq!(evidence.reliability, ProofReliability::Exact);
        assert!(
            report
                .diagnostics
                .issues
                .iter()
                .any(|issue| { issue.code == "CpConflictEvidenceReplaced" })
        );
    }
}
