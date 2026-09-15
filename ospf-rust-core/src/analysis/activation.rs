//! Diagnostic activation and assumption protocol / 诊断激活与假设协议。
//!
//! 本模块实现计划事项 J：把"原始证据是否激活"表达为一等协议，并在后端能力不同的情况下
//! 选择原生 core、assumption 求解或重复求解回退，同时保证任何后端证据都能回映为原始
//! `DiagnosticSource`。
//! This module implements plan item J: it makes "is this original evidence active?" a
//! first-class protocol, routes across native cores, assumption solving, and repeated-solve
//! fallback according to declared backend capability, and guarantees that any backend evidence
//! can be remapped back to an original [`DiagnosticSource`].

use std::collections::BTreeSet;

use crate::model::constraint_programming::ConstraintProgrammingSnapshot;
use crate::solver::StableConstraintId;
use crate::solver::constraint_programming::{
    ConstraintProgrammingAssumption, ConstraintProgrammingAssumptionId,
};
use crate::solver::report::InfeasibilityEvidenceMember;

use super::{
    AnalysisCapability, BoundSide, CapabilityMatrix, CapabilitySupport, DiagnosticSource,
};

/// 激活协议 schema / Activation protocol schema.
pub const ACTIVATION_REPORT_SCHEMA_VERSION: &str = "1.0";

/// 激活状态 / Activation state.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivationState {
    /// 该原始证据参与派生模型 / The original evidence participates in the derived model.
    Activated,
    /// 该原始证据被移出派生模型 / The original evidence is removed from the derived model.
    Deactivated,
}

impl ActivationState {
    /// 是否为激活 / Whether this is an activated state.
    pub const fn is_activated(self) -> bool {
        matches!(self, Self::Activated)
    }
}

/// 一条原始证据的激活记录 / Activation record for one original evidence source.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticActivation {
    /// 原始证据来源 / Original evidence source.
    pub source: DiagnosticSource,
    /// 激活状态 / Activation state.
    pub state: ActivationState,
}

impl DiagnosticActivation {
    /// 构造激活记录 / Build an activation record.
    pub fn new(source: DiagnosticSource, state: ActivationState) -> Self {
        Self { source, state }
    }

    /// 激活一条原始证据 / Activate one original evidence source.
    pub fn activate(source: DiagnosticSource) -> Self {
        Self::new(source, ActivationState::Activated)
    }

    /// 停用一条原始证据 / Deactivate one original evidence source.
    pub fn deactivate(source: DiagnosticSource) -> Self {
        Self::new(source, ActivationState::Deactivated)
    }

    /// 稳定身份 / Stable identity.
    pub fn stable_id(&self) -> String {
        format!(
            "{}:{}",
            if self.state.is_activated() {
                "activated"
            } else {
                "deactivated"
            },
            self.source.stable_id()
        )
    }
}

/// 后端冲突提取层级 / Backend conflict-extraction tier.
///
/// 顺序即计划事项 K 的优先顺序：原生 unsat core 优先，其次 assumption 提取，最后重复求解。
/// The order is exactly plan item K's priority: native unsat core first, then assumption-based
/// extraction, then repeated solving.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConflictExtractionTier {
    /// 后端原生 unsat core / Native backend unsat core.
    NativeUnsatCore,
    /// 基于 assumption 的框架提取 / Framework extraction over assumptions.
    AssumptionExtraction,
    /// 重复求解回退 / Repeated-solving fallback.
    RepeatedSolving,
    /// 没有任何可用路径 / No usable path is declared.
    Unavailable,
}

impl ConflictExtractionTier {
    /// 该层级是否能在不重复求解的情况下给出冲突 / Whether this tier avoids repeated solving.
    pub const fn is_native(self) -> bool {
        matches!(self, Self::NativeUnsatCore)
    }

    /// 该层级是否可用 / Whether the tier is usable.
    pub const fn is_available(self) -> bool {
        !matches!(self, Self::Unavailable)
    }

