//! RHS 扰动与删除重优化分析 / RHS perturbation and removal reoptimization analysis.
//!
//! 分析器只克隆并修改线性模型的一个原始 RHS，随后通过现有 `LinearSolver` 重新求解。
//! 报告只保留原始约束身份、目标变化和统一求解状态，不暴露派生模型中的 row/column。
//! The analyzer clones a linear model, changes one original RHS, and re-solves it through the
//! existing `LinearSolver`. Reports retain only original constraint identity, objective changes,
//! and unified solve status; derived row/column details never cross the public boundary.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::error::{CoreError, Result, SolverError};
use crate::model::ObjectiveCategory;
use crate::model::intermediate::LinearTriadModel;
use crate::solver::audit::{evaluate_linear_solution, linear_model_mapping};
use crate::solver::fingerprint::{
    linear_model_fingerprint, solver_descriptor_fingerprint,
};
use crate::solver::report::{ProblemStatus, SolveReport, TerminationReason};
use crate::solver::{LinearSolver, StableConstraintId};

use super::{AnalysisStatus, ConstraintId};

/// RHS 扰动报告 schema 版本 / RHS perturbation report schema version.
pub const PERTURBATION_REPORT_SCHEMA_VERSION: &str = "1.0";

/// 自适应扰动报告 schema 版本 / Adaptive-perturbation report schema version.
pub const ADAPTIVE_PERTURBATION_REPORT_SCHEMA_VERSION: &str = "1.0";

/// 自适应扰动配置 / Adaptive perturbation configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptivePerturbationConfig {
    /// 初始正扰动 / Initial positive perturbation.
    pub initial_delta: f64,
    /// 每次扩大倍数 / Growth factor for each step.
    pub growth_factor: f64,
    /// 最大允许扰动 / Maximum allowed perturbation.
    pub max_delta: f64,
    /// 是否在首次有效后进行二分细化 / Whether to refine the first effective interval.
    pub refine_threshold: bool,
    /// 二分停止宽度 / Binary-search stopping width.
    pub refinement_tolerance: f64,
}

impl Default for AdaptivePerturbationConfig {
    fn default() -> Self {
        Self {
            initial_delta: 1.0,
            growth_factor: 2.0,
            max_delta: 16.0,
            refine_threshold: true,
            refinement_tolerance: 1e-7,
        }
    }
}

impl AdaptivePerturbationConfig {
    /// 校验自适应策略 / Validate the adaptive strategy.
    pub fn validate(&self) -> Result<()> {
        if !self.initial_delta.is_finite()
            || self.initial_delta <= 0.0
            || !self.growth_factor.is_finite()
            || self.growth_factor <= 1.0
            || !self.max_delta.is_finite()
            || self.max_delta < self.initial_delta
            || !self.refinement_tolerance.is_finite()
            || self.refinement_tolerance <= 0.0
        {
            return Err(invalid_perturbation(
                "adaptive perturbation values must be finite and ordered",
            ));
        }
        Ok(())
    }

    /// 设置是否细化首次有效区间 / Set whether to refine the first effective interval.
    pub const fn with_refinement(mut self, refine: bool) -> Self {
        self.refine_threshold = refine;
        self
    }

    /// 设置二分停止宽度 / Set the binary-search stopping width.
    pub const fn with_refinement_tolerance(mut self, tolerance: f64) -> Self {
        self.refinement_tolerance = tolerance;
        self
    }
}

/// RHS 扰动策略 / RHS perturbation policy.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintPerturbationPolicy {
    /// 显式扰动序列 / Explicit perturbation sequence.
    pub deltas: Vec<f64>,
    /// 自适应策略；与显式序列互斥 / Adaptive strategy, mutually exclusive with explicit deltas.
    pub adaptive: Option<AdaptivePerturbationConfig>,
    /// 是否执行删除重优化 / Whether to run a removal reoptimization.
    pub removal_test: bool,
    /// 本次分析最多调用 solver 的次数 / Maximum number of solver calls.
    pub max_solves: usize,
    /// 分析编排的总时间预算 / Overall orchestration time budget.
    pub time_budget: Option<Duration>,
    /// 判断目标改善的容差 / Objective-improvement tolerance.
    pub objective_tolerance: f64,
}

/// RHS 扰动策略的兼容名称 / Compatibility name for the RHS perturbation policy.
pub type PerturbationPolicy = ConstraintPerturbationPolicy;

impl Default for ConstraintPerturbationPolicy {
    fn default() -> Self {
        Self {
            deltas: vec![1.0],
            adaptive: None,
            removal_test: false,
            max_solves: 32,
            time_budget: None,
            objective_tolerance: 1e-7,
        }
    }
}

impl ConstraintPerturbationPolicy {
    /// 创建单一扰动策略 / Create a single-delta policy.
    pub fn single_delta(delta: f64) -> Self {
        Self {
            deltas: vec![delta],
            adaptive: None,
            ..Self::default()
        }
    }

    /// 创建多扰动策略 / Create a multiple-delta policy.
    pub fn multiple_deltas<I>(deltas: I) -> Self
    where
        I: IntoIterator<Item = f64>,
    {
        Self {
            deltas: deltas.into_iter().collect(),
            adaptive: None,
            ..Self::default()
        }
    }

    /// 创建自适应策略 / Create an adaptive policy.
    pub fn adaptive(config: AdaptivePerturbationConfig) -> Self {
        Self {
            deltas: Vec::new(),
            adaptive: Some(config),
            ..Self::default()
        }
    }

