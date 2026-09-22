//! Multi-target analysis orchestration / 多 target 分析编排。
//!
//! This module runs the existing target and conflict analyzers against one immutable CP
//! snapshot. Solver adapters remain responsible for real solving; Gurobi and SCIP facades enter
//! through the same `ConstraintProgrammingSolver` contract.
//! 本模块针对同一个不可变 CP snapshot 运行现有 target 与 conflict analyzer。真实求解仍由
//! solver adapter 负责；Gurobi 与 SCIP facade 通过同一个 `ConstraintProgrammingSolver` 契约接入。

use std::collections::{BTreeMap, BTreeSet};

use crate::error::Result;
use crate::model::constraint_programming::ConstraintProgrammingSnapshot;
use crate::solver::constraint_programming::ConstraintProgrammingSolveOptions;
use crate::solver::ConstraintProgrammingSolver;

use super::{
    alternative_improvement_plans_from_conflict, build_criticality_profile,
    AlternativeImprovementPlan, AnalysisStatus, ConflictAnalysisOptions, ConflictAnalyzer,
    ConflictExplanation, CriticalityObservation, CriticalityProfile, ObjectiveTarget,
    RelaxabilityPolicy, RelaxationCost, TargetFeasibilityReport,
};

/// Multi-target report schema / 多 target 报告 schema。
pub const MULTI_TARGET_ANALYSIS_REPORT_SCHEMA_VERSION: &str = "1.0";

/// Options shared by every target in one batch / 一批 target 共享的选项。
pub struct MultiTargetAnalysisOptions<'a> {
    /// CP solve options forwarded to target and conflict checks / 传递给 target 与 conflict 检查的 CP 选项。
    pub solve_options: ConstraintProgrammingSolveOptions<'a>,
    /// Maximum deletion and final verification solves per target / 每个 target 的删除与最终复验求解上限。
    pub max_deletion_checks: usize,
    /// Wall-clock budget per target / 每个 target 的墙钟预算。
    pub time_budget: Option<std::time::Duration>,
    /// Policy applied to generated relaxation plans / 应用于生成松弛方案的策略。
    pub relaxation_policy: RelaxabilityPolicy,
    /// Stable evidence ID to business cost mapping / 稳定 evidence ID 到业务成本的映射。
    pub relaxation_costs: BTreeMap<String, RelaxationCost>,
}

impl<'a> Default for MultiTargetAnalysisOptions<'a> {
    fn default() -> Self {
        Self {
            solve_options: ConstraintProgrammingSolveOptions::new(),
            max_deletion_checks: usize::MAX,
            time_budget: None,
            relaxation_policy: RelaxabilityPolicy::default(),
            relaxation_costs: BTreeMap::new(),
        }
    }
}

impl<'a> MultiTargetAnalysisOptions<'a> {
    /// Validate batch options before any backend solve / 在调用后端前校验批量选项。
    pub fn validate(&self) -> Result<()> {
        self.solve_options.validate()?;
        self.relaxation_policy.validate()?;
        if self.time_budget.is_some_and(|budget| budget.is_zero()) {
            return Err(super::invalid_analysis(
                "multi-target time budget must be positive when provided",
            ));
        }
        for cost in self.relaxation_costs.values() {
            cost.validate(&self.relaxation_policy)?;
        }
        Ok(())
    }

    /// Set one evidence cost and return the updated options / 设置一个 evidence 成本并返回更新后的选项。
    pub fn with_relaxation_cost(mut self, source: impl Into<String>, cost: RelaxationCost) -> Self {
        self.relaxation_costs.insert(source.into(), cost);
        self
    }

    fn conflict_options(&self) -> ConflictAnalysisOptions<'a> {
        ConflictAnalysisOptions {
            solve_options: self.solve_options,
            max_deletion_checks: self.max_deletion_checks,
            time_budget: self.time_budget,
        }
    }
}

/// Analysis result for one target / 一个 target 的分析结果。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct MultiTargetAnalysisResult {
    /// Requested target / 请求的 target。
    pub target: ObjectiveTarget,
    /// Target feasibility report / target 可行性报告。
    pub feasibility: TargetFeasibilityReport,
    /// Conflict/MUS report, including the target report / conflict/MUS 报告，其中包含 target 报告。
    pub conflict: ConflictExplanation,
    /// Policy-approved weighted correction recommendations / 通过策略的加权 correction 建议。
    pub correction_sets: Vec<super::CorrectionSet>,
    /// Ranked relaxation recommendations that require caller revalidation / 需要调用方重新验证的排序松弛建议。
    pub improvement_plans: Vec<AlternativeImprovementPlan>,
}