    /// 稳定的层级名称 / Stable tier name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NativeUnsatCore => "native-unsat-core",
            Self::AssumptionExtraction => "assumption-extraction",
            Self::RepeatedSolving => "repeated-solving",
            Self::Unavailable => "unavailable",
        }
    }

    /// 依据能力矩阵选择层级 / Select a tier from a capability matrix.
    ///
    /// 能力必须来自后端声明，而不是框架假定：未声明的能力一律不得被当作可用。
    /// Capability must come from the backend declaration rather than a framework assumption:
    /// an undeclared capability is never treated as available.
    pub fn select(matrix: &CapabilityMatrix) -> Self {
        if matrix.is_native(AnalysisCapability::NativeUnsatCore)
            || matrix.support(AnalysisCapability::NativeUnsatCore) == CapabilitySupport::Supported
        {
            return Self::NativeUnsatCore;
        }
        if matrix.is_native(AnalysisCapability::AssumptionSolving)
            || matrix.support(AnalysisCapability::AssumptionSolving) == CapabilitySupport::Supported
        {
            return Self::AssumptionExtraction;
        }
        if matrix.fallback_satisfaction_only
            && matrix.support(AnalysisCapability::Conflict) != CapabilitySupport::Unsupported
        {
            return Self::RepeatedSolving;
        }
        Self::Unavailable
    }
}

/// 一次求解中激活的原始证据集合 / The set of original evidence active in one solve.
///
/// 该集合是框架层的求解请求描述，独立于任何后端表示；后端只看到派生模型或 assumption，
/// 公开报告只看到这里的原始 `DiagnosticSource`。
/// This set describes a framework-level solve request independently of any backend
/// representation. The backend only sees a derived model or assumptions, while public reports
/// only ever see the original [`DiagnosticSource`] values held here.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiagnosticActivationSet {
    entries: Vec<DiagnosticActivation>,
}

impl DiagnosticActivationSet {
    /// 空集合 / An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// 从快照构造：所有原始约束与固定背景证据均激活。
    /// Build from a snapshot: every original constraint and fixed-background source is active.
    pub fn from_snapshot(snapshot: &ConstraintProgrammingSnapshot) -> Self {
        let mut set = Self::new();
        for constraint in &snapshot.constraints {
            set.insert(DiagnosticActivation::activate(
                DiagnosticSource::Constraint {
                    id: constraint.id.clone(),
                },
            ));
        }
        for variable in &snapshot.variables {
            let variable_id = variable.variable.stable_id.clone();
            let _ = set.activate_source(DiagnosticSource::VariableLowerBound {
                variable_id: variable_id.clone(),
            });
            let _ = set.activate_source(DiagnosticSource::VariableUpperBound {
                variable_id: variable_id.clone(),
            });
            if matches!(
                variable.domain,
                crate::model::constraint_programming::IntegerDomain::Values(_)
            ) {
                let _ = set.activate_source(DiagnosticSource::SparseDomain { variable_id });
            }
        }
        set
    }

    /// 记录一条激活记录（后者覆盖前者）/ Record an activation, overriding any earlier one.
    pub fn insert(&mut self, activation: DiagnosticActivation) {
        self.entries
            .retain(|existing| existing.source != activation.source);
        self.entries.push(activation);
    }

    /// 激活一条来源 / Activate a source.
    pub fn activate_source(&mut self, source: DiagnosticSource) -> crate::error::Result<()> {
        source.validate()?;
        self.insert(DiagnosticActivation::activate(source));
        Ok(())
    }

    /// 停用一条来源 / Deactivate a source.
    pub fn deactivate_source(&mut self, source: DiagnosticSource) -> crate::error::Result<()> {
        source.validate()?;
        self.insert(DiagnosticActivation::deactivate(source));
        Ok(())
    }

    /// 全部记录 / Every record.
    pub fn entries(&self) -> &[DiagnosticActivation] {
        &self.entries
    }

    /// 某个来源是否激活 / Whether a source is active.
    ///
    /// 未出现的来源按未激活处理：调用方必须先显式声明其证据全集。
    /// An absent source is inactive: callers must declare their evidence universe explicitly.
    pub fn is_active(&self, source: &DiagnosticSource) -> bool {
        self.entries
            .iter()
            .any(|entry| &entry.source == source && entry.state.is_activated())
    }

