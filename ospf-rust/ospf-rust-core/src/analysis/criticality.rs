//! Multi-target criticality profile protocol.

use super::{
    AnalysisStatus, ConflictAnalysisOptions, ConflictAnalyzer, ConflictValidity, ObjectiveTarget,
};
use crate::error::Result;
use crate::model::constraint_programming::ConstraintProgrammingSnapshot;
use crate::solver::ConstraintProgrammingSolver;
use crate::solver::StableConstraintId;
use std::collections::{BTreeMap, BTreeSet};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CriticalityKind {
    LocalBottleneck,
    PersistentBottleneck,
    StructuralBottleneck,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct CriticalityObservation {
    pub target: ObjectiveTarget,
    pub status: AnalysisStatus,
    pub blocking_constraint_ids: BTreeSet<StableConstraintId>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct CriticalityProfile {
    pub observations: Vec<CriticalityObservation>,
    pub classifications: BTreeMap<StableConstraintId, BTreeSet<CriticalityKind>>,
    pub schema_version: String,
}

impl CriticalityProfile {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version.trim().is_empty() {
            return Err(super::invalid_analysis(
                "criticality profile schema must not be blank",
            ));
        }
        let mut targets = BTreeSet::new();
        for observation in &self.observations {
            observation.target.validate()?;
            if !targets.insert(observation.target.stable_id()) {
                return Err(super::invalid_analysis(
                    "criticality profile contains duplicate targets",
                ));
            }
            if observation
                .blocking_constraint_ids
                .iter()
                .any(|id| id.0.trim().is_empty())
            {
                return Err(super::invalid_analysis(
                    "criticality profile constraint ID must not be blank",
                ));
            }
        }
        if self.classifications.keys().any(|id| id.0.trim().is_empty()) {
            return Err(super::invalid_analysis(
                "criticality profile ID must not be blank",
            ));
        }
        if self.classifications != classifications_for_observations(&self.observations) {
            return Err(super::invalid_analysis(
                "criticality profile classifications do not match observations",
            ));
        }
        Ok(())
    }

    /// Number of observations with a proven unreachable target.
    pub fn proven_target_count(&self) -> usize {
        self.observations
            .iter()
            .filter(|observation| observation.status == AnalysisStatus::Unreachable)
            .count()
    }

    pub fn constraints(&self, kind: CriticalityKind) -> BTreeSet<StableConstraintId> {
        self.classifications
            .iter()
            .filter_map(|(id, kinds)| kinds.contains(&kind).then(|| id.clone()))
            .collect()
    }
}

pub fn build_criticality_profile(
    observations: Vec<CriticalityObservation>,
) -> Result<CriticalityProfile> {
    let classifications = classifications_for_observations(&observations);
    let profile = CriticalityProfile {
        observations,
        classifications,
        schema_version: "1.0".to_owned(),
    };
    profile.validate()?;
    Ok(profile)
}

fn classifications_for_observations(
    observations: &[CriticalityObservation],
) -> BTreeMap<StableConstraintId, BTreeSet<CriticalityKind>> {
    let proven: Vec<_> = observations
        .iter()
        .filter(|o| o.status == AnalysisStatus::Unreachable)
        .collect();
    let mut counts = BTreeMap::<StableConstraintId, usize>::new();
    for observation in &proven {
        for id in &observation.blocking_constraint_ids {
            *counts.entry(id.clone()).or_default() += 1;
        }
    }
    let total = proven.len();
    counts
        .into_iter()
        .map(|(id, count)| {
            let mut kinds = BTreeSet::new();
            if count == 1 {
                kinds.insert(CriticalityKind::LocalBottleneck);
            }
            if count > 1 {
                kinds.insert(CriticalityKind::PersistentBottleneck);
            }
            if total > 1 && count == total {
                kinds.insert(CriticalityKind::StructuralBottleneck);
            }
            (id, kinds)
        })
        .collect()
}

/// Execute target conflict analysis for every target and aggregate the verified evidence.
pub fn analyze_criticality_targets<S: ConstraintProgrammingSolver + ?Sized>(
    solver: &S,
    snapshot: &ConstraintProgrammingSnapshot,
    targets: &[ObjectiveTarget],
    options: &ConflictAnalysisOptions<'_>,
) -> Result<CriticalityProfile> {
    // Keep this batch facade aligned with MultiTargetAnalyzer: reject malformed target batches
    // before any backend solve, and use the same duplicate identity semantics.
    snapshot.validate_identity()?;
    options.validate()?;
    if targets.is_empty() {
        return Err(super::invalid_analysis(
            "multi-target analysis requires at least one target",
        ));
    }
    let mut seen = BTreeSet::new();
    for target in targets {
        target.validate()?;
        if !seen.insert(target.stable_id()) {
            return Err(super::invalid_analysis(
                "multi-target analysis contains duplicate targets",
            ));
        }
    }

    let mut observations = Vec::with_capacity(targets.len());
    for target in targets {
        let explanation = ConflictAnalyzer::new().analyze(solver, snapshot, target, options)?;
        // A status alone is insufficient for cross-target evidence. Partial conflicts and
        // unknown/unsupported reports must remain auditable in the observation, but their
        // members cannot be promoted into a criticality classification.
        let blocking_constraint_ids = (explanation.status == AnalysisStatus::Unreachable
            && explanation.validity == ConflictValidity::Verified
            && explanation.minimality_verified())
        .then(|| explanation.constraint_ids())
        .unwrap_or_default();
        observations.push(CriticalityObservation {
            target: target.clone(),
            status: explanation.status,
            blocking_constraint_ids,
        });
    }
    build_criticality_profile(observations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::constraint_programming::{
        ConstraintDefinition, ConstraintProgrammingConstraint, ConstraintProgrammingModel,
        IntegerDomain, IntegerExpression, IntegerObjective, IntegerRelation, IntegerVariable,
    };
    use crate::solver::constraint_programming::FakeConstraintProgrammingSolver;

    fn snapshot() -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("criticality-profile");
        model
            .register_variable(x.clone(), IntegerDomain::boolean())
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "upper",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x.clone()),
                    IntegerRelation::LessOrEqual,
                    0,
                ),
            ))
            .expect("constraint");
        model.set_objective(IntegerObjective::maximize(IntegerExpression::variable(x)));
        model.freeze().expect("snapshot")
    }

    #[test]
    fn unknown_targets_do_not_create_structural_evidence() {
        let target = ObjectiveTarget::at_least("objective", 1.0).unwrap();
        let mut first = BTreeSet::new();
        first.insert(StableConstraintId("a".into()));
        let profile = build_criticality_profile(vec![
            CriticalityObservation {
                target: target.clone(),
                status: AnalysisStatus::Unreachable,
                blocking_constraint_ids: first,
            },
            CriticalityObservation {
                target: ObjectiveTarget::at_least("objective-unknown", 2.0).unwrap(),
                status: AnalysisStatus::Unknown,
                blocking_constraint_ids: BTreeSet::new(),
            },
        ])
        .unwrap();
        assert!(profile
            .constraints(CriticalityKind::LocalBottleneck)
            .contains(&StableConstraintId("a".into())));
        assert!(profile
            .constraints(CriticalityKind::StructuralBottleneck)
            .is_empty());
        assert_eq!(profile.proven_target_count(), 1);
    }

    #[test]
    fn forged_classifications_are_rejected() {
        let target = ObjectiveTarget::at_least("objective", 1.0).unwrap();
        let mut blocking = BTreeSet::new();
        blocking.insert(StableConstraintId("a".into()));
        let mut forged = BTreeMap::new();
        let mut kinds = BTreeSet::new();
        kinds.insert(CriticalityKind::LocalBottleneck);
        forged.insert(StableConstraintId("other".into()), kinds);
        let profile = CriticalityProfile {
            observations: vec![CriticalityObservation {
                target,
                status: AnalysisStatus::Unreachable,
                blocking_constraint_ids: blocking,
            }],
            classifications: forged,
            schema_version: "1.0".to_owned(),
        };
        assert!(profile.validate().is_err());
    }

    #[test]
    fn batch_rejects_empty_and_duplicate_targets_like_multi_target_analyzer() {
        let solver = FakeConstraintProgrammingSolver::new();
        let snapshot = snapshot();
        let options = ConflictAnalysisOptions::default();

        let empty = analyze_criticality_targets(&solver, &snapshot, &[], &options)
            .expect_err("empty target batch");
        assert!(empty.to_string().contains("at least one target"));

        let target = ObjectiveTarget::at_least("objective", 1.0).unwrap();
        let duplicate =
            analyze_criticality_targets(&solver, &snapshot, &[target.clone(), target], &options)
                .expect_err("duplicate target batch");
        assert!(duplicate.to_string().contains("duplicate targets"));
    }

    #[test]
    fn partial_conflict_is_retained_but_not_promoted_to_criticality() {
        let solver = FakeConstraintProgrammingSolver::new();
        let target = ObjectiveTarget::at_least("objective", 1.0).unwrap();
        let options = ConflictAnalysisOptions {
            max_deletion_checks: 0,
            ..ConflictAnalysisOptions::default()
        };

        let profile = analyze_criticality_targets(&solver, &snapshot(), &[target], &options)
            .expect("partial conflict profile");
        assert_eq!(profile.observations.len(), 1);
        assert_eq!(profile.observations[0].status, AnalysisStatus::Unreachable);
        assert!(profile.observations[0].blocking_constraint_ids.is_empty());
        assert!(profile.classifications.is_empty());
    }
}