    /// 启用删除测试 / Enable the removal test.
    pub const fn with_removal_test(mut self, enabled: bool) -> Self {
        self.removal_test = enabled;
        self
    }

    /// 设置最大求解次数 / Set the maximum number of solver calls.
    pub const fn with_max_solves(mut self, max_solves: usize) -> Self {
        self.max_solves = max_solves;
        self
    }

    /// 设置总时间预算 / Set the total analysis time budget.
    pub const fn with_time_budget(mut self, budget: Duration) -> Self {
        self.time_budget = Some(budget);
        self
    }

    /// 设置目标改善容差 / Set the objective-improvement tolerance.
    pub const fn with_objective_tolerance(mut self, tolerance: f64) -> Self {
        self.objective_tolerance = tolerance;
        self
    }

    /// 校验策略 / Validate the policy.
    pub fn validate(&self) -> Result<()> {
        if self.max_solves == 0
            || !self.objective_tolerance.is_finite()
            || self.objective_tolerance < 0.0
        {
            return Err(invalid_perturbation(
                "max solves must be positive and objective tolerance must be finite",
            ));
        }
        if self.deltas.is_empty() == self.adaptive.is_none() {
            return Err(invalid_perturbation(
                "a perturbation policy must contain either explicit deltas or an adaptive strategy",
            ));
        }
        if self
            .deltas
            .iter()
            .any(|delta| !delta.is_finite() || *delta <= 0.0)
        {
            return Err(invalid_perturbation(
                "RHS perturbations must be finite and strictly positive",
            ));
        }
        if let Some(config) = self.adaptive {
            config.validate()?;
        }
        if self.time_budget.is_some_and(|budget| budget.is_zero()) {
            return Err(invalid_perturbation("time budget must be positive"));
        }
        Ok(())
    }
}

/// 自适应扰动结果 / Adaptive perturbation outcome.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "camelCase"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdaptivePerturbationOutcome {
    /// 已完成扫描但没有观察到改善 / Scan completed without an observed improvement.
    NoObservedEffect,
    /// 在给定扰动处首次观察到改善 / Improvement first observed at a delta.
    EffectiveAt {
        /// 首次有效扰动 / First effective delta.
        delta: f64,
    },
    /// 首次有效扰动的阈值区间 / Threshold interval for the first effective delta.
    ThresholdInterval {
        /// 无改善上界 / Largest tested ineffective delta.
        lower: f64,
        /// 有改善下界 / Smallest tested effective delta.
        upper: f64,
    },
    /// 因预算耗尽无法形成结论 / Budget prevented a complete conclusion.
    UnknownDueToBudget,
    /// solver 结果没有形成改善结论 / Solver results did not form a conclusion.
    Unknown,
}

impl AdaptivePerturbationOutcome {
    /// 是否观察到全局改善 / Whether an improvement was observed.
    pub const fn is_effective(self) -> bool {
        matches!(
            self,
            Self::EffectiveAt { .. } | Self::ThresholdInterval { .. }
        )
    }
}

/// 单次 RHS 扰动观察 / One RHS perturbation observation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintPerturbationObservation {
    /// 正扰动量 / Positive perturbation amount.
    pub delta: f64,
    /// 扰动后的 RHS / Perturbed RHS.
    pub perturbed_rhs: f64,
    /// 统一求解状态 / Unified solve status.
    pub status: AnalysisStatus,
    /// solver 终止原因 / Solver termination reason.
    pub termination_reason: TerminationReason,
    /// 新目标值 / New objective value.
    pub objective: Option<f64>,
    /// 原始目标变化（新值减旧值）/ Raw objective change (new minus baseline).
    pub objective_change: Option<f64>,
    /// 按优化方向归一化的改善量 / Improvement normalized by objective direction.
    pub improvement: Option<f64>,
    /// 是否已证明目标改善 / Whether an objective improvement is proven.
    pub improved: Option<bool>,
    /// 整数变量模式是否改变 / Whether the integer-variable pattern changed.
    pub integer_pattern_changed: Option<bool>,
    /// 本次求解耗时 / Solve duration.
    pub solve_time: Duration,
    /// 是否有最优性证明 / Whether an optimality proof is available.
    pub objective_proven: bool,
}

/// 单次扰动观察的兼容名称 / Compatibility name for one perturbation observation.
pub type PerturbationObservation = ConstraintPerturbationObservation;

/// 删除重优化观察 / Removal-test observation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct RemovalTestObservation {
    /// 统一求解状态 / Unified solve status.
    pub status: AnalysisStatus,
    /// solver 终止原因 / Solver termination reason.
    pub termination_reason: TerminationReason,
    /// 删除后的目标值 / Objective after removal.
    pub objective: Option<f64>,
    /// 原始目标变化（新值减旧值）/ Raw objective change (new minus baseline).
    pub objective_change: Option<f64>,
    /// 按优化方向归一化的改善量 / Improvement normalized by objective direction.
    pub improvement: Option<f64>,
    /// 单独删除是否已证明有效 / Whether removal is proven effective on its own.
    pub effective: Option<bool>,
    /// 整数变量模式是否改变 / Whether the integer-variable pattern changed.
    pub integer_pattern_changed: Option<bool>,
    /// 本次求解耗时 / Solve duration.
    pub solve_time: Duration,
    /// 是否有最优性证明 / Whether an optimality proof is available.
    pub objective_proven: bool,
}