impl MultiTargetAnalysisResult {
    /// Whether this result contains a verified irreducible conflict / 是否包含已验证的不可约 conflict。
    pub fn has_verified_conflict(&self) -> bool {
        self.conflict.status == AnalysisStatus::Unreachable
            && self.conflict.minimality.is_verified()
            && self.conflict.validity == super::ConflictValidity::Verified
            && !self.conflict.members.is_empty()
    }

    /// Validate this result against a recommendation policy / 按推荐策略校验此结果。
    pub fn validate(&self, policy: &RelaxabilityPolicy) -> Result<()> {
        policy.validate()?;
        self.target.validate()?;
        self.feasibility.validate()?;
        self.conflict.validate()?;
        if self.feasibility != self.conflict.target_feasibility {
            return Err(super::invalid_analysis(
                "multi-target feasibility does not match conflict report",
            ));
        }
        if self.target != self.conflict.target || self.target != self.feasibility.target {
            return Err(super::invalid_analysis(
                "multi-target result contains inconsistent target identities",
            ));
        }
        if !self.has_verified_conflict()
            && (!self.correction_sets.is_empty() || !self.improvement_plans.is_empty())
        {
            return Err(super::invalid_analysis(
                "correction recommendations require a verified irreducible conflict",
            ));
        }
        for correction_set in &self.correction_sets {
            correction_set.validate(policy)?;
        }
        for plan in &self.improvement_plans {
            plan.validate(policy)?;
            if plan.target != self.target {
                return Err(super::invalid_analysis(
                    "improvement plan target does not match result target",
                ));
            }
        }
        if self.correction_sets.len() != self.improvement_plans.len() {
            return Err(super::invalid_analysis(
                "correction sets and improvement plans must have the same length",
            ));
        }
        for (index, (correction_set, plan)) in self
            .correction_sets
            .iter()
            .zip(&self.improvement_plans)
            .enumerate()
        {
            if plan.rank != index {
                return Err(super::invalid_analysis(
                    "improvement plan ranks must be dense and ordered",
                ));
            }
            if correction_set != &plan.correction_set {
                return Err(super::invalid_analysis(
                    "correction sets must match improvement plans",
                ));
            }
        }
        Ok(())
    }
}

/// Aggregated analysis over multiple objective targets / 多个 objective target 的聚合分析。
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct MultiTargetAnalysisReport {
    /// Report schema / 报告 schema。
    pub schema_version: String,
    /// One result in input target order / 按输入 target 顺序排列的结果。
    pub results: Vec<MultiTargetAnalysisResult>,
    /// Cross-target criticality profile / 跨 target criticality profile。
    pub criticality_profile: CriticalityProfile,
}

impl MultiTargetAnalysisReport {
    /// Validate all target results and the cross-target profile / 校验所有 target 结果及跨 target profile。
    pub fn validate(&self, policy: &RelaxabilityPolicy) -> Result<()> {
        if self.schema_version.trim().is_empty() {
            return Err(super::invalid_analysis(
                "multi-target report schema version must not be blank",
            ));
        }
        if self.results.is_empty() {
            return Err(super::invalid_analysis(
                "multi-target report must contain at least one result",
            ));
        }
        let mut targets = BTreeSet::new();
        for result in &self.results {
            if !targets.insert(result.target.stable_id()) {
                return Err(super::invalid_analysis(
                    "multi-target report contains duplicate targets",
                ));
            }
            result.validate(policy)?;
        }
        self.criticality_profile.validate()?;
        if self.criticality_profile.observations.len() != self.results.len()
            || self
                .criticality_profile
                .observations
                .iter()
                .zip(&self.results)
                .any(|(observation, result)| {
                    observation.target != result.target
                        || observation.status != result.feasibility.status
                })
        {
            return Err(super::invalid_analysis(
                "criticality observations do not match multi-target results",
            ));
        }
        let expected_observations = self
            .results
            .iter()
            .map(|result| CriticalityObservation {
                target: result.target.clone(),
                status: result.feasibility.status,
                blocking_constraint_ids: result
                    .has_verified_conflict()
                    .then(|| result.conflict.constraint_ids())
                    .unwrap_or_default(),
            })
            .collect::<Vec<_>>();
        if self.criticality_profile.observations != expected_observations {
            return Err(super::invalid_analysis(
                "criticality observations contain inconsistent evidence",
            ));
        }
        let expected_classifications = super::build_criticality_profile(expected_observations)?
            .classifications;
        if self.criticality_profile.classifications != expected_classifications {
            return Err(super::invalid_analysis(
                "criticality classifications do not match multi-target results",
            ));
        }
        Ok(())
    }