    /// 全部激活来源 / Every active source.
    pub fn active_sources(&self) -> Vec<DiagnosticSource> {
        self.entries
            .iter()
            .filter(|entry| entry.state.is_activated())
            .map(|entry| entry.source.clone())
            .collect()
    }

    /// 全部激活的原始约束 / Every active original constraint.
    pub fn active_constraints(&self) -> BTreeSet<StableConstraintId> {
        self.entries
            .iter()
            .filter(|entry| entry.state.is_activated())
            .filter_map(|entry| match &entry.source {
                DiagnosticSource::Constraint { id } => Some(id.clone()),
                _ => None,
            })
            .collect()
    }

    /// 集合是否为空 / Whether the set has no records.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 记录数量 / Number of records.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 把可表示的激活证据编码为后端 assumption / Encode representable active evidence as
    /// backend assumptions.
    ///
    /// 只有变量边界与稀疏域可以精确表达为 assumption。原始约束无法表达为 assumption，必须
    /// 通过派生模型（移除约束）表达，因此这里**不**为约束伪造 assumption。
    /// Only variable bounds and sparse domains map exactly onto assumptions. An original
    /// constraint cannot be expressed as an assumption and must be expressed by deriving a model
    /// without it, so this method deliberately does **not** fabricate assumptions for constraints.
    pub fn to_assumptions(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
    ) -> Vec<ConstraintProgrammingAssumption> {
        let mut assumptions = Vec::new();
        for entry in &self.entries {
            if !entry.state.is_activated() {
                continue;
            }
            let variable_id = match &entry.source {
                DiagnosticSource::VariableLowerBound { variable_id }
                | DiagnosticSource::VariableUpperBound { variable_id } => variable_id,
                DiagnosticSource::VariableBound { bound } => &bound.variable_id,
                DiagnosticSource::SparseDomain { variable_id } => variable_id,
                _ => continue,
            };
            let Some(variable) = snapshot.variable(variable_id) else {
                continue;
            };
            if let DiagnosticSource::SparseDomain { .. } = &entry.source {
                if let crate::model::constraint_programming::IntegerDomain::Values(values) =
                    &variable.domain
                {
                    assumptions.push(ConstraintProgrammingAssumption::SparseDomain(
                        variable.variable.clone(),
                        crate::model::constraint_programming::IntegerDomain::Values(values.clone()),
                    ));
                }
                continue;
            }
            let side = match &entry.source {
                DiagnosticSource::VariableLowerBound { .. } => BoundSide::Lower,
                DiagnosticSource::VariableUpperBound { .. } => BoundSide::Upper,
                DiagnosticSource::VariableBound { bound } => bound.side,
                _ => continue,
            };
            let value = match side {
                BoundSide::Lower => lower_bound(&variable.domain),
                BoundSide::Upper => upper_bound(&variable.domain),
            };
            let Some(value) = value else {
                continue;
            };
            assumptions.push(match side {
                BoundSide::Lower => {
                    ConstraintProgrammingAssumption::LowerBound(variable.variable.clone(), value)
                }
                BoundSide::Upper => {
                    ConstraintProgrammingAssumption::UpperBound(variable.variable.clone(), value)
                }
            });
        }
        assumptions
    }

    /// 把后端返回的 assumption 身份回映为原始证据 / Remap a backend assumption identity back
    /// to original evidence.
    ///
    /// 回映只在本次激活集合内进行；后端返回的任何未知身份都会被丢弃，因此不会把
    /// solver 生成的辅助元素泄露到公开证据中。
    /// Remapping is restricted to this activation set; any unrecognised backend identity is
    /// dropped, so a solver-generated auxiliary element can never leak into public evidence.
    pub fn source_of_assumption(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        assumption_id: &ConstraintProgrammingAssumptionId,
    ) -> Option<DiagnosticSource> {
        self.to_assumptions(snapshot)
            .into_iter()
            .find(|assumption| &assumption.stable_id() == assumption_id)
            .and_then(|assumption| match assumption {
                ConstraintProgrammingAssumption::LowerBound(variable, _) => {
                    Some(DiagnosticSource::VariableLowerBound {
                        variable_id: variable.stable_id,
                    })
                }
                ConstraintProgrammingAssumption::UpperBound(variable, _) => {
                    Some(DiagnosticSource::VariableUpperBound {
                        variable_id: variable.stable_id,
                    })
                }
                ConstraintProgrammingAssumption::SparseDomain(variable, _) => {
                    Some(DiagnosticSource::SparseDomain {
                        variable_id: variable.stable_id,
                    })
                }
                _ => None,
            })
    }