/// 单条约束的 RHS 扰动报告 / RHS perturbation report for one constraint.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintPerturbationReport {
    /// 报告 schema 版本 / Report schema version.
    pub schema_version: String,
    /// 原始约束稳定身份 / Stable original-constraint identity.
    pub constraint_id: ConstraintId,
    /// baseline 目标值 / Baseline objective value.
    pub baseline_objective: f64,
    /// 扰动观察列表 / Perturbation observations.
    pub observations: Vec<ConstraintPerturbationObservation>,
    /// 可选删除观察 / Optional removal observation.
    pub removal: Option<RemovalTestObservation>,
    /// 自适应扰动结论 / Adaptive perturbation conclusion.
    pub adaptive_outcome: Option<AdaptivePerturbationOutcome>,
    /// 总体分析状态 / Overall analysis status.
    pub status: AnalysisStatus,
    /// 实际调用 solver 的次数 / Number of solver calls made.
    pub solve_count: usize,
    /// 未形成结论的原因 / Reason no complete conclusion was formed.
    pub unavailable_reason: Option<String>,
}

impl ConstraintPerturbationReport {
    /// 校验报告不变量 / Validate report invariants.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version.trim().is_empty()
            || self.constraint_id.0.trim().is_empty()
            || !self.baseline_objective.is_finite()
            || self.observations.iter().any(|observation| {
                !observation.delta.is_finite()
                    || observation.delta <= 0.0
                    || !observation.perturbed_rhs.is_finite()
                    || observation
                        .objective
                        .is_some_and(|value| !value.is_finite())
                    || observation
                        .objective_change
                        .is_some_and(|value| !value.is_finite())
                    || observation
                        .improvement
                        .is_some_and(|value| !value.is_finite())
            })
        {
            return Err(invalid_perturbation("perturbation report is invalid"));
        }
        if self.removal.as_ref().is_some_and(|removal| {
            removal.objective.is_some_and(|value| !value.is_finite())
                || removal
                    .objective_change
                    .is_some_and(|value| !value.is_finite())
                || removal.improvement.is_some_and(|value| !value.is_finite())
        }) {
            return Err(invalid_perturbation("removal observation is invalid"));
        }
        let expected_solve_count = self.observations.len() + usize::from(self.removal.is_some());
        if self.solve_count != expected_solve_count {
            return Err(invalid_perturbation(
                "perturbation solve count does not match its observations",
            ));
        }
        Ok(())
    }

    /// 是否观察到已证明的目标改善 / Whether a proven objective improvement was observed.
    pub fn has_global_improvement(&self) -> bool {
        self.observations
            .iter()
            .any(|observation| observation.improved == Some(true))
            || self
                .removal
                .as_ref()
                .is_some_and(|removal| removal.effective == Some(true))
    }

    /// 返回最后一次扰动观察 / Return the last perturbation observation.
    pub fn last_observation(&self) -> Option<&ConstraintPerturbationObservation> {
        self.observations.last()
    }
}

/// RHS 扰动报告缓存 / Cache for RHS perturbation reports.
#[derive(Debug, Clone, Default)]
pub struct ConstraintPerturbationCache {
    entries: BTreeMap<String, ConstraintPerturbationReport>,
}

impl ConstraintPerturbationCache {
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

/// RHS 扰动分析器 / RHS perturbation analyzer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConstraintPerturbationAnalyzer {
    /// 默认策略 / Default policy.
    policy: ConstraintPerturbationPolicy,
}

impl ConstraintPerturbationAnalyzer {
    /// 创建使用默认策略的分析器 / Create an analyzer using the default policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用显式策略创建分析器 / Create an analyzer with an explicit policy.
    ///
    /// 该入口适合在多次分析中复用同一策略；单次调用也可直接传给 `analyze_with_policy`。
    /// This entry point is useful when the same policy is reused; one-off calls may use
    /// `analyze_with_policy` directly.
    pub fn with_policy(policy: &ConstraintPerturbationPolicy) -> Result<Self> {
        policy.validate()?;
        Ok(Self {
            policy: policy.clone(),
        })
    }

    /// 返回分析器默认策略 / Return the analyzer's default policy.
    pub fn policy(&self) -> &ConstraintPerturbationPolicy {
        &self.policy
    }

