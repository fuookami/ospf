//! 固定整数 LP 局部有效性分析 / Fixed-integer LP local-effectiveness analysis.
//!
//! 本模块把 MILP incumbent 中的整数变量固定后生成一个连续 LP，并把该 LP 的行对偶
//! 映射回原始线性约束身份。派生模型和对偶向量只在分析器内部使用，公共报告不暴露
//! solver 行列或 lower 产生的辅助元素。
//! This module fixes the integer variables of a MILP incumbent, solves the resulting continuous
//! LP, and maps row duals back to original linear-constraint identities. The derived model and
//! dual vector stay inside the analyzer; public reports expose neither solver rows/columns nor
//! lowering-generated auxiliary elements.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{CoreError, Result, SolverError};
use crate::model::constraint_programming::ConstraintProgrammingSnapshot;
use crate::model::intermediate::LinearTriadModel;
use crate::solver::audit::{evaluate_linear_solution, linear_model_mapping};
use crate::solver::constraint_programming::mip::{
    lower_constraint_programming, MipLoweringResult, MipRowOrigin,
};
use crate::solver::fingerprint::{
    linear_model_fingerprint, solver_descriptor_fingerprint,
};
use crate::solver::report::{ProblemStatus, SolveReport, TerminationReason};
use crate::solver::{LinearSolver, StableConstraintId, StableVariableId};
use crate::variable::VariableType;

use super::{AnalysisStatus, ConstraintActivity, ConstraintActivityAnalyzer, ConstraintId};

/// 固定整数 LP 报告 schema 版本 / Fixed-integer LP report schema version.
pub const FIXED_INTEGER_LP_REPORT_SCHEMA_VERSION: &str = "1.0";

/// 返回线性模型的稳定原始约束身份 / Return stable original-constraint identities for a linear model.
pub fn linear_constraint_ids(model: &LinearTriadModel) -> Result<Vec<ConstraintId>> {
    Ok(linear_model_mapping(model)?
        .constraints_by_row
        .into_iter()
        .map(StableConstraintId)
        .collect())
}

/// 局部灵敏度的语义范围 / Semantic scope of local sensitivity.
///
/// 该范围只描述 incumbent 的整数模式保持不变时的连续 LP，不代表 MILP 全局影子价格。
/// This scope describes only the continuous LP under a fixed incumbent integer pattern; it is
/// not a global MILP shadow price.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SensitivityScope {
    /// 固定当前 incumbent 的整数模式 / Incumbent integer pattern fixed.
    #[default]
    FixedIntegerIncumbent,
}

/// 固定当前整数模式范围的兼容名称 / Compatibility name for the fixed incumbent scope.
pub type FixedIntegerIncumbentScope = SensitivityScope;

impl SensitivityScope {
    /// 返回稳定范围名称 / Return the stable scope name.
    pub const fn stable_name(self) -> &'static str {
        match self {
            Self::FixedIntegerIncumbent => "fixed_integer_incumbent",
        }
    }
}

/// 兼容性别名 / Compatibility alias.
pub type FixedIntegerLpScope = SensitivityScope;

/// 固定整数 LP 分析配置 / Fixed-integer LP analysis configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedIntegerLpSensitivityConfig {
    /// 校验 incumbent 整数性的绝对容差 / Absolute tolerance for incumbent integrality.
    pub integrality_tolerance: f64,
    /// 判断对偶是否为零的绝对容差 / Absolute tolerance for a zero dual.
    pub dual_tolerance: f64,
}

impl Default for FixedIntegerLpSensitivityConfig {
    fn default() -> Self {
        Self {
            integrality_tolerance: 1e-7,
            dual_tolerance: 1e-9,
        }
    }
}

impl FixedIntegerLpSensitivityConfig {
    /// 校验配置 / Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        if !self.integrality_tolerance.is_finite()
            || self.integrality_tolerance < 0.0
            || !self.dual_tolerance.is_finite()
            || self.dual_tolerance < 0.0
        {
            return Err(invalid_sensitivity(
                "integrality and dual tolerances must be finite and non-negative",
            ));
        }
        Ok(())
    }

    /// 设置整数性容差 / Set the integrality tolerance.
    pub const fn with_integrality_tolerance(mut self, tolerance: f64) -> Self {
        self.integrality_tolerance = tolerance;
        self
    }

    /// 设置零对偶容差 / Set the zero-dual tolerance.
    pub const fn with_dual_tolerance(mut self, tolerance: f64) -> Self {
        self.dual_tolerance = tolerance;
        self
    }
}

/// 固定整数后的连续 LP / Continuous LP derived from an incumbent.
///
/// 该类型保存可重用的派生模型和稳定变量值，但不会把 solver 列索引写入分析报告。
/// This type stores a reusable derived model and stable variable values without writing solver
/// column indices into analysis reports.
#[derive(Debug, Clone)]
pub struct FixedIntegerLpModel {
    model: LinearTriadModel,
    fixed_values: BTreeMap<StableVariableId, f64>,
    fixed_integer_ids: BTreeSet<StableVariableId>,
}