    /// Return only results with verified target conflicts / 仅返回具有已验证 target conflict 的结果。
    pub fn verified_conflicts(&self) -> Vec<&ConflictExplanation> {
        self.results
            .iter()
            .filter(|result| result.has_verified_conflict())
            .map(|result| &result.conflict)
            .collect()
    }

    /// Return all recommended plans in rank order for each target / 按每个 target 的 rank 返回所有推荐方案。
    pub fn improvement_plans(&self) -> Vec<&AlternativeImprovementPlan> {
        self.results
            .iter()
            .filter(|result| result.has_verified_conflict())
            .flat_map(|result| result.improvement_plans.iter())
            .collect()
    }
}

/// Stateless multi-target analyzer / 无状态多 target 分析器。
#[derive(Debug, Clone, Copy, Default)]
pub struct MultiTargetAnalyzer;

impl MultiTargetAnalyzer {
    /// Create an analyzer / 创建分析器。
    pub const fn new() -> Self {
        Self
    }

    /// Analyze all targets using one real or test CP adapter / 使用真实或测试 CP adapter 分析全部 target。
    ///
    /// The input snapshot is never mutated. Targets are solved in input order so report ordering
    /// and backend usage are deterministic; each backend solve still uses its native adapter.
    /// 输入 snapshot 始终不会修改。target 按输入顺序执行，以保证报告顺序和后端调用确定；
    /// 每次求解仍使用传入的原生 adapter。
    pub fn analyze<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        targets: &[ObjectiveTarget],
        options: &MultiTargetAnalysisOptions<'_>,
    ) -> Result<MultiTargetAnalysisReport> {
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

        let conflict_options = options.conflict_options();
        let conflict_analyzer = ConflictAnalyzer::new();
        let mut results = Vec::with_capacity(targets.len());
        for target in targets {
            let conflict =
                conflict_analyzer.analyze(solver, snapshot, target, &conflict_options)?;
            let feasibility = conflict.target_feasibility.clone();
            let improvement_plans = alternative_improvement_plans_from_conflict(
                &conflict,
                &options.relaxation_costs,
                &options.relaxation_policy,
            )?;
            let correction_sets = improvement_plans
                .iter()
                .map(|plan| plan.correction_set.clone())
                .collect();
            results.push(MultiTargetAnalysisResult {
                target: target.clone(),
                feasibility,
                conflict,
                correction_sets,
                improvement_plans,
            });
        }

        let observations = results
            .iter()
            .map(|result| CriticalityObservation {
                target: result.target.clone(),
                status: result.feasibility.status,
                // Only a verified conflict is evidence for a criticality classification.
                // An unverified/partial conflict remains available in the per-target report,
                // but must not be promoted into the aggregate bottleneck profile.
                blocking_constraint_ids: result
                    .has_verified_conflict()
                    .then(|| result.conflict.constraint_ids())
                    .unwrap_or_default(),
            })
            .collect();
        let criticality_profile = build_criticality_profile(observations)?;
        let report = MultiTargetAnalysisReport {
            schema_version: MULTI_TARGET_ANALYSIS_REPORT_SCHEMA_VERSION.to_owned(),
            results,
            criticality_profile,
        };
        report.validate(&options.relaxation_policy)?;
        Ok(report)
    }

    /// Analyze targets with default options / 使用默认选项分析 targets。
    pub fn analyze_default<S: ConstraintProgrammingSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        targets: &[ObjectiveTarget],
    ) -> Result<MultiTargetAnalysisReport> {
        self.analyze(
            solver,
            snapshot,
            targets,
            &MultiTargetAnalysisOptions::default(),
        )
    }
}

/// Convenience batch entry point / 批量分析便捷入口。
pub fn analyze_targets<S: ConstraintProgrammingSolver + ?Sized>(
    solver: &S,
    snapshot: &ConstraintProgrammingSnapshot,
    targets: &[ObjectiveTarget],
    options: &MultiTargetAnalysisOptions<'_>,
) -> Result<MultiTargetAnalysisReport> {
    MultiTargetAnalyzer::new().analyze(solver, snapshot, targets, options)
}