    /// 使用默认策略分析一条约束 / Analyze one constraint with the default policy.
    pub fn analyze<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        model: &LinearTriadModel,
        constraint_id: &ConstraintId,
        baseline_values: &[f64],
    ) -> Result<ConstraintPerturbationReport> {
        self.analyze_with_policy(solver, model, constraint_id, baseline_values, &self.policy)
    }

    /// 分析一条约束的兼容入口 / Compatibility entry point for one constraint.
    pub fn analyze_constraint<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        model: &LinearTriadModel,
        constraint_id: &ConstraintId,
        baseline_values: &[f64],
    ) -> Result<ConstraintPerturbationReport> {
        self.analyze(solver, model, constraint_id, baseline_values)
    }

    /// 从已完成 baseline 报告分析 / Analyze from a baseline solve report.
    pub fn analyze_from_baseline_report<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        model: &LinearTriadModel,
        constraint_id: &ConstraintId,
        baseline_report: &SolveReport<f64>,
    ) -> Result<ConstraintPerturbationReport> {
        baseline_report.validate()?;
        let solution = baseline_report
            .solution
            .as_ref()
            .ok_or_else(|| invalid_perturbation("baseline report has no incumbent"))?;
        let baseline_values =
            if !solution.values.is_empty() {
                solution.values.clone()
            } else {
                let mapping = linear_model_mapping(model)?;
                mapping
                    .variables_by_column
                    .iter()
                    .map(|id| {
                        solution.stable_values.get(id).copied().ok_or_else(|| {
                            invalid_perturbation("baseline report misses a variable")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            };
        self.analyze(solver, model, constraint_id, &baseline_values)
    }

    /// 使用显式策略分析一条约束 / Analyze one constraint with an explicit policy.
    pub fn analyze_with_policy<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        model: &LinearTriadModel,
        constraint_id: &ConstraintId,
        baseline_values: &[f64],
        policy: &ConstraintPerturbationPolicy,
    ) -> Result<ConstraintPerturbationReport> {
        policy.validate()?;
        let mapping = linear_model_mapping(model)?;
        let row = mapping.solver_row(&constraint_id.0).ok_or_else(|| {
            invalid_perturbation(format!("unknown constraint ID {}", constraint_id.0))
        })?;
        let baseline_objective = objective_value(model, baseline_values)?;
        validate_baseline(model, baseline_values, policy.objective_tolerance)?;
        let started = Instant::now();
        let mut observations = Vec::new();
        let mut solve_count = 0usize;
        let mut unavailable_reason = None;

        let _deltas = if let Some(config) = policy.adaptive {
            self.adaptive_deltas(
                solver,
                model,
                row,
                baseline_values,
                baseline_objective,
                config,
                policy,
                &mut observations,
                &mut solve_count,
                &started,
                &mut unavailable_reason,
            )?
        } else {
            let requested = policy.deltas.clone();
            for delta in &requested {
                if !can_solve(policy, &started, solve_count) {
                    unavailable_reason = Some("perturbation solve budget was exhausted".to_owned());
                    break;
                }
                let observation = self.solve_perturbation(
                    solver,
                    model,
                    row,
                    baseline_values,
                    baseline_objective,
                    *delta,
                    policy.objective_tolerance,
                )?;
                solve_count = solve_count.saturating_add(1);
                observations.push(observation);
            }
            requested
        };

        let removal = if policy.removal_test {
            if can_solve(policy, &started, solve_count) {
                let removed_model = remove_constraint(model, row)?;
                let report = solver.solve_linear_report(&removed_model)?;
                solve_count = solve_count.saturating_add(1);
                Some(removal_observation(
                    &report,
                    model,
                    &removed_model,
                    baseline_values,
                    baseline_objective,
                    policy.objective_tolerance,
                )?)
            } else {
                unavailable_reason = Some("removal test exceeded the solve budget".to_owned());
                None
            }
        } else {
            None
        };

        let adaptive_outcome = policy
            .adaptive
            .map(|config| classify_adaptive_outcome(config, &observations));
        let status = overall_status(
            &observations,
            removal.as_ref(),
            unavailable_reason.is_some(),
        );
        let report = ConstraintPerturbationReport {
            schema_version: PERTURBATION_REPORT_SCHEMA_VERSION.to_owned(),
            constraint_id: constraint_id.clone(),
            baseline_objective,
            observations,
            removal,
            adaptive_outcome,
            status,
            solve_count,
            unavailable_reason,
        };
        report.validate()?;
        Ok(report)
    }

    /// 使用显式缓存分析 / Analyze while reusing an explicit cache.
    pub fn analyze_cached<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        model: &LinearTriadModel,
        constraint_id: &ConstraintId,
        baseline_values: &[f64],
        policy: &ConstraintPerturbationPolicy,
        cache: &mut ConstraintPerturbationCache,
    ) -> Result<ConstraintPerturbationReport> {
        let key = cache_key(solver, model, constraint_id, baseline_values, policy)?;
        if let Some(report) = cache.entries.get(&key) {
            return Ok(report.clone());
        }
        let report =
            self.analyze_with_policy(solver, model, constraint_id, baseline_values, policy)?;
        cache.entries.insert(key, report.clone());
        Ok(report)
    }

    fn adaptive_deltas<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        model: &LinearTriadModel,
        row: usize,
        baseline_values: &[f64],
        baseline_objective: f64,
        config: AdaptivePerturbationConfig,
        policy: &ConstraintPerturbationPolicy,
        observations: &mut Vec<ConstraintPerturbationObservation>,
        solve_count: &mut usize,
        started: &Instant,
        unavailable_reason: &mut Option<String>,
    ) -> Result<Vec<f64>> {
        config.validate()?;
        let mut deltas = Vec::new();
        let mut delta = config.initial_delta;
        let mut ineffective_delta = None;
        let mut effective_delta = None;
        while delta <= config.max_delta + config.refinement_tolerance {
            if !can_solve(policy, started, *solve_count) {
                *unavailable_reason = Some("adaptive perturbation budget was exhausted".to_owned());
                break;
            }
            let observation = self.solve_perturbation(
                solver,
                model,
                row,
                baseline_values,
                baseline_objective,
                delta.min(config.max_delta),
                policy.objective_tolerance,
            )?;
            *solve_count = solve_count.saturating_add(1);
            let improved = observation.improved;
            deltas.push(observation.delta);
            observations.push(observation);
            match improved {
                Some(true) => {
                    effective_delta = Some(delta.min(config.max_delta));
                    break;
                }
                Some(false) => ineffective_delta = Some(delta.min(config.max_delta)),
                None => {
                    *unavailable_reason = Some("adaptive solver result was not optimal".to_owned());
                    break;
                }
            }
            let next = delta * config.growth_factor;
            if next <= delta {
                break;
            }
            delta = next;
        }

        if let (Some(lower), Some(upper)) = (ineffective_delta, effective_delta)
            && config.refine_threshold
        {
            let mut lower = lower;
            let mut upper = upper;
            while upper - lower > config.refinement_tolerance
                && can_solve(policy, started, *solve_count)
            {
                let middle = lower + (upper - lower) / 2.0;
                let observation = self.solve_perturbation(
                    solver,
                    model,
                    row,
                    baseline_values,
                    baseline_objective,
                    middle,
                    policy.objective_tolerance,
                )?;
                *solve_count = solve_count.saturating_add(1);
                match observation.improved {
                    Some(true) => upper = middle,
                    Some(false) => lower = middle,
                    None => {
                        *unavailable_reason = Some(
                            "adaptive threshold refinement did not receive an optimal result"
                                .to_owned(),
                        );
                        break;
                    }
                }
                deltas.push(middle);
                observations.push(observation);
            }
            if upper - lower > config.refinement_tolerance && unavailable_reason.is_none() {
                *unavailable_reason =
                    Some("adaptive threshold refinement budget was exhausted".to_owned());
            }
        }
        Ok(deltas)
    }

    fn solve_perturbation<S: LinearSolver + ?Sized>(
        &self,
        solver: &S,
        model: &LinearTriadModel,
        row: usize,
        baseline_values: &[f64],
        baseline_objective: f64,
        delta: f64,
        objective_tolerance: f64,
    ) -> Result<ConstraintPerturbationObservation> {
        let perturbed_model = perturb_rhs(model, row, delta)?;
        let report = solver.solve_linear_report(&perturbed_model)?;
        perturbation_observation(
            &report,
            model,
            &perturbed_model,
            baseline_values,
            baseline_objective,
            delta,
            perturbed_model.basic.b[row],
            objective_tolerance,
        )
    }
}