impl FixedIntegerLpModel {
    /// 从线性模型和 incumbent 构造派生 LP / Build a derived LP from a model and incumbent.
    pub fn from_incumbent(
        model: &LinearTriadModel,
        incumbent: &[f64],
        integrality_tolerance: f64,
    ) -> Result<Self> {
        if !integrality_tolerance.is_finite() || integrality_tolerance < 0.0 {
            return Err(invalid_sensitivity(
                "integrality tolerance must be finite and non-negative",
            ));
        }
        let mapping = linear_model_mapping(model)?;
        if incumbent.len() != model.num_variables() {
            return Err(invalid_sensitivity(format!(
                "incumbent length {} does not match variable count {}",
                incumbent.len(),
                model.num_variables()
            )));
        }
        if incumbent.iter().any(|value| !value.is_finite()) {
            return Err(invalid_sensitivity("incumbent contains a non-finite value"));
        }

        let mut fixed_values = BTreeMap::new();
        let mut fixed_integer_ids = BTreeSet::new();
        for (index, value) in incumbent.iter().copied().enumerate() {
            let variable_type = model
                .basic
                .var_types
                .get(index)
                .ok_or_else(|| invalid_sensitivity("variable type is missing"))?;
            if !variable_type.is_integer() {
                continue;
            }
            let nearest = value.round();
            if (value - nearest).abs() > integrality_tolerance {
                return Err(invalid_sensitivity(format!(
                    "integer incumbent at variable {} is not integral: {}",
                    index, value
                )));
            }
            let lower = model.basic.lb[index];
            let upper = model.basic.ub[index];
            if (lower.is_finite() && nearest + integrality_tolerance < lower)
                || (upper.is_finite() && nearest - integrality_tolerance > upper)
            {
                return Err(invalid_sensitivity(format!(
                    "integer incumbent at variable {} is outside its declared bounds",
                    index
                )));
            }
            let id = mapping
                .variables_by_column
                .get(index)
                .cloned()
                .ok_or_else(|| invalid_sensitivity("variable mapping is incomplete"))?;
            fixed_values.insert(id.clone(), nearest);
            fixed_integer_ids.insert(id);
        }

        // Relax every integer type before fixing its value so the derived problem is a true LP.
        // 在固定数值前先松弛所有整数类型，确保派生问题确实是 LP。
        let mut derived = model.linear_relaxed();
        for (index, value) in incumbent.iter().copied().enumerate() {
            if model
                .basic
                .var_types
                .get(index)
                .is_some_and(VariableType::is_integer)
            {
                derived
                    .basic
                    .set_bounds(index, value.round(), value.round());
            }
        }
        let derived = Self {
            model: derived,
            fixed_values,
            fixed_integer_ids,
        };
        derived.validate()?;
        Ok(derived)
    }

    /// 返回派生线性模型 / Return the derived linear model.
    pub fn model(&self) -> &LinearTriadModel {
        &self.model
    }

    /// 返回固定的稳定变量值 / Return fixed values keyed by stable identity.
    pub fn fixed_values(&self) -> &BTreeMap<StableVariableId, f64> {
        &self.fixed_values
    }

    /// 返回固定整数变量身份 / Return stable identities of fixed integer variables.
    pub fn fixed_integer_ids(&self) -> &BTreeSet<StableVariableId> {
        &self.fixed_integer_ids
    }

    /// 返回固定整数变量数量 / Return the number of fixed integer variables.
    pub fn fixed_integer_count(&self) -> usize {
        self.fixed_integer_ids.len()
    }

    /// 校验派生模型仍是连续 LP / Validate that the derived model is a continuous LP.
    pub fn validate(&self) -> Result<()> {
        linear_model_mapping(&self.model)?;
        if self
            .model
            .basic
            .var_types
            .iter()
            .any(VariableType::is_integer)
        {
            return Err(invalid_sensitivity(
                "fixed-integer derived model still contains integer variables",
            ));
        }
        if self.fixed_values.len() != self.fixed_integer_ids.len()
            || self
                .fixed_values
                .keys()
                .any(|id| !self.fixed_integer_ids.contains(id))
        {
            return Err(invalid_sensitivity(
                "fixed-integer value map is inconsistent with fixed IDs",
            ));
        }
        Ok(())
    }
}

/// 单条约束的局部灵敏度 / Local sensitivity for one constraint.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct LocalConstraintSensitivity {
    /// 原始约束稳定身份 / Stable original-constraint identity.
    pub constraint_id: ConstraintId,
    /// LP 对偶值；无可用证书时为空 / LP dual; absent when no usable certificate exists.
    pub dual_value: Option<f64>,
    /// 是否在当前固定整数模式下局部有效 / Whether effective for the fixed integer pattern.
    pub local_effective: Option<bool>,
    /// 分析状态 / Analysis status.
    pub status: AnalysisStatus,
    /// 语义范围 / Semantic scope.
    pub scope: SensitivityScope,
    /// 可选的当前活动性证据 / Optional current activity evidence.
    pub activity: Option<ConstraintActivity>,
}

/// 固定整数 LP 局部有效性报告 / Fixed-integer LP local-effectiveness report.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct LocalConstraintSensitivityReport {
    /// 报告 schema 版本 / Report schema version.
    pub schema_version: String,
    /// 灵敏度范围 / Sensitivity scope.
    pub scope: SensitivityScope,
    /// baseline 目标值 / Baseline objective value.
    pub baseline_objective: f64,
    /// 固定整数 LP 目标值 / Fixed-integer LP objective value.
    pub lp_objective: Option<f64>,
    /// 固定整数变量数量 / Number of fixed integer variables.
    pub fixed_integer_count: usize,
    /// 每条原始约束的局部证据 / Per-original-constraint local evidence.
    pub constraints: Vec<LocalConstraintSensitivity>,
    /// 整体分析状态 / Overall analysis status.
    pub status: AnalysisStatus,
    /// solver 终止原因 / Solver termination reason.
    pub termination_reason: Option<TerminationReason>,
    /// 失败或降级原因 / Failure or downgrade reason.
    pub unavailable_reason: Option<String>,
}