/// Project a result's verified conflict into its weighted plans / 将结果中的已验证 conflict 投影为加权方案。
pub fn relaxation_plans_for_result(
    result: &MultiTargetAnalysisResult,
    costs: &BTreeMap<String, RelaxationCost>,
    policy: &RelaxabilityPolicy,
) -> Result<Vec<AlternativeImprovementPlan>> {
    // The returned plans carry candidate amounts; callers must revalidate them after application.
    // 返回的方案携带候选幅度；调用方应用后必须重新验证。
    if !result.has_verified_conflict() {
        policy.validate()?;
        return Ok(Vec::new());
    }
    alternative_improvement_plans_from_conflict(&result.conflict, costs, policy)
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
        let mut model = ConstraintProgrammingModel::new("multi-target-analysis");
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
    fn batch_runs_targets_and_builds_weighted_plans() {
        let target_unreachable = ObjectiveTarget::at_least("objective", 1.0).unwrap();
        let target_reachable = ObjectiveTarget::at_least("objective", 0.0).unwrap();
        let options = MultiTargetAnalysisOptions::default().with_relaxation_cost(
            "constraint:upper",
            RelaxationCost {
                weight: 2.0,
                relaxation: 3.0,
            },
        );
        let report = MultiTargetAnalyzer::new()
            .analyze(
                &FakeConstraintProgrammingSolver::new(),
                &snapshot(),
                &[target_unreachable.clone(), target_reachable.clone()],
                &options,
            )
            .expect("batch report");
        assert_eq!(report.results.len(), 2);
        assert_eq!(
            report.results[0].feasibility.status,
            AnalysisStatus::Unreachable
        );
        assert_eq!(
            report.results[1].feasibility.status,
            AnalysisStatus::Reachable
        );
        assert_eq!(report.results[0].improvement_plans.len(), 1);
        assert_eq!(
            report.results[0].improvement_plans[0]
                .correction_set
                .total_cost,
            6.0
        );
        assert_eq!(report.results[1].improvement_plans.len(), 0);
        assert!(report.verified_conflicts().len() == 1);
        assert!(report
            .criticality_profile
            .constraints(super::super::CriticalityKind::LocalBottleneck)
            .contains(&crate::solver::StableConstraintId::from("upper")));
    }

    #[test]
    fn result_validation_requires_dense_matching_recommendations() {
        let target = ObjectiveTarget::at_least("objective", 1.0).unwrap();
        let options = MultiTargetAnalysisOptions::default();
        let mut report = MultiTargetAnalyzer::new()
            .analyze(
                &FakeConstraintProgrammingSolver::new(),
                &snapshot(),
                &[target],
                &options,
            )
            .expect("batch report");

        report.results[0].improvement_plans[0].rank = 1;
        assert!(report.validate(&options.relaxation_policy).is_err());

        let mut report = MultiTargetAnalyzer::new()
            .analyze(
                &FakeConstraintProgrammingSolver::new(),
                &snapshot(),
                &[ObjectiveTarget::at_least("objective", 1.0).unwrap()],
                &options,
            )
            .expect("batch report");
        report.results[0].correction_sets.clear();
        assert!(report.validate(&options.relaxation_policy).is_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn report_round_trips_through_json() {
        let report = MultiTargetAnalyzer::new()
            .analyze(
                &FakeConstraintProgrammingSolver::new(),
                &snapshot(),
                &[ObjectiveTarget::at_least("objective", 1.0).unwrap()],
                &MultiTargetAnalysisOptions::default(),
            )
            .expect("batch report");
        let encoded = serde_json::to_vec(&report).expect("report should serialize");
        let decoded: MultiTargetAnalysisReport =
            serde_json::from_slice(&encoded).expect("report should deserialize");
        assert_eq!(decoded, report);
    }

    #[test]
    fn batch_rejects_duplicate_targets_before_solving() {
        let target = ObjectiveTarget::at_least("objective", 1.0).unwrap();
        let error = MultiTargetAnalyzer::new()
            .analyze_default(
                &FakeConstraintProgrammingSolver::new(),
                &snapshot(),
                &[target.clone(), target],
            )
            .expect_err("duplicate targets");
        assert!(error.to_string().contains("duplicate targets"));
    }
}