fn validate_baseline(model: &LinearTriadModel, values: &[f64], tolerance: f64) -> Result<()> {
    let diagnostics = evaluate_linear_solution(model, values, tolerance)?;
    if diagnostics
        .constraint_evaluations
        .iter()
        .any(|evaluation| !evaluation.satisfied)
        || diagnostics
            .variable_bound_evaluations
            .iter()
            .any(|evaluation| !evaluation.satisfied)
    {
        return Err(invalid_perturbation("baseline incumbent is infeasible"));
    }
    Ok(())
}

fn perturb_rhs(model: &LinearTriadModel, row: usize, delta: f64) -> Result<LinearTriadModel> {
    let mut derived = model.clone();
    let rhs = derived
        .basic
        .b
        .get_mut(row)
        .ok_or_else(|| invalid_perturbation("constraint row is missing a right-hand side"))?;
    *rhs += delta;
    if !rhs.is_finite() {
        return Err(invalid_perturbation("perturbed RHS is non-finite"));
    }
    Ok(derived)
}

fn remove_constraint(model: &LinearTriadModel, row: usize) -> Result<LinearTriadModel> {
    let mut derived = model.clone();
    if row >= derived.basic.A.rows.len() || row >= derived.basic.b.len() {
        return Err(invalid_perturbation("constraint row is missing"));
    }
    derived.basic.A.rows.remove(row);
    derived.basic.b.remove(row);
    remove_metadata(&mut derived.basic.constraint_names, row);
    remove_metadata(&mut derived.basic.constraint_group_ids, row);
    remove_metadata(&mut derived.basic.constraint_lazy_flags, row);
    remove_metadata(&mut derived.basic.constraint_priorities, row);
    remove_metadata(&mut derived.basic.constraint_args, row);
    remove_metadata(&mut derived.basic.constraint_source_symbol_ids, row);
    Ok(derived)
}

fn remove_metadata<T>(values: &mut Vec<T>, row: usize) {
    if row < values.len() {
        values.remove(row);
    }
}

fn perturbation_observation(
    report: &SolveReport<f64>,
    original_model: &LinearTriadModel,
    solved_model: &LinearTriadModel,
    baseline_values: &[f64],
    baseline_objective: f64,
    delta: f64,
    perturbed_rhs: f64,
    objective_tolerance: f64,
) -> Result<ConstraintPerturbationObservation> {
    report.validate()?;
    let objective = report
        .solution
        .as_ref()
        .and_then(|solution| solution.objective_value.or(solution.objective));
    let objective_change = objective.map(|value| value - baseline_objective);
    let improvement = objective_change.map(|change| match original_model.objective_category {
        ObjectiveCategory::Maximum => change,
        ObjectiveCategory::Minimum => -change,
    });
    let objective_proven = report.problem_status == ProblemStatus::Feasible
        && report.is_optimal()
        && solution_is_feasible(report, solved_model);
    let improved =
        objective_proven.then(|| improvement.is_some_and(|value| value > objective_tolerance));
    let integer_pattern_changed = solution_values(report, solved_model)
        .and_then(|values| pattern_changed(original_model, baseline_values, &values));
    Ok(ConstraintPerturbationObservation {
        delta,
        perturbed_rhs,
        status: analysis_status(report),
        termination_reason: report.termination_reason,
        objective,
        objective_change,
        improvement,
        improved,
        integer_pattern_changed,
        solve_time: report.statistics.solve_time,
        objective_proven,
    })
}