impl LocalConstraintSensitivityReport {
    /// 校验报告不变量 / Validate report invariants.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version.trim().is_empty()
            || !self.baseline_objective.is_finite()
            || self.constraints.iter().any(|constraint| {
                constraint.constraint_id.0.trim().is_empty()
                    || constraint
                        .dual_value
                        .is_some_and(|value| !value.is_finite())
            })
        {
            return Err(invalid_sensitivity("local sensitivity report is invalid"));
        }
        if let Some(value) = self.lp_objective
            && !value.is_finite()
        {
            return Err(invalid_sensitivity("LP objective must be finite"));
        }
        Ok(())
    }

    /// 返回某条约束的局部记录 / Find a local record by stable identity.
    pub fn constraint(&self, id: &ConstraintId) -> Option<&LocalConstraintSensitivity> {
        self.constraints
            .iter()
            .find(|constraint| &constraint.constraint_id == id)
    }

    /// 是否有至少一个有效对偶 / Whether at least one usable dual is present.
    pub fn has_duals(&self) -> bool {
        self.constraints
            .iter()
            .any(|constraint| constraint.dual_value.is_some())
    }
}

/// 固定整数 LP 报告缓存 / Cache for fixed-integer LP reports.
#[derive(Debug, Clone, Default)]
pub struct FixedIntegerLpSensitivityCache {
    entries: BTreeMap<String, LocalConstraintSensitivityReport>,
}

impl FixedIntegerLpSensitivityCache {
    /// 创建空缓存 / Create an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// 清空缓存 / Clear the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 返回缓存条目数 / Return the cache size.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空 / Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 固定整数 LP 局部有效性分析器 / Fixed-integer LP local-effectiveness analyzer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedIntegerLpSensitivityAnalyzer {
    /// 分析配置 / Analysis configuration.
    config: FixedIntegerLpSensitivityConfig,
}

impl Default for FixedIntegerLpSensitivityAnalyzer {
    fn default() -> Self {
        Self {
            config: FixedIntegerLpSensitivityConfig::default(),
        }
    }
}

impl FixedIntegerLpSensitivityAnalyzer {
    /// 创建分析器 / Create an analyzer.
    pub fn new(config: FixedIntegerLpSensitivityConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// 返回配置 / Return the configuration.
    pub const fn config(&self) -> FixedIntegerLpSensitivityConfig {
        self.config
    }

    /// 构造固定整数 LP / Build a fixed-integer LP without solving it.
    pub fn derive_model(
        &self,
        model: &LinearTriadModel,
        incumbent: &[f64],
    ) -> Result<FixedIntegerLpModel> {
        self.config.validate()?;
        FixedIntegerLpModel::from_incumbent(model, incumbent, self.config.integrality_tolerance)
    }

    /// 分析固定整数 LP / Analyze the fixed-integer LP.
    pub fn analyze<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        model: &LinearTriadModel,
        incumbent: &[f64],
    ) -> Result<LocalConstraintSensitivityReport> {
        let derived = self.derive_model(model, incumbent)?;
        derived.validate()?;
        let baseline_objective = objective_value(model, incumbent)?;
        let diagnostics = evaluate_linear_solution(model, incumbent, 1e-7)?;
        if diagnostics
            .constraint_evaluations
            .iter()
            .any(|evaluation| !evaluation.satisfied)
            || diagnostics
                .variable_bound_evaluations
                .iter()
                .any(|evaluation| !evaluation.satisfied)
        {
            return Err(invalid_sensitivity("baseline incumbent is infeasible"));
        }
        let report = solver.solve_linear_report(derived.model())?;
        self.analyze_report(model, incumbent, &derived, baseline_objective, &report)
    }