    /// 把后端返回的不可行证据成员回映为原始证据 / Remap backend infeasibility evidence back
    /// to original evidence.
    ///
    /// 回映以**本激活集合**为唯一来源：先枚举激活的原始证据，再检查后端证据是否指向其中
    /// 之一。后端返回的任何未知成员都会被丢弃，因此 solver 生成的辅助元素不可能出现在
    /// 结果里。
    /// Remapping is driven exclusively by this activation set: active original evidence is
    /// enumerated first, then matched against the backend members. Any unrecognised backend
    /// member is dropped, so a solver-generated auxiliary element can never appear in the
    /// result.
    pub fn remap_backend_evidence(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        members: &BTreeSet<InfeasibilityEvidenceMember>,
    ) -> Vec<DiagnosticSource> {
        self.active_sources()
            .into_iter()
            .filter(|source| {
                source.as_infeasibility_member().is_some_and(|member| {
                    members.contains(&member)
                        || self
                            .to_assumptions(snapshot)
                            .into_iter()
                            .find(|assumption| {
                                self.source_of_assumption(snapshot, &assumption.stable_id())
                                    .is_some_and(|mapped| mapped.stable_id() == source.stable_id())
                            })
                            .is_some_and(|assumption| {
                                members.contains(&InfeasibilityEvidenceMember::Assumption(
                                    assumption.stable_id().0,
                                ))
                            })
                })
            })
            .collect()
    }
}

fn lower_bound(domain: &crate::model::constraint_programming::IntegerDomain) -> Option<i64> {
    match domain {
        crate::model::constraint_programming::IntegerDomain::Range { lower, .. } => Some(*lower),
        crate::model::constraint_programming::IntegerDomain::Values(values) => {
            values.iter().copied().min()
        }
    }
}