fn removal_observation(
    report: &SolveReport<f64>,
    original_model: &LinearTriadModel,
    solved_model: &LinearTriadModel,
    baseline_values: &[f64],
    baseline_objective: f64,
    objective_tolerance: f64,
) -> Result<RemovalTestObservation> {
    report.validate()?;
    let objective = report
        .solution
        .as_ref()
        .and_then(|solution| solution.objective_value.or(solution.objective));
    let objective_change = objective.map(|value| value - baseline_objective);
    let improvement = objective_change.map(|change| match original_model.objective_category {
        ObjectiveCategory::Maximum => change,
        ObjectiveCategory::Minimum => -change,
    });
    let objective_proven = report.problem_status == ProblemStatus::Feasible
        && report.is_optimal()
        && solution_is_feasible(report, solved_model);
    let effective =
        objective_proven.then(|| improvement.is_some_and(|value| value > objective_tolerance));
    let integer_pattern_changed = solution_values(report, solved_model)
        .and_then(|values| pattern_changed(original_model, baseline_values, &values));
    Ok(RemovalTestObservation {
        status: analysis_status(report),
        termination_reason: report.termination_reason,
        objective,
        objective_change,
        improvement,
        effective,
        integer_pattern_changed,
        solve_time: report.statistics.solve_time,
        objective_proven,
    })
}

fn analysis_status(report: &SolveReport<f64>) -> AnalysisStatus {
    // A feasibility result only becomes a proven reachability conclusion when the same run also
    // proved optimality. Infeasibility likewise requires a verified certificate. Without these
    // gates a budget-limited run that merely returned an incumbent, or a budget-limited
    // "infeasible", would be reported as a proven conclusion.
    // 只有在同一次运行同时证明最优性时，可行解才能升格为已证明的可达结论；不可行同样需要
    // 已验证证书。缺少这些门控时，"受限但拿到 incumbent"或"受限的不可行"会被报告为已证明结论。
    let proven = match report.problem_status {
        ProblemStatus::Feasible => report.is_optimal(),
        ProblemStatus::Infeasible => {
            crate::solver::require_infeasibility_certificate(report).is_ok()
        }
        ProblemStatus::Unbounded
        | ProblemStatus::InfeasibleOrUnbounded
        | ProblemStatus::Unknown => false,
    };
    AnalysisStatus::from_problem_status_with_proof(report.problem_status, proven)
}

fn solution_values(report: &SolveReport<f64>, model: &LinearTriadModel) -> Option<Vec<f64>> {
    let solution = report.solution.as_ref()?;
    if solution.values.len() == model.num_variables() {
        return Some(solution.values.clone());
    }
    let mapping = linear_model_mapping(model).ok()?;
    if solution.stable_values.len() != mapping.variables_by_column.len() {
        return None;
    }
    mapping
        .variables_by_column
        .iter()
        .map(|id| solution.stable_values.get(id).copied())
        .collect()
}

fn solution_is_feasible(report: &SolveReport<f64>, model: &LinearTriadModel) -> bool {
    let Some(values) = solution_values(report, model) else {
        return false;
    };
    let Ok(diagnostics) = evaluate_linear_solution(model, &values, 1e-7) else {
        return false;
    };
    diagnostics
        .constraint_evaluations
        .iter()
        .all(|evaluation| evaluation.satisfied)
        && diagnostics
            .variable_bound_evaluations
            .iter()
            .all(|evaluation| evaluation.satisfied)
}

fn pattern_changed(
    model: &LinearTriadModel,
    baseline_values: &[f64],
    perturbed_values: &[f64],
) -> Option<bool> {
    if baseline_values.len() != model.num_variables()
        || perturbed_values.len() != model.num_variables()
    {
        return None;
    }
    Some(
        model
            .basic
            .var_types
            .iter()
            .enumerate()
            .filter(|(_, variable_type)| variable_type.is_integer())
            .any(|(index, _)| (baseline_values[index] - perturbed_values[index]).abs() > 1e-7),
    )
}

fn objective_value(model: &LinearTriadModel, values: &[f64]) -> Result<f64> {
    if values.len() != model.num_variables() || model.c.len() != model.num_variables() {
        return Err(invalid_perturbation(
            "objective and baseline dimensions do not match the model",
        ));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(invalid_perturbation("baseline contains a non-finite value"));
    }
    let objective = model
        .c
        .iter()
        .zip(values)
        .map(|(coefficient, value)| coefficient * value)
        .sum::<f64>();
    if objective.is_finite() {
        Ok(objective)
    } else {
        Err(invalid_perturbation("baseline objective is non-finite"))
    }
}

fn can_solve(policy: &ConstraintPerturbationPolicy, started: &Instant, solve_count: usize) -> bool {
    solve_count < policy.max_solves
        && policy
            .time_budget
            .is_none_or(|budget| started.elapsed() < budget)
}