    /// 分析线性模型的兼容入口 / Compatibility entry point for a linear model.
    pub fn analyze_linear<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        model: &LinearTriadModel,
        incumbent: &[f64],
    ) -> Result<LocalConstraintSensitivityReport> {
        self.analyze(solver, model, incumbent)
    }

    /// 从已完成 baseline 报告分析 / Analyze from a completed baseline report.
    pub fn analyze_from_baseline_report<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        model: &LinearTriadModel,
        baseline_report: &SolveReport<f64>,
    ) -> Result<LocalConstraintSensitivityReport> {
        baseline_report.validate()?;
        let solution = baseline_report
            .solution
            .as_ref()
            .ok_or_else(|| invalid_sensitivity("baseline report has no incumbent"))?;
        let incumbent = if !solution.values.is_empty() {
            solution.values.clone()
        } else {
            let mapping = linear_model_mapping(model)?;
            mapping
                .variables_by_column
                .iter()
                .map(|id| {
                    solution
                        .stable_values
                        .get(id)
                        .copied()
                        .ok_or_else(|| invalid_sensitivity("baseline report misses a variable"))
                })
                .collect::<Result<Vec<_>>>()?
        };
        self.analyze(solver, model, &incumbent)
    }

    /// 从固定 LP 报告分析 / Analyze a fixed-integer LP report with automatic derivation.
    pub fn analyze_solved_report(
        &self,
        model: &LinearTriadModel,
        incumbent: &[f64],
        lp_report: &SolveReport<f64>,
    ) -> Result<LocalConstraintSensitivityReport> {
        let derived = self.derive_model(model, incumbent)?;
        let baseline_objective = objective_value(model, incumbent)?;
        self.analyze_report(model, incumbent, &derived, baseline_objective, lp_report)
    }

    /// 固定 LP 报告的兼容名称 / Compatibility name for a solved fixed-integer LP report.
    pub fn analyze_lp_report(
        &self,
        model: &LinearTriadModel,
        incumbent: &[f64],
        lp_report: &SolveReport<f64>,
    ) -> Result<LocalConstraintSensitivityReport> {
        self.analyze_solved_report(model, incumbent, lp_report)
    }

    /// 分析一个已取得的 LP 报告 / Analyze an already obtained LP report.
    pub fn analyze_report(
        &self,
        original_model: &LinearTriadModel,
        incumbent: &[f64],
        derived: &FixedIntegerLpModel,
        baseline_objective: f64,
        report: &SolveReport<f64>,
    ) -> Result<LocalConstraintSensitivityReport> {
        self.config.validate()?;
        derived.validate()?;
        if !baseline_objective.is_finite() {
            return Err(invalid_sensitivity("baseline objective must be finite"));
        }
        if incumbent.len() != original_model.num_variables() {
            return Err(invalid_sensitivity(
                "incumbent dimension does not match model",
            ));
        }
        report.validate()?;
        let original_mapping = linear_model_mapping(original_model)?;
        let derived_mapping = linear_model_mapping(derived.model())?;
        if original_mapping.constraints_by_row != derived_mapping.constraints_by_row {
            return Err(invalid_sensitivity(
                "fixed-integer LP changed original constraint identities",
            ));
        }

        let lp_objective = report
            .solution
            .as_ref()
            .and_then(|solution| solution.objective_value.or(solution.objective));
        let solved_optimal =
            report.is_optimal() && report.problem_status == ProblemStatus::Feasible;
        let mut status = if solved_optimal {
            AnalysisStatus::Reachable
        } else {
            AnalysisStatus::Unknown
        };
        let mut unavailable_reason = None;
        let activity_report =
            ConstraintActivityAnalyzer::default().analyze_linear(original_model, incumbent)?;
        let duals = report
            .solution
            .as_ref()
            .and_then(|solution| solution.dual_solution.as_deref());
        let usable_duals = if solved_optimal {
            let certificate_valid = crate::solver::require_optimal_lp_certificate_for_linear_model(
                report,
                derived.model(),
            )
            .is_ok();
            match (duals, certificate_valid) {
                (Some(duals), true)
                    if duals.len() == original_mapping.constraints_by_row.len()
                        && duals.iter().all(|value| value.is_finite()) =>
                {
                    Some(duals)
                }
                (Some(_), _) | (None, _) => {
                    status = AnalysisStatus::Unsupported;
                    unavailable_reason = Some(if duals.is_none() {
                        "the selected LP backend did not return row duals".to_owned()
                    } else {
                        "LP dual certificate failed independent primal-dual validation".to_owned()
                    });
                    None
                }
            }
        } else {
            None
        };
        // 证明门控：可行性侧由 `solved_optimal` 把关，不可行侧必须要求已验证的不可行证书，
        // 否则受限求解返回的 `Infeasible` 会被读成"已证明不可达"（计划 8.14 / 3.5）。
        // Proof gating: the feasibility side is guarded by `solved_optimal`, and the infeasibility
        // side must require a verified certificate; otherwise a budget-limited `Infeasible` would be
        // read as a proven unreachability.
        let infeasibility_proven = report.problem_status == ProblemStatus::Infeasible
            && crate::solver::require_infeasibility_certificate(report).is_ok();
        let final_status = match report.problem_status {
            ProblemStatus::Feasible => status,
            ProblemStatus::Infeasible => {
                AnalysisStatus::from_problem_status_with_proof(
                    ProblemStatus::Infeasible,
                    infeasibility_proven,
                )
            }
            ProblemStatus::Unknown
            | ProblemStatus::Unbounded
            | ProblemStatus::InfeasibleOrUnbounded => AnalysisStatus::Unknown,
        };

        let constraints = original_mapping
            .constraints_by_row
            .iter()
            .enumerate()
            .map(|(row, id)| {
                let dual_value = usable_duals.and_then(|duals| duals.get(row).copied());
                let local_effective =
                    dual_value.map(|value| value.abs() > self.config.dual_tolerance);
                let activity = activity_report
                    .constraints
                    .iter()
                    .find(|activity| activity.constraint_id.0 == *id)
                    .cloned();
                LocalConstraintSensitivity {
                    constraint_id: StableConstraintId(id.clone()),
                    dual_value,
                    local_effective,
                    status: if dual_value.is_some() {
                        AnalysisStatus::Reachable
                    } else {
                        final_status
                    },
                    scope: SensitivityScope::FixedIntegerIncumbent,
                    activity,
                }
            })
            .collect();

        let result = LocalConstraintSensitivityReport {
            schema_version: FIXED_INTEGER_LP_REPORT_SCHEMA_VERSION.to_owned(),
            scope: SensitivityScope::FixedIntegerIncumbent,
            baseline_objective,
            lp_objective,
            fixed_integer_count: derived.fixed_integer_count(),
            constraints,
            status: final_status,
            termination_reason: Some(report.termination_reason),
            unavailable_reason,
        };
        result.validate()?;
        Ok(result)
    }

    /// CP snapshot 的固定整数 LP 分析 / Fixed-integer LP analysis for a CP snapshot.
    ///
    /// 这是事项 C / 12.3-S2 的 **CP 侧入口**：先经现有 CP→线性降维，再用降维过程的
    /// `row_origins` provenance 把行对偶回映为**原始 CP 约束**身份。
    ///
    /// 三条语义边界：
    /// 1. **只有"一个原始约束恰好对应唯一降阶行"时才回映对偶**。这是当前的主要限制：Rust 的
    ///    CP→MIP 降维对**单个**线性比较采用 Big-M 重化（2 行关系编码 + 2 行等值强制，合计 4 行、
    ///    全部同源），因此不存在"主行"——关系变量在 LP 松弛中是连续的，任何单行对偶都不等于原始
    ///    约束的影子价格。挑一行冒充是不可辩护的，所以此时显式返回 `Unsupported` 并点名多行编码。
    /// 2. `Interval` 行是区间定义背景，不作为原始约束证据出现在报告中。
    /// 3. 降维为表达约束而引入的**辅助整数变量**（如 Big-M 重化的 0/1 关系变量）是编码产物、
    ///    不是原始决策，因此被**松弛为连续**而不是固定——固定它们会因取值不由 CP incumbent 决定
    ///    而使求解不成立。原始 CP 整数变量仍然固定为 incumbent 值。
    ///
    /// This is the CP-side entry point for item C / 12.3-S2: lower the CP snapshot, then use the
    /// lowering's `row_origins` provenance to map row duals back to **original CP constraint**
    /// identities. Boundary 1 is the current limitation: Rust's CP→MIP lowering encodes a **single**
    /// linear comparison with a Big-M reification (two relation rows plus two equality-enforcement rows,
    /// four in total and all sharing one origin), so no "main row" exists — the relation variable is
    /// continuous in the LP relaxation and no single row's dual equals the original constraint's shadow
    /// price. Picking one row is not defensible, so this returns `Unsupported` and names the multi-row
    /// encoding. Boundary 2 keeps interval rows out of the report. Boundary 3 relaxes lowering-generated
    /// **auxiliary integer variables** to continuous instead of fixing them, because the CP incumbent
    /// does not determine their values; original CP integer variables are still fixed.
    pub fn analyze_constraint_programming<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        snapshot: &ConstraintProgrammingSnapshot,
        fixed_values: &BTreeMap<StableVariableId, i64>,
        baseline_objective: f64,
    ) -> Result<LocalConstraintSensitivityReport> {
        self.config.validate()?;
        if !baseline_objective.is_finite() {
            return Err(invalid_sensitivity("baseline objective must be finite"));
        }
        let lowering = lower_constraint_programming(snapshot).map_err(|error| {
            invalid_sensitivity(format!(
                "CP snapshot could not be lowered for fixed-integer LP analysis: {error}"
            ))
        })?;
        let identity = cp_row_identity(&lowering);
        let coverage_complete = snapshot
            .variables
            .iter()
            .all(|variable| fixed_values.contains_key(&variable.variable.stable_id));
        if !coverage_complete {
            return Ok(cp_unsupported_report(
                baseline_objective,
                fixed_values.len(),
                identity,
                "the fixed integer assignment does not cover every source CP variable",
            ));
        }

        // 先把全部整数类型松弛，再**只**把源 CP 变量标记回整数；辅助整数因此保持连续。
        // Relax every integer type first, then mark **only** the source CP variables back as integer, so
        // auxiliary integers stay continuous.
        let mut model = lowering.model.linear_relaxed();
        let mut incumbent = vec![0.0; model.num_variables()];
        for (id, column) in &lowering.source_variables {
            let value = fixed_values
                .get(id)
                .copied()
                .ok_or_else(|| invalid_sensitivity("a source variable has no fixed value"))?;
            if *column >= model.basic.var_types.len() {
                return Err(invalid_sensitivity("lowered column index is out of range"));
            }
            model.basic.var_types[*column] = VariableType::Integer;
            incumbent[*column] = value as f64;
        }

        let derived = self.derive_model(&model, &incumbent)?;
        derived.validate()?;
        let report = solver.solve_linear_report(derived.model())?;
        let analysis = self.analyze_report(&model, &incumbent, &derived, baseline_objective, &report)?;

        let cp_activity = ConstraintActivityAnalyzer::default()
            .analyze_cp(snapshot, fixed_values)
            .ok();
        let mut result = LocalConstraintSensitivityReport {
            schema_version: FIXED_INTEGER_LP_REPORT_SCHEMA_VERSION.to_owned(),
            scope: SensitivityScope::FixedIntegerIncumbent,
            baseline_objective,
            lp_objective: analysis.lp_objective,
            fixed_integer_count: analysis.fixed_integer_count,
            constraints: analysis
                .constraints
                .iter()
                .enumerate()
                .filter_map(|(row, entry)| {
                    let id = identity.get(row).and_then(|entry| entry.clone())?;
                    Some(LocalConstraintSensitivity {
                        activity: cp_activity.as_ref().and_then(|activity| {
                            activity
                                .constraints
                                .iter()
                                .find(|activity| activity.constraint_id == id)
                                .cloned()
                        }),
                        constraint_id: id,
                        dual_value: entry.dual_value,
                        local_effective: entry.local_effective,
                        status: entry.status,
                        scope: entry.scope,
                    })
                })
                .collect(),
            status: analysis.status,
            termination_reason: analysis.termination_reason,
            unavailable_reason: analysis.unavailable_reason,
        };
        if result.constraints.is_empty() && !snapshot.constraints.is_empty() {
            result.status = AnalysisStatus::Unsupported;
            result.unavailable_reason = Some(
                "no original CP constraint maps to a unique lowered row (Rust encodes a linear \
                 comparison as a multi-row Big-M reification), and for a pure-integer CP model the \
                 fixed-integer LP has no continuous freedom at all: with every CP variable pinned the \
                 objective is constant, so its row duals carry no marginal information even under a \
                 non-reified encoding. CP-side local effectiveness must therefore be established by \
                 RHS perturbation or removal, not by this stage's duals"
                    .to_owned(),
            );
        }
        result.validate()?;
        Ok(result)
    }

    /// 使用显式缓存分析 / Analyze while reusing an explicit cache.
    pub fn analyze_cached<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        model: &LinearTriadModel,
        incumbent: &[f64],
        cache: &mut FixedIntegerLpSensitivityCache,
    ) -> Result<LocalConstraintSensitivityReport> {
        let key = cache_key(solver, model, incumbent, self.config)?;
        if let Some(report) = cache.entries.get(&key) {
            return Ok(report.clone());
        }
        let report = self.analyze(solver, model, incumbent)?;
        cache.entries.insert(key, report.clone());
        Ok(report)
    }
}