fn upper_bound(domain: &crate::model::constraint_programming::IntegerDomain) -> Option<i64> {
    match domain {
        crate::model::constraint_programming::IntegerDomain::Range { upper, .. } => Some(*upper),
        crate::model::constraint_programming::IntegerDomain::Values(values) => {
            values.iter().copied().max()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::constraint_programming::{
        ConstraintDefinition, ConstraintProgrammingConstraint, ConstraintProgrammingModel,
        IntegerDomain, IntegerExpression, IntegerObjective, IntegerRelation, IntegerVariable,
    };

    fn snapshot() -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("activation");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 5).expect("domain"))
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "capacity",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x.clone()),
                    IntegerRelation::LessOrEqual,
                    5,
                ),
            ))
            .expect("constraint");
        model.set_objective(IntegerObjective::maximize(IntegerExpression::variable(x)));
        model.freeze().expect("snapshot")
    }

    fn sparse_snapshot() -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("sparse-activation");
        model
            .register_variable(
                x.clone(),
                IntegerDomain::values([0, 2]).expect("sparse domain"),
            )
            .expect("variable");
        model.set_objective(IntegerObjective::maximize(IntegerExpression::variable(x)));
        model.freeze().expect("snapshot")
    }

    #[test]
    fn activation_set_tracks_original_evidence_and_never_names_solver_rows() {
        let snapshot = snapshot();
        let mut set = DiagnosticActivationSet::from_snapshot(&snapshot);
        let constraint = DiagnosticSource::constraint("capacity").expect("source");
        assert!(set.is_active(&constraint));

        set.deactivate_source(constraint.clone()).expect("deactivate");
        assert!(!set.is_active(&constraint));
        assert!(set.active_constraints().is_empty());

        // Every recorded identity is an original-model identity.
        for entry in set.entries() {
            let id = entry.source.stable_id();
            assert!(!id.contains("row"), "activation leaked a solver row: {id}");
            assert!(!id.contains("column"), "activation leaked a solver column: {id}");
        }
    }

    #[test]
    fn constraints_never_become_fabricated_assumptions() {
        let snapshot = snapshot();
        let set = DiagnosticActivationSet::from_snapshot(&snapshot);
        let assumptions = set.to_assumptions(&snapshot);

        // The only constraint in the model must not appear as an assumption; only the two
        // variable bounds may.
        assert_eq!(assumptions.len(), 2);
        assert!(
            assumptions
                .iter()
                .all(|assumption| !assumption.stable_id().0.starts_with("literal/"))
        );
        assert!(
            assumptions
                .iter()
                .any(|assumption| assumption.stable_id().0.starts_with("lower-bound/"))
        );
        assert!(
            assumptions
                .iter()
                .any(|assumption| assumption.stable_id().0.starts_with("upper-bound/"))
        );
    }

    #[test]
    fn assumption_identities_remap_back_to_original_evidence() {
        let snapshot = snapshot();
        let set = DiagnosticActivationSet::from_snapshot(&snapshot);
        let assumptions = set.to_assumptions(&snapshot);
        let lower = assumptions
            .iter()
            .find(|assumption| assumption.stable_id().0.starts_with("lower-bound/"))
            .expect("lower bound assumption");
        let remapped = set
            .source_of_assumption(&snapshot, &lower.stable_id())
            .expect("remapped source");
        assert!(matches!(
            remapped,
            DiagnosticSource::VariableLowerBound { .. }
        ));
        assert!(remapped.stable_id().contains("variable"));

        // An unknown backend identity must be dropped rather than surfaced.
        let unknown = ConstraintProgrammingAssumptionId("equal/9:auxiliary:0".to_owned());
        assert!(set.source_of_assumption(&snapshot, &unknown).is_none());
    }

    #[test]
    fn sparse_domain_assumption_is_forwarded_and_remapped() {
        let snapshot = sparse_snapshot();
        let set = DiagnosticActivationSet::from_snapshot(&snapshot);
        let assumptions = set.to_assumptions(&snapshot);
        let sparse = assumptions
            .iter()
            .find(|assumption| {
                matches!(
                    assumption,
                    ConstraintProgrammingAssumption::SparseDomain(_, _)
                )
            })
            .expect("sparse-domain assumption");
        assert_eq!(sparse.stable_id().0, "sparse-domain/1:x");
        assert_eq!(
            set.source_of_assumption(&snapshot, &sparse.stable_id()),
            Some(DiagnosticSource::SparseDomain {
                variable_id: "x".into()
            })
        );
    }

    #[test]
    fn extraction_tier_follows_declared_backend_capability() {
        // Nothing declared: no tier may be claimed.
        let empty = CapabilityMatrix::default();
        assert_eq!(
            ConflictExtractionTier::select(&empty),
            ConflictExtractionTier::Unavailable
        );

        // A backend with native cores is preferred over assumption extraction.
        let native = CapabilityMatrix::default()
            .with_capability(AnalysisCapability::NativeUnsatCore, CapabilitySupport::Supported)
            .with_capability(
                AnalysisCapability::AssumptionSolving,
                CapabilitySupport::Supported,
            );
        assert_eq!(
            ConflictExtractionTier::select(&native),
            ConflictExtractionTier::NativeUnsatCore
        );
        assert!(ConflictExtractionTier::select(&native).is_native());

        // A satisfaction-only backend falls back to repeated solving.
        let mut fallback = CapabilityMatrix::default();
        fallback.fallback_satisfaction_only = true;
        fallback
            .analysis_capabilities
            .insert(AnalysisCapability::Conflict, CapabilitySupport::Conditional);
        assert_eq!(
            ConflictExtractionTier::select(&fallback),
            ConflictExtractionTier::RepeatedSolving
        );
        assert!(!ConflictExtractionTier::select(&fallback).is_native());
    }
}