fn overall_status(
    observations: &[ConstraintPerturbationObservation],
    removal: Option<&RemovalTestObservation>,
    budget_exhausted: bool,
) -> AnalysisStatus {
    if observations
        .iter()
        .any(|observation| observation.status == AnalysisStatus::Unreachable)
        || removal.is_some_and(|observation| observation.status == AnalysisStatus::Unreachable)
    {
        return AnalysisStatus::Unreachable;
    }
    if budget_exhausted
        || observations
            .iter()
            .any(|observation| !observation.objective_proven)
        || removal.is_some_and(|observation| !observation.objective_proven)
    {
        return AnalysisStatus::Unknown;
    }
    if observations.is_empty() && removal.is_none() {
        AnalysisStatus::Unknown
    } else {
        AnalysisStatus::Reachable
    }
}

fn classify_adaptive_outcome(
    config: AdaptivePerturbationConfig,
    observations: &[ConstraintPerturbationObservation],
) -> AdaptivePerturbationOutcome {
    let effective = observations
        .iter()
        .find(|observation| observation.improved == Some(true))
        .map(|observation| observation.delta);
    let ineffective_before = effective.and_then(|upper| {
        observations
            .iter()
            .filter(|observation| observation.improved == Some(false) && observation.delta < upper)
            .map(|observation| observation.delta)
            .max_by(f64::total_cmp)
    });
    if let (Some(lower), Some(upper)) = (ineffective_before, effective)
        && config.refine_threshold
    {
        return AdaptivePerturbationOutcome::ThresholdInterval { lower, upper };
    }
    if let Some(delta) = effective {
        return AdaptivePerturbationOutcome::EffectiveAt { delta };
    }
    if observations
        .iter()
        .any(|observation| observation.improved.is_none())
    {
        return AdaptivePerturbationOutcome::Unknown;
    }
    let reached_limit = observations.last().is_some_and(|observation| {
        observation.delta + config.refinement_tolerance >= config.max_delta
    });
    if reached_limit {
        AdaptivePerturbationOutcome::NoObservedEffect
    } else {
        AdaptivePerturbationOutcome::UnknownDueToBudget
    }
}

fn cache_key<S: LinearSolver + ?Sized>(
    solver: &S,
    model: &LinearTriadModel,
    constraint_id: &StableConstraintId,
    baseline_values: &[f64],
    policy: &ConstraintPerturbationPolicy,
) -> Result<String> {
    let fingerprint = linear_model_fingerprint(model)?;
    let solver_identity = solver_descriptor_fingerprint(&solver.descriptor());
    let values = baseline_values
        .iter()
        .map(|value| value.to_bits().to_string())
        .collect::<Vec<_>>()
        .join(",");
    let deltas = policy
        .deltas
        .iter()
        .map(|value| value.to_bits().to_string())
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        solver_identity.value,
        fingerprint.value,
        constraint_id.0,
        values,
        deltas,
        policy
            .adaptive
            .map(|config| {
                format!(
                    "{}:{}:{}:{}:{}",
                    config.initial_delta.to_bits(),
                    config.growth_factor.to_bits(),
                    config.max_delta.to_bits(),
                    config.refine_threshold,
                    config.refinement_tolerance.to_bits()
                )
            })
            .unwrap_or_default(),
        policy.max_solves,
        policy.removal_test,
        policy
            .time_budget
            .map(|budget| budget.as_nanos().to_string())
            .unwrap_or_default(),
        policy.objective_tolerance.to_bits()
    ))
}