/// 降阶行 → 原始 CP 约束身份 / Lowered row → original CP constraint identity.
///
/// 只有"一个原始约束恰好对应唯一降阶行"时才回映该行：一个 CP 全局约束若被降为多行，
/// 任意单行的对偶都不能冒充该原始约束的对偶。`Interval` 行属于区间定义背景，同样不回映。
///
/// A row is mapped only when exactly one lowered row carries that source: when a CP global
/// constraint lowers to several rows, no single row's dual may stand in for the original
/// constraint's dual. `Interval` rows are interval-definition background and are not mapped either.
fn cp_row_identity(lowering: &MipLoweringResult) -> Vec<Option<StableConstraintId>> {
    let mut rows_per_source: BTreeMap<&StableConstraintId, usize> = BTreeMap::new();
    for origin in &lowering.row_origins {
        if let MipRowOrigin::Constraint(id) = origin {
            *rows_per_source.entry(id).or_insert(0) += 1;
        }
    }
    lowering
        .row_origins
        .iter()
        .map(|origin| match origin {
            MipRowOrigin::Constraint(id) if rows_per_source.get(id) == Some(&1) => Some(id.clone()),
            _ => None,
        })
        .collect()
}

/// 构造 CP 侧 `Unsupported` 报告 / Build a CP-side `Unsupported` report.
///
/// 报告仍然列出全部可回映的原始约束身份，但**不携带任何对偶值**：缺少证据必须显式表达，
/// 而不是留空让调用方猜。
/// The report still lists every mappable original constraint identity but carries **no dual
/// values**: missing evidence must be stated explicitly rather than left for the caller to guess.
fn cp_unsupported_report(
    baseline_objective: f64,
    fixed_integer_count: usize,
    identity: Vec<Option<StableConstraintId>>,
    reason: &str,
) -> LocalConstraintSensitivityReport {
    let constraints = identity
        .into_iter()
        .flatten()
        .map(|id| LocalConstraintSensitivity {
            constraint_id: id,
            dual_value: None,
            local_effective: None,
            status: AnalysisStatus::Unsupported,
            scope: SensitivityScope::FixedIntegerIncumbent,
            activity: None,
        })
        .collect();
    LocalConstraintSensitivityReport {
        schema_version: FIXED_INTEGER_LP_REPORT_SCHEMA_VERSION.to_owned(),
        scope: SensitivityScope::FixedIntegerIncumbent,
        baseline_objective,
        lp_objective: None,
        fixed_integer_count,
        constraints,
        status: AnalysisStatus::Unsupported,
        termination_reason: None,
        unavailable_reason: Some(reason.to_owned()),
    }
}