fn invalid_perturbation(message: impl Into<String>) -> CoreError {
    CoreError::Solver(SolverError::InvalidInput(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::intermediate::{BasicLinearTriadModel, SparseVector};
    use crate::solver::audit::attach_linear_model_mapping;
    use crate::solver::{
        SolveFingerprints, SolveProof, SolveReport, SolverCapability, SolverInfo, SolverOutput,
        SolverStatus,
    };
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableType};

    #[derive(Debug, Clone, Copy)]
    struct ReoptimizingSolver;

    impl SolverInfo for ReoptimizingSolver {
        fn name(&self) -> &str {
            "perturbation-test"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Mip]
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct TimeLimitedSolver;

    impl SolverInfo for TimeLimitedSolver {
        fn name(&self) -> &str {
            "perturbation-time-limit-test"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Mip]
        }
    }

    impl LinearSolver for TimeLimitedSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<crate::solver::SolverOutput> {
            Ok(crate::solver::SolverOutput::new(SolverStatus::TimeLimit)
                .with_solution(vec![5.0, 5.0])
                .with_objective(10.0))
        }
    }

    impl LinearSolver for ReoptimizingSolver {
        fn solve_linear(&self, model: &LinearTriadModel) -> Result<crate::solver::SolverOutput> {
            // 该小夹具为 x+y<=rhs 且最大化 x+y；此 solver 是确定性测试替身，仍通过生产兼容 API 返回报告。
            // The tiny fixture has x+y<=rhs and max x+y; this solver acts as a deterministic
            // test double while still returning a report through the production compatibility API.
            let rhs = model.basic.b.first().copied().unwrap_or(0.0);
            let x = rhs.clamp(model.basic.lb[0], model.basic.ub[0]);
            let y = (rhs - x).clamp(model.basic.lb[1], model.basic.ub[1]);
            let values = vec![x, y];
            let objective = model.c.iter().zip(&values).map(|(c, v)| c * v).sum();
            Ok(SolverOutput::new(SolverStatus::Optimal)
                .with_solution(values)
                .with_objective(objective))
        }

        fn solve_linear_report(&self, model: &LinearTriadModel) -> Result<SolveReport<f64>> {
            let rhs = model.basic.b.first().copied().unwrap_or(0.0);
            let x = rhs.clamp(model.basic.lb[0], model.basic.ub[0]);
            let y = (rhs - x).clamp(model.basic.lb[1], model.basic.ub[1]);
            let values = vec![x, y];
            let objective = model.c.iter().zip(&values).map(|(c, v)| c * v).sum();
            let mut solution = crate::solver::SolveSolution::vector(values);
            solution.objective = Some(objective);
            solution.objective_value = Some(objective);
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

    fn fixture() -> (LinearTriadModel, ConstraintId) {
        let mut basic = BasicLinearTriadModel::new("perturbation");
        for (index, name) in [(0, "x"), (1, "y")] {
            basic.add_variable_with_bounds(
                Token::from_generic(ContinuousVariableItem::auto(name), index),
                0.0,
                10.0,
                VariableType::Continuous,
            );
        }
        let mut row = SparseVector::new();
        row.add(0, 1.0);
        row.add(1, 1.0);
        basic.add_constraint_with_metadata(
            row,
            10.0,
            "capacity".to_owned(),
            Some(7),
            false,
            0,
            None,
            None,
        );
        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![1.0, 1.0], ObjectiveCategory::Maximum);
        let mapping = linear_model_mapping(&model).expect("mapping");
        (
            model,
            StableConstraintId(mapping.constraints_by_row[0].clone()),
        )
    }

    #[test]
    fn single_delta_records_objective_change_and_stable_identity() {
        let (model, id) = fixture();
        let policy = ConstraintPerturbationPolicy::single_delta(2.0);
        let report = ConstraintPerturbationAnalyzer::new()
            .analyze_with_policy(&ReoptimizingSolver, &model, &id, &[5.0, 5.0], &policy)
            .expect("perturbation report");
        assert_eq!(report.status, AnalysisStatus::Reachable);
        assert_eq!(report.observations.len(), 1);
        assert_eq!(report.observations[0].perturbed_rhs, 12.0);
        assert_eq!(report.observations[0].improvement, Some(2.0));
        assert_eq!(report.observations[0].improved, Some(true));
        assert!(report.constraint_id.0.contains("capacity"));
        assert!(report.validate().is_ok());
    }

    #[test]
    fn removal_is_separate_from_perturbation_and_tracks_integer_pattern_optionally() {
        let (model, id) = fixture();
        let policy = ConstraintPerturbationPolicy::single_delta(1.0).with_removal_test(true);
        let report = ConstraintPerturbationAnalyzer::new()
            .analyze_with_policy(&ReoptimizingSolver, &model, &id, &[5.0, 5.0], &policy)
            .expect("report");
        assert!(report.removal.is_some());
        assert_eq!(report.solve_count, 2);
        assert!(report.has_global_improvement());
    }

    #[test]
    fn adaptive_policy_finds_and_refines_first_effective_delta() {
        let (model, id) = fixture();
        let config = AdaptivePerturbationConfig {
            initial_delta: 1.0,
            growth_factor: 2.0,
            max_delta: 8.0,
            refine_threshold: true,
            refinement_tolerance: 0.5,
        };
        let policy = ConstraintPerturbationPolicy::adaptive(config).with_max_solves(20);
        let report = ConstraintPerturbationAnalyzer::new()
            .analyze_with_policy(&ReoptimizingSolver, &model, &id, &[5.0, 5.0], &policy)
            .expect("adaptive report");
        assert!(
            report
                .adaptive_outcome
                .is_some_and(AdaptivePerturbationOutcome::is_effective)
        );
        assert!(
            report
                .observations
                .iter()
                .any(|observation| observation.delta == 1.0)
        );
        assert_eq!(
            report.adaptive_outcome,
            Some(AdaptivePerturbationOutcome::EffectiveAt { delta: 1.0 })
        );
    }

    #[test]
    fn explicit_cache_reuses_report() {
        let (model, id) = fixture();
        let policy = ConstraintPerturbationPolicy::single_delta(1.0);
        let analyzer = ConstraintPerturbationAnalyzer::new();
        let mut cache = ConstraintPerturbationCache::new();
        analyzer
            .analyze_cached(
                &ReoptimizingSolver,
                &model,
                &id,
                &[5.0, 5.0],
                &policy,
                &mut cache,
            )
            .expect("first");
        analyzer
            .analyze_cached(
                &ReoptimizingSolver,
                &model,
                &id,
                &[5.0, 5.0],
                &policy,
                &mut cache,
            )
            .expect("second");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn time_limited_incumbent_never_becomes_a_proven_effect() {
        let (model, id) = fixture();
        let policy = ConstraintPerturbationPolicy::single_delta(1.0);
        let report = ConstraintPerturbationAnalyzer::new()
            .analyze_with_policy(&TimeLimitedSolver, &model, &id, &[5.0, 5.0], &policy)
            .expect("time-limited report");
        assert_eq!(report.status, AnalysisStatus::Unknown);
        // The solver returned an incumbent without proving optimality, so the observation is a
        // budget-limited result: it must stay Unknown rather than claim a proven reachability.
        // solver 返回了 incumbent 但未证明最优性，因此该观测属于受限结果：
        // 必须保持 Unknown，不得声称已证明的可达性。
        assert_eq!(report.observations[0].status, AnalysisStatus::Unknown);
        assert!(!report.observations[0].status.is_proven());
        assert!(!report.observations[0].objective_proven);
        assert_eq!(report.observations[0].improved, None);
        assert!(!report.has_global_improvement());
    }
}