fn objective_value(model: &LinearTriadModel, values: &[f64]) -> Result<f64> {
    if values.len() != model.num_variables() || model.c.len() != model.num_variables() {
        return Err(invalid_sensitivity(
            "objective and incumbent dimensions do not match the model",
        ));
    }
    let value = model
        .c
        .iter()
        .zip(values)
        .map(|(coefficient, value)| coefficient * value)
        .sum::<f64>();
    if value.is_finite() {
        Ok(value)
    } else {
        Err(invalid_sensitivity("objective evaluation is non-finite"))
    }
}

fn cache_key<S: LinearSolver + ?Sized>(
    solver: &S,
    model: &LinearTriadModel,
    incumbent: &[f64],
    config: FixedIntegerLpSensitivityConfig,
) -> Result<String> {
    let fingerprint = linear_model_fingerprint(model)?;
    let solver_identity = solver_descriptor_fingerprint(&solver.descriptor());
    let values = incumbent
        .iter()
        .map(|value| value.to_bits().to_string())
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "{}:{}:{}:{}:{}",
        solver_identity.value,
        fingerprint.value,
        values,
        config.integrality_tolerance.to_bits(),
        config.dual_tolerance.to_bits()
    ))
}

fn invalid_sensitivity(message: impl Into<String>) -> CoreError {
    CoreError::Solver(SolverError::InvalidInput(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ObjectiveCategory;
    use crate::model::intermediate::{BasicLinearTriadModel, SparseVector};
    use crate::solver::audit::attach_linear_model_mapping;
    use crate::solver::{
        SolveFingerprints, SolveProof, SolveReport, SolverCapability, SolverInfo, SolverOutput,
        SolverStatus,
    };
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableType};

    #[derive(Debug, Clone, Copy)]
    struct DualSolver;

    impl SolverInfo for DualSolver {
        fn name(&self) -> &str {
            "fixed-lp-test"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Mip]
        }
    }

    impl LinearSolver for DualSolver {
        fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput> {
            let values = model
                .basic
                .lb
                .iter()
                .zip(&model.basic.ub)
                .map(|(lower, upper)| {
                    if lower.is_finite() {
                        *lower
                    } else if upper.is_finite() {
                        *upper
                    } else {
                        0.0
                    }
                })
                .collect::<Vec<_>>();
            let objective = model
                .c
                .iter()
                .zip(&values)
                .map(|(coefficient, value)| coefficient * value)
                .sum();
            Ok(SolverOutput::new(SolverStatus::Optimal)
                .with_solution(values)
                .with_objective(objective)
                .with_dual(vec![1.0; model.num_constraints()]))
        }

        fn solve_linear_report(&self, model: &LinearTriadModel) -> Result<SolveReport<f64>> {
            let values = vec![3.0, 7.0];
            let objective = model
                .c
                .iter()
                .zip(&values)
                .map(|(coefficient, value)| coefficient * value)
                .sum();
            let mut solution = crate::solver::SolveSolution::vector(values);
            solution.objective = Some(objective);
            solution.objective_value = Some(objective);
            solution.dual_solution = Some(vec![1.0; model.num_constraints()]);
            let report = SolveReport::builder(
                crate::solver::ProblemStatus::Feasible,
                crate::solver::TerminationReason::Completed,
            )
            .solution(solution)
            .proof(SolveProof::optimality())
            .fingerprints(SolveFingerprints {
                model: Some(crate::solver::linear_model_fingerprint(model)?),
                ..Default::default()
            })
            .build()?;
            attach_linear_model_mapping(report, model)
        }
    }

    fn model() -> LinearTriadModel {        let mut basic = BasicLinearTriadModel::new("fixed-lp");
        let integer = ContinuousVariableItem::auto("integer");
        let continuous = ContinuousVariableItem::auto("continuous");
        basic.add_variable_with_bounds(
            Token::from_generic(integer, 0),
            0.0,
            10.0,
            VariableType::Integer,
        );
        basic.add_variable_with_bounds(
            Token::from_generic(continuous, 1),
            0.0,
            10.0,
            VariableType::Continuous,
        );
        let mut row = SparseVector::new();
        row.add(0, 1.0);
        row.add(1, 1.0);
        basic.add_constraint_with_metadata(
            row,
            10.0,
            "capacity".to_owned(),
            None,
            false,
            0,
            None,
            None,
        );
        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![1.0, 1.0], ObjectiveCategory::Maximum);
        model
    }

    #[test]
    fn derived_model_fixes_integers_and_keeps_original_model_unchanged() {
        let model = model();
        let fingerprint = linear_model_fingerprint(&model).expect("fingerprint");
        let derived = FixedIntegerLpSensitivityAnalyzer::default()
            .derive_model(&model, &[3.0, 4.0])
            .expect("derived LP");
        assert_eq!(derived.fixed_integer_count(), 1);
        assert_eq!(derived.model().basic.lb[0], 3.0);
        assert_eq!(derived.model().basic.ub[0], 3.0);
        assert!(!derived.model().basic.var_types[0].is_integer());
        assert_eq!(
            linear_model_fingerprint(&model).expect("fingerprint"),
            fingerprint
        );
        assert!(
            FixedIntegerLpSensitivityAnalyzer::default()
                .derive_model(&model, &[3.25, 4.0])
                .is_err()
        );
    }

    #[test]
    fn report_maps_duals_to_original_constraints_without_solver_rows() {
        let model = model();
        let report = FixedIntegerLpSensitivityAnalyzer::default()
            .analyze(&DualSolver, &model, &[3.0, 4.0])
            .expect("sensitivity report");
        assert_eq!(report.status, AnalysisStatus::Reachable);
        assert_eq!(report.fixed_integer_count, 1);
        assert_eq!(report.constraints.len(), 1);
        assert!(report.constraints[0].dual_value.is_some());
        assert!(report.constraints[0].activity.is_some());
        assert!(report.constraints[0].constraint_id.0.contains("capacity"));
        assert!(!format!("{:?}", report).contains("solver_row"));
        assert_eq!(report.scope, SensitivityScope::FixedIntegerIncumbent);
    }

    #[test]
    fn cp_entry_reports_the_real_blocker_for_reified_lowerings() {
        use crate::model::constraint_programming::{
            ConstraintDefinition, ConstraintProgrammingConstraint, ConstraintProgrammingModel,
            IntegerDomain, IntegerExpression, IntegerObjective, IntegerRelation, IntegerTerm,
            IntegerVariable,
        };

        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let mut model = ConstraintProgrammingModel::new("cp-fixed-lp");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 10).expect("domain"))
            .expect("x");
        model
            .register_variable(y.clone(), IntegerDomain::range(0, 10).expect("domain"))
            .expect("y");
        let capacity = IntegerExpression::linear(
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
        .expect("capacity expression");
        model
            .add_constraint(ConstraintDefinition::new(
                "capacity",
                ConstraintProgrammingConstraint::integer(
                    capacity,
                    IntegerRelation::LessOrEqual,
                    10,
                ),
            ))
            .expect("capacity constraint");
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
            .expect("objective expression"),
        ));
        let snapshot = model.freeze().expect("snapshot");

        // 先确认阻塞点究竟在哪里：单个线性比较被降维成**多行**（Big-M 重化），且全部同源。
        // 这比"辅助整数变量取值不可得"更根本——即使把辅助整数松弛为连续，多行同源仍然使
        // 任何单行对偶都无法代表原始约束的影子价格。
        //
        // First establish where the real blocker is: a single linear comparison lowers to **several**
        // rows (Big-M reification), all carrying the same origin. This is more fundamental than the
        // unavailable auxiliary values — even with auxiliary integers relaxed to continuous, multiple
        // same-origin rows mean no single row's dual can represent the original constraint's shadow
        // price.
        let lowering = lower_constraint_programming(&snapshot).expect("lowering");
        assert_eq!(snapshot.constraints.len(), 1);
        assert!(
            lowering.row_origins.len() > 1,
            "expected a multi-row reified encoding, got {} row(s)",
            lowering.row_origins.len()
        );
        assert!(
            lowering
                .row_origins
                .iter()
                .all(|origin| matches!(origin, MipRowOrigin::Constraint(id) if id.0 == "capacity")),
            "every generated row must carry the original CP constraint as its origin"
        );

        let fixed_values = BTreeMap::from([
            (x.stable_id.clone(), 3_i64),
            (y.stable_id.clone(), 4_i64),
        ]);
        let report = FixedIntegerLpSensitivityAnalyzer::default()
            .analyze_constraint_programming(&DualSolver, &snapshot, &fixed_values, 7.0)
            .expect("CP sensitivity report");

        // 因此桥接必须显式降级并点名多行重化编码，而不是挑一行冒充原始约束对偶，
        // 也不是留下"Reachable 但零约束"这种会被读成"没有约束需要分析"的结果。
        // The bridge must therefore degrade explicitly and name the multi-row reified encoding, rather
        // than picking one row to impersonate the original constraint's dual or leaving "Reachable with
        // zero constraints", which reads as "there are no constraints to analyze".
        assert_eq!(report.status, AnalysisStatus::Unsupported);
        assert!(report.constraints.is_empty());
        let reason = report.unavailable_reason.as_deref().unwrap_or_default();
        assert!(
            reason.contains("multi-row"),
            "the reason must name the multi-row encoding, got {reason:?}"
        );
        // 原因还必须点出**结构性**结论：纯整数 CP 模型固定全部整数后 LP 没有连续自由度，
        // 因此即使换成非重化编码，本阶段的对偶也不携带边际信息。这决定了 N2 的替代路径。
        // The reason must also state the **structural** conclusion: for a pure-integer CP model the
        // fixed-integer LP has no continuous freedom once every CP variable is pinned, so this stage's
        // duals carry no marginal information even under a non-reified encoding. That determines the
        // alternative path for N2.
        assert!(
            reason.contains("no continuous freedom"),
            "the reason must state why non-reified lowering would not help either, got {reason:?}"
        );
        assert_eq!(report.scope, SensitivityScope::FixedIntegerIncumbent);
        assert_eq!(report.fixed_integer_count, 2);
        assert!(!format!("{:?}", report).contains("solver_row"));

        // 未覆盖全部源变量时同样必须显式 Unsupported，并给出原因、且不携带任何对偶值。
        // Incomplete coverage of source variables must likewise be explicitly Unsupported with a
        // reason and must carry no dual values.
        let partial = BTreeMap::from([(x.stable_id.clone(), 3_i64)]);
        let unsupported = FixedIntegerLpSensitivityAnalyzer::default()
            .analyze_constraint_programming(&DualSolver, &snapshot, &partial, 7.0)
            .expect("unsupported CP report");
        assert_eq!(unsupported.status, AnalysisStatus::Unsupported);
        assert!(unsupported.unavailable_reason.is_some());
        assert!(
            unsupported
                .constraints
                .iter()
                .all(|entry| entry.dual_value.is_none())
        );
    }

    #[test]
    fn explicit_cache_avoids_duplicate_entries() {        let model = model();
        let analyzer = FixedIntegerLpSensitivityAnalyzer::default();
        let mut cache = FixedIntegerLpSensitivityCache::new();
        analyzer
            .analyze_cached(&DualSolver, &model, &[3.0, 4.0], &mut cache)
            .expect("first analysis");
        analyzer
            .analyze_cached(&DualSolver, &model, &[3.0, 4.0], &mut cache)
            .expect("cached analysis");
        assert_eq!(cache.len(), 1);
    }
}
