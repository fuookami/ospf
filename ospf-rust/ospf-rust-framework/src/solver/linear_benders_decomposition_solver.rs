//! Benders 分解求解器
//! Benders Decomposition Solver
//!
//! 本模块提供 Benders 分解算法的求解器接口。
//! This module provides solver interfaces for Benders decomposition algorithms.

use std::time::Instant;

use super::column_generation_solver::emit_combinatorial_progress;
use super::{
    BendersIterationSnapshot, BendersRuntimeMetrics, BendersStopReason, FeasibleSolution,
    FeasibleSolutionV, FrameworkSolveOptions, LinearDualSolution, LinearDualSolutionV,
};
use ospf_rust_core::error::{CoreError, Result, SolverError};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::intermediate::{LinearTriadModel, QuadraticTetradModel};
use ospf_rust_core::solver::{
    ModelFingerprint, ProgressValue, SolveHandle, SolveReport, SolveStage, SolveValue,
    SolveValueConversionPolicy, SolverOutput, SolverProvenance, TerminationReason,
    cancelled_solve_report, convert_report_value, linear_model_fingerprint,
    quadratic_model_fingerprint, require_infeasibility_certificate_for_model,
    require_optimal_lp_certificate_for_model,
};
#[cfg(test)]
use ospf_rust_core::solver::{require_infeasibility_certificate, require_optimal_lp_certificate};

/// 线性子问题结果 / Linear Sub-problem Result
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum LinearSubResult {
    /// 可行解 / Feasible solution
    Feasible(LinearFeasibleResult),
    /// 不可行解 / Infeasible solution
    Infeasible(LinearInfeasibleResult),
}

/// 线性子问题结果（typed）/ Linear sub-problem result (typed)
#[derive(Debug, Clone)]
pub enum LinearSubResultV<V>
where
    V: SolveValue,
{
    /// 可行解 / Feasible solution
    Feasible(LinearFeasibleResultV<V>),
    /// 不可行解 / Infeasible solution
    Infeasible(LinearInfeasibleResultV<V>),
}

/// 线性可行结果 / Linear Feasible Result
#[derive(Debug, Clone)]
pub struct LinearFeasibleResult {
    /// 求解结果 / Solver result
    pub result: FeasibleSolution,
    /// 对偶解 / Dual solution
    pub dual_solution: LinearDualSolution,
    /// Benders 切割 / Benders cuts
    pub cuts: Option<Vec<LinearCut>>,
    /// 产生该子结果的统一报告 / Unified report that produced this sub-result.
    pub report: Option<SolveReport<f64>>,
}

/// 线性可行结果（typed）/ Linear feasible result (typed)
#[derive(Debug, Clone)]
pub struct LinearFeasibleResultV<V>
where
    V: SolveValue,
{
    /// 求解结果 / Solver result
    pub result: FeasibleSolutionV<V>,
    /// 对偶解 / Dual solution
    pub dual_solution: LinearDualSolutionV<V>,
    /// Benders 切割 / Benders cuts
    pub cuts: Option<Vec<LinearCut>>,
    /// typed 统一报告 / Typed unified report.
    pub report: Option<SolveReport<V>>,
}

impl LinearFeasibleResult {
    /// 创建新的线性可行结果 / Create new linear feasible result
    pub fn new(result: FeasibleSolution, dual_solution: LinearDualSolution) -> Self {
        Self {
            result,
            dual_solution,
            cuts: None,
            report: None,
        }
    }

    /// 添加切割 / Add cuts
    pub fn with_cuts(mut self, cuts: Vec<LinearCut>) -> Self {
        self.cuts = Some(cuts);
        self
    }

    /// 绑定产生结果的统一报告 / Attach the unified report that produced this result.
    pub fn with_report(mut self, report: SolveReport<f64>) -> Self {
        self.result = self.result.clone().with_source_report(report.clone());
        self.report = Some(report);
        self
    }

    /// 转换为 typed 可行结果 / Convert into typed feasible result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<LinearFeasibleResultV<V>>
    where
        V: SolveValue,
    {
        Ok(LinearFeasibleResultV {
            result: self.result.try_into_typed(policy)?,
            dual_solution: self.dual_solution.try_into_typed(policy)?,
            cuts: self.cuts,
            report: self
                .report
                .map(|report| convert_report_value(report, policy))
                .transpose()?,
        })
    }
}

/// 线性不可行结果 / Linear Infeasible Result
#[derive(Debug, Clone)]
pub struct LinearInfeasibleResult {
    /// Farkas 对偶解 / Farkas dual solution
    pub farkas_dual_solution: LinearDualSolution,
    /// Benders 切割 / Benders cuts
    pub cuts: Option<Vec<LinearCut>>,
    /// 产生该子结果的统一报告 / Unified report that produced this sub-result.
    pub report: Option<SolveReport<f64>>,
}

/// 线性不可行结果（typed）/ Linear infeasible result (typed)
#[derive(Debug, Clone)]
pub struct LinearInfeasibleResultV<V>
where
    V: SolveValue,
{
    /// Farkas 对偶解 / Farkas dual solution
    pub farkas_dual_solution: LinearDualSolutionV<V>,
    /// Benders 切割 / Benders cuts
    pub cuts: Option<Vec<LinearCut>>,
    /// typed 统一报告 / Typed unified report.
    pub report: Option<SolveReport<V>>,
}

impl LinearInfeasibleResult {
    /// 创建新的线性不可行结果 / Create new linear infeasible result
    pub fn new(farkas_dual_solution: LinearDualSolution) -> Self {
        Self {
            farkas_dual_solution,
            cuts: None,
            report: None,
        }
    }

    /// 添加切割 / Add cuts
    pub fn with_cuts(mut self, cuts: Vec<LinearCut>) -> Self {
        self.cuts = Some(cuts);
        self
    }

    /// 绑定产生结果的统一报告 / Attach the unified report that produced this result.
    pub fn with_report(mut self, report: SolveReport<f64>) -> Self {
        self.report = Some(report);
        self
    }

    /// 转换为 typed 不可行结果 / Convert into typed infeasible result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<LinearInfeasibleResultV<V>>
    where
        V: SolveValue,
    {
        Ok(LinearInfeasibleResultV {
            farkas_dual_solution: self.farkas_dual_solution.try_into_typed(policy)?,
            cuts: self.cuts,
            report: self
                .report
                .map(|report| convert_report_value(report, policy))
                .transpose()?,
        })
    }
}

impl LinearSubResult {
    /// 转换为 typed 子结果 / Convert into typed sub result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<LinearSubResultV<V>>
    where
        V: SolveValue,
    {
        match self {
            LinearSubResult::Feasible(feasible) => {
                Ok(LinearSubResultV::Feasible(feasible.try_into_typed(policy)?))
            }
            LinearSubResult::Infeasible(infeasible) => Ok(LinearSubResultV::Infeasible(
                infeasible.try_into_typed(policy)?,
            )),
        }
    }
}

/// 线性切割 / Linear Cut
///
/// Benders 切割，用于添加到主问题中。
/// Benders cut, used to add to the master problem.
#[derive(Debug, Clone)]
pub struct LinearCut {
    /// 切割名称 / Cut name
    pub name: String,
    /// 切割系数 / Cut coefficients
    pub coefficients: Vec<(usize, f64)>,
    /// 切割右端项 / Cut right-hand side
    pub rhs: f64,
    /// 切割方向 / Cut sense (<=, >=, =)
    pub sense: CutSense,
}

impl LinearCut {
    /// 创建新的线性切割 / Create new linear cut
    pub fn new(
        name: impl Into<String>,
        coefficients: Vec<(usize, f64)>,
        rhs: f64,
        sense: CutSense,
    ) -> Self {
        Self {
            name: name.into(),
            coefficients,
            rhs,
            sense,
        }
    }

    /// 创建小于等于切割 / Create less-or-equal cut
    pub fn le(name: impl Into<String>, coefficients: Vec<(usize, f64)>, rhs: f64) -> Self {
        Self::new(name, coefficients, rhs, CutSense::LessOrEqual)
    }

    /// 创建大于等于切割 / Create greater-or-equal cut
    pub fn ge(name: impl Into<String>, coefficients: Vec<(usize, f64)>, rhs: f64) -> Self {
        Self::new(name, coefficients, rhs, CutSense::GreaterOrEqual)
    }

    /// 创建等式切割 / Create equality cut
    pub fn eq(name: impl Into<String>, coefficients: Vec<(usize, f64)>, rhs: f64) -> Self {
        Self::new(name, coefficients, rhs, CutSense::Equal)
    }
}

/// 切割方向 / Cut Sense
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutSense {
    /// 小于等于 / Less or equal
    LessOrEqual,
    /// 大于等于 / Greater or equal
    GreaterOrEqual,
    /// 等于 / Equal
    Equal,
}

fn build_benders_runtime_metrics(
    executed_iterations: usize,
    best_solution_iteration: Option<usize>,
    total_cuts: usize,
    no_cut_iterations: usize,
    no_obj_improvement_iterations: usize,
    stop_reason: Option<BendersStopReason>,
    iteration_snapshots: Vec<BendersIterationSnapshot>,
) -> BendersRuntimeMetrics {
    BendersRuntimeMetrics {
        executed_iterations,
        best_solution_iteration,
        total_cuts,
        no_cut_iterations,
        no_obj_improvement_iterations,
        stop_reason,
        iteration_snapshots,
    }
}

fn master_solution_from_report(report: &SolveReport<f64>) -> Result<Vec<f64>> {
    report.validate()?;
    if !report.is_optimal() {
        return Err(CoreError::contract_error(
            "Benders master must return a verified optimal report",
        ));
    }
    report
        .solution
        .as_ref()
        .map(|solution| solution.values.clone())
        .ok_or_else(|| CoreError::contract_error("Benders master report has no incumbent vector"))
}

fn master_objective_from_report(report: &SolveReport<f64>) -> Result<f64> {
    report.validate()?;
    if !report.is_optimal() {
        return Err(CoreError::contract_error(
            "Benders master must return a verified optimal report",
        ));
    }
    report
        .solution
        .as_ref()
        .and_then(|solution| solution.objective_value.or(solution.objective))
        .ok_or_else(|| CoreError::contract_error("Benders master report has no objective value"))
}

fn partial_master_solution_from_report(
    report: &SolveReport<f64>,
    executed_iterations: usize,
    total_cuts: usize,
    no_cut_iterations: usize,
    no_obj_improvement_iterations: usize,
    iteration_snapshots: Vec<BendersIterationSnapshot>,
) -> Result<FeasibleSolution> {
    report.validate()?;
    if report.is_optimal() || !report.has_incumbent() {
        return Err(CoreError::contract_error(
            "a partial Benders master result requires a non-optimal incumbent report",
        ));
    }
    let solution = FeasibleSolution::try_from_report(report)?;
    let metrics = build_benders_runtime_metrics(
        executed_iterations,
        None,
        total_cuts,
        no_cut_iterations,
        no_obj_improvement_iterations,
        Some(BendersStopReason::MasterTermination(
            report.termination_reason,
        )),
        iteration_snapshots,
    );
    Ok(solution
        .with_benders_iterations(executed_iterations)
        .with_benders_runtime_metrics(metrics))
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-8 * left.abs().max(right.abs()).max(1.0)
}

fn apply_master_bound(
    mut solution: FeasibleSolution,
    master_report: &SolveReport<f64>,
) -> (FeasibleSolution, bool) {
    let bound = master_report.statistics.best_bound_value;
    solution.possible_best_obj = bound;
    solution.gap = bound
        .map(|value| (solution.obj - value).abs() / solution.obj.abs().max(1.0))
        .unwrap_or(0.0);
    (solution, master_report.is_optimal() && bound.is_some())
}

fn benders_proof_reference(
    master_report: &SolveReport<f64>,
    subproblem_report: Option<&SolveReport<f64>>,
    subproblem_certificate: Option<&str>,
) -> Option<String> {
    if let Some(reference) = subproblem_certificate {
        return Some(reference.to_owned());
    }
    if master_report.is_optimal() {
        return Some("master-optimality-gate".to_owned());
    }
    subproblem_report.and_then(|report| report.proof.as_ref()?.reference.clone())
}

fn update_last_benders_snapshot_stop_reason(
    snapshots: &mut [BendersIterationSnapshot],
    stop_reason: Option<BendersStopReason>,
) {
    if let Some(snapshot) = snapshots.last_mut() {
        snapshot.stop_reason = stop_reason;
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_benders_progress(
    options: &FrameworkSolveOptions,
    phase: &str,
    iteration: usize,
    max_iterations: usize,
    stage_progress: f64,
    elapsed: std::time::Duration,
    objective_value: Option<f64>,
    best_bound: Option<f64>,
    terminal: bool,
) -> Result<()> {
    let overall_progress = if terminal || max_iterations == 0 {
        ProgressValue::known(100.0)?
    } else {
        ProgressValue::known(iteration as f64 / max_iterations as f64 * 100.0)?
    };
    let relative_gap = objective_value
        .zip(best_bound)
        .map(|(objective, bound)| (objective - bound).abs() / objective.abs().max(1.0));
    let (stage, stage_path) = if terminal {
        (
            SolveStage::Completed,
            vec!["benders".to_owned(), "completed".to_owned()],
        )
    } else {
        (
            SolveStage::Combinatorial,
            vec![
                "benders".to_owned(),
                format!("iteration/{iteration}"),
                phase.to_owned(),
            ],
        )
    };
    emit_combinatorial_progress(
        options,
        options.name.as_deref().unwrap_or("benders"),
        stage,
        stage_path,
        ProgressValue::known(stage_progress)?,
        overall_progress,
        elapsed,
        objective_value,
        best_bound,
        relative_gap,
        terminal,
    )
}

#[cfg(test)]
fn certified_linear_feasible_result(result: &LinearFeasibleResult) -> bool {
    let Some(report) = result.report.as_ref() else {
        return false;
    };
    let Ok(certificate) = require_optimal_lp_certificate(report) else {
        return false;
    };
    let Some(solution) = report.solution.as_ref() else {
        return false;
    };
    let Some(report_objective) = solution.objective_value.or(solution.objective) else {
        return false;
    };
    if !approximately_equal(report_objective, result.result.obj)
        || solution.values.len() != result.result.solution.len()
        || solution
            .values
            .iter()
            .zip(&result.result.solution)
            .any(|(left, right)| !approximately_equal(*left, *right))
        || certificate.dual.len() != result.dual_solution.constraints.len()
        || certificate
            .dual
            .iter()
            .zip(&result.dual_solution.constraints)
            .any(|(left, right)| !approximately_equal(*left, *right))
    {
        return false;
    }
    true
}

fn certified_linear_feasible_result_for_model(
    result: &LinearFeasibleResult,
    expected_model: &ModelFingerprint,
) -> bool {
    let Some(report) = result.report.as_ref() else {
        return false;
    };
    let Ok(certificate) = require_optimal_lp_certificate_for_model(report, expected_model) else {
        return false;
    };
    let Some(solution) = report.solution.as_ref() else {
        return false;
    };
    let Some(report_objective) = solution.objective_value.or(solution.objective) else {
        return false;
    };
    approximately_equal(report_objective, result.result.obj)
        && solution.values.len() == result.result.solution.len()
        && solution
            .values
            .iter()
            .zip(&result.result.solution)
            .all(|(left, right)| approximately_equal(*left, *right))
        && certificate.dual.len() == result.dual_solution.constraints.len()
        && certificate
            .dual
            .iter()
            .zip(&result.dual_solution.constraints)
            .all(|(left, right)| approximately_equal(*left, *right))
}

#[cfg(test)]
fn certified_linear_infeasible_result(result: &LinearInfeasibleResult) -> bool {
    let Some(report) = result.report.as_ref() else {
        return false;
    };
    if require_infeasibility_certificate(report).is_err() {
        return false;
    }
    let evidence = report
        .proof
        .as_ref()
        .and_then(|proof| proof.evidence.as_deref())
        .or_else(|| {
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.dual_solution.as_deref())
        });
    let Some(evidence) = evidence else {
        return false;
    };
    evidence.len() == result.farkas_dual_solution.constraints.len()
        && evidence
            .iter()
            .zip(&result.farkas_dual_solution.constraints)
            .all(|(left, right)| approximately_equal(*left, *right))
}

fn certified_linear_infeasible_result_for_model(
    result: &LinearInfeasibleResult,
    expected_model: &ModelFingerprint,
) -> bool {
    let Some(report) = result.report.as_ref() else {
        return false;
    };
    if require_infeasibility_certificate_for_model(report, expected_model).is_err() {
        return false;
    }
    let evidence = report
        .proof
        .as_ref()
        .and_then(|proof| proof.evidence.as_deref())
        .or_else(|| {
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.dual_solution.as_deref())
        });
    let Some(evidence) = evidence else {
        return false;
    };
    evidence.len() == result.farkas_dual_solution.constraints.len()
        && evidence
            .iter()
            .zip(&result.farkas_dual_solution.constraints)
            .all(|(left, right)| approximately_equal(*left, *right))
}

fn benders_cancelled(options: &FrameworkSolveOptions) -> bool {
    options
        .cancellation_handle
        .as_ref()
        .is_some_and(SolveHandle::is_cancelled)
}

fn benders_cancel_error() -> CoreError {
    CoreError::cancelled("BENDERS")
}

fn subproblem_termination_reason(report: Option<&SolveReport<f64>>) -> TerminationReason {
    report
        .map(|report| report.termination_reason)
        .unwrap_or(TerminationReason::BackendFailure)
}

fn benders_cancelled_report(
    solver_name: &str,
    options: &FrameworkSolveOptions,
) -> Result<SolveReport<f64>> {
    let handle = options.cancellation_handle.as_ref().ok_or_else(|| {
        CoreError::contract_error("Benders cancellation report requires a SolveHandle")
    })?;
    cancelled_solve_report(
        SolverProvenance {
            solver_id: solver_name.to_owned(),
            backend_name: "framework-benders".to_owned(),
            ..SolverProvenance::default()
        },
        handle,
    )
}

/// 线性 Benders 分解求解器 trait / Linear Benders Decomposition Solver Trait
///
/// 定义线性 Benders 分解算法的求解器接口。
/// Defines solver interface for linear Benders decomposition algorithms.
///
/// 推荐应用层入口：`solve_meta` 与 `solve_meta_with_options`。
/// Recommended application-facing entries are `solve_meta` and `solve_meta_with_options`.
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait LinearBendersDecompositionSolver: Send + Sync {
    /// 获取求解器名称 / Get solver name
    fn name(&self) -> &str;

    /// 求解主问题 / Solve master problem
    #[cfg(feature = "async")]
    async fn solve_master(
        &self,
        model: &LinearTriadModel,
        cuts: &[LinearCut],
    ) -> Result<SolverOutput>;

    /// 求解主问题（同步）/ Solve master problem (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_master(&self, model: &LinearTriadModel, cuts: &[LinearCut]) -> Result<SolverOutput>;

    /// 以统一报告求解主问题 / Solve the master problem as a unified report.
    ///
    /// 旧 `solve_master` 实现只提供兼容 facade，因此默认路径不会凭状态枚举
    /// 生成 optimality proof；原生 adapter 应覆盖此方法并直接返回 backend report。
    /// The legacy `solve_master` method only provides a compatibility facade, so the
    /// default path never invents an optimality proof from its status enum. Native
    /// adapters should override this method and return the backend report directly.
    #[cfg(feature = "async")]
    async fn solve_master_report(
        &self,
        model: &LinearTriadModel,
        cuts: &[LinearCut],
    ) -> Result<SolveReport<f64>> {
        self.solve_master(model, cuts)
            .await?
            .try_into_solve_report()
    }

    /// 以统一报告求解主问题（同步）/ Solve the master problem as a unified report (sync).
    #[cfg(not(feature = "async"))]
    fn solve_master_report(
        &self,
        model: &LinearTriadModel,
        cuts: &[LinearCut],
    ) -> Result<SolveReport<f64>> {
        self.solve_master(model, cuts)?.try_into_solve_report()
    }

    /// 使用统一选项求解 master report / Solve the master report with unified options.
    #[cfg(feature = "async")]
    async fn solve_master_report_with_options(
        &self,
        model: &LinearTriadModel,
        cuts: &[LinearCut],
        _options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_master_report(model, cuts).await
    }

    /// 使用统一选项求解 master report / Solve the master report with unified options.
    #[cfg(not(feature = "async"))]
    fn solve_master_report_with_options(
        &self,
        model: &LinearTriadModel,
        cuts: &[LinearCut],
        _options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_master_report(model, cuts)
    }

    /// 求解子问题 / Solve sub-problem
    #[cfg(feature = "async")]
    async fn solve_sub(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
    ) -> Result<LinearSubResult>;

    /// 求解子问题（typed，async）/ Solve sub-problem with typed output (async)
    #[cfg(feature = "async")]
    fn solve_sub_typed<'a, V>(
        &'a self,
        model: &'a LinearTriadModel,
        master_solution: &'a [f64],
        policy: SolveValueConversionPolicy,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LinearSubResultV<V>>> + Send + 'a>>
    where
        V: SolveValue,
    {
        Box::pin(async move {
            self.solve_sub(model, master_solution)
                .await?
                .try_into_typed(policy)
        })
    }

    /// 求解子问题（同步）/ Solve sub-problem (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_sub(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
    ) -> Result<LinearSubResult>;

    /// 求解子问题（typed，同步）/ Solve sub-problem with typed output (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_sub_typed<V>(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
        policy: SolveValueConversionPolicy,
    ) -> Result<LinearSubResultV<V>>
    where
        V: SolveValue,
    {
        self.solve_sub(model, master_solution)?
            .try_into_typed(policy)
    }

    /// 使用统一选项求解子问题 / Solve the sub-problem with unified options.
    #[cfg(not(feature = "async"))]
    fn solve_sub_with_options(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
        _options: &FrameworkSolveOptions,
    ) -> Result<LinearSubResult> {
        self.solve_sub(model, master_solution)
    }

    /// 使用统一选项求解子问题 / Solve the sub-problem with unified options.
    #[cfg(feature = "async")]
    async fn solve_sub_with_options(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
        _options: &FrameworkSolveOptions,
    ) -> Result<LinearSubResult> {
        self.solve_sub(model, master_solution).await
    }

    /// 使用 MetaModel 执行 Benders 分解（async）/
    /// Execute Benders decomposition from MetaModel (async)
    #[cfg(feature = "async")]
    fn solve_meta<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行 Benders 并返回统一报告 / Execute Benders from MetaModel and return a unified report.
    #[cfg(feature = "async")]
    fn solve_report<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SolveReport<V>>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_report_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用参数对象执行 Benders 并返回统一报告 / Execute Benders with options and return a unified report.
    #[cfg(feature = "async")]
    fn solve_report_with_options<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SolveReport<V>>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        let cancel_options = options.clone();
        let solve_future = self.solve_meta_with_options(master_meta_model, sub_meta_model, options);
        let solver_name = self.name().to_owned();
        Box::pin(async move {
            match solve_future.await {
                Ok(solution) => {
                    convert_report_value(solution.to_solve_report(&solver_name)?, policy)
                }
                Err(_error) if benders_cancelled(&cancel_options) => convert_report_value(
                    benders_cancelled_report(&solver_name, &cancel_options)?,
                    policy,
                ),
                Err(error) => Err(error),
            }
        })
    }

    /// 使用 MetaModel 执行 Benders 分解（简化参数对象，async）/
    /// Execute Benders decomposition from MetaModel with simplified options object (async)
    #[cfg(feature = "async")]
    fn solve_meta_with_options<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let master_mechanism_model = match master_meta_model
            .try_to_mechanism_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_mechanism_model = match sub_meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let master_mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &master_mechanism_model,
            options.value_conversion_policy,
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &sub_mechanism_model,
            options.value_conversion_policy,
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let master_model = match master_mechanism_model
            .try_into_linear_triad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_model = match sub_mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        Box::pin(async move {
            self.solve_with_options(&master_model, &sub_model, options)
                .await
        })
    }

    /// 使用 MetaModel 执行 Benders 分解并返回 typed 结果（async）/
    /// Execute Benders decomposition from MetaModel and return typed result (async)
    #[cfg(feature = "async")]
    fn solve_meta_typed<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<FeasibleSolutionV<V>>> + Send + 'a>,
    >
    where
        V: SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_typed_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行 Benders 分解并返回 typed 结果（参数对象，async）/
    /// Execute Benders decomposition from MetaModel and return typed result with options (async)
    #[cfg(feature = "async")]
    fn solve_meta_typed_with_options<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<FeasibleSolutionV<V>>> + Send + 'a>,
    >
    where
        V: SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        let solve_future = self.solve_meta_with_options(master_meta_model, sub_meta_model, options);
        Box::pin(async move { solve_future.await?.try_into_typed(policy) })
    }

    /// 使用 MetaModel 执行 Benders 分解（同步）/
    /// Execute Benders decomposition from MetaModel (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
    ) -> Result<FeasibleSolution>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行 Benders 并返回统一报告 / Execute Benders from MetaModel and return a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_report<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
    ) -> Result<SolveReport<V>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_report_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用参数对象执行 Benders 并返回统一报告 / Execute Benders with options and return a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_report_with_options<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<V>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        let cancel_options = options.clone();
        match self.solve_meta_with_options(master_meta_model, sub_meta_model, options) {
            Ok(solution) => convert_report_value(solution.to_solve_report(self.name())?, policy),
            Err(_error) if benders_cancelled(&cancel_options) => convert_report_value(
                benders_cancelled_report(self.name(), &cancel_options)?,
                policy,
            ),
            Err(error) => Err(error),
        }
    }

    /// 使用 MetaModel 执行 Benders 分解（简化参数对象，同步）/
    /// Execute Benders decomposition from MetaModel with simplified options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_with_options<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let master_mechanism_model = master_meta_model
            .try_to_mechanism_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            )?;
        let sub_mechanism_model = sub_meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        let master_mechanism_model = ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &master_mechanism_model,
            options.value_conversion_policy,
        )?;
        let sub_mechanism_model = ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &sub_mechanism_model,
            options.value_conversion_policy,
        )?;
        let master_model = master_mechanism_model
            .try_into_linear_triad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            )?;
        let sub_model = sub_mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        self.solve_with_options(&master_model, &sub_model, options)
    }

    /// 使用 MetaModel 执行 Benders 分解并返回 typed 结果（同步）/
    /// Execute Benders decomposition from MetaModel and return typed result (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_typed<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
    ) -> Result<FeasibleSolutionV<V>>
    where
        V: SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_typed_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行 Benders 分解并返回 typed 结果（参数对象，同步）/
    /// Execute Benders decomposition from MetaModel and return typed result with options (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_typed_with_options<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolutionV<V>>
    where
        V: SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        self.solve_meta_with_options(master_meta_model, sub_meta_model, options)?
            .try_into_typed(policy)
    }

    /// 使用参数对象执行 Benders 分解（async）/
    /// Execute Benders decomposition with options object (async)
    #[cfg(feature = "async")]
    fn solve_with_options<'a>(
        &'a self,
        master_model: &'a LinearTriadModel,
        sub_model: &'a LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    {
        Box::pin(async move {
            self.solve_with_options_impl(master_model, sub_model, options)
                .await
        })
    }

    /// 使用统一选项执行 Benders 迭代 / Execute Benders iterations with unified options.
    #[cfg(feature = "async")]
    async fn solve_with_options_impl(
        &self,
        master_model: &LinearTriadModel,
        sub_model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve_core_with_options(master_model, sub_model, options)
            .await
    }

    /// 使用参数对象执行 Benders 分解（同步）/
    /// Execute Benders decomposition with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_with_options(
        &self,
        master_model: &LinearTriadModel,
        sub_model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve_with_options_impl(master_model, sub_model, options)
    }

    /// 使用统一选项执行 Benders 迭代 / Execute Benders iterations with unified options.
    #[cfg(not(feature = "async"))]
    fn solve_with_options_impl(
        &self,
        master_model: &LinearTriadModel,
        sub_model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve_core_with_options(master_model, sub_model, options)
    }

    /// 执行 Benders 分解迭代 / Execute Benders decomposition iteration
    #[cfg(feature = "async")]
    async fn solve(
        &self,
        master_model: &LinearTriadModel,
        sub_model: &LinearTriadModel,
        max_iterations: usize,
        tolerance: f64,
        max_stall_iterations: Option<usize>,
        objective_stall_iterations: Option<usize>,
    ) -> Result<FeasibleSolution> {
        let mut options =
            FrameworkSolveOptions::default().with_iterations(max_iterations, tolerance);
        options.max_stall_iterations = max_stall_iterations;
        options.objective_stall_iterations = objective_stall_iterations;
        self.solve_core_with_options(master_model, sub_model, options)
            .await
    }

    /// 带统一选项的线性 Benders 核心循环 / Linear Benders core loop with unified options.
    #[cfg(feature = "async")]
    async fn solve_core_with_options(
        &self,
        master_model: &LinearTriadModel,
        sub_model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        let max_iterations = options.max_iterations;
        let tolerance = options.tolerance;
        let max_stall_iterations = options.max_stall_iterations;
        let objective_stall_iterations = options.objective_stall_iterations;
        let mut cuts = Vec::new();
        let mut best_solution: Option<FeasibleSolution> = None;
        let mut best_solution_iterations: Option<usize> = None;
        let mut prev_obj = f64::NEG_INFINITY;
        let max_stall_iterations = max_stall_iterations.filter(|window| *window > 0);
        let objective_stall_iterations = objective_stall_iterations.unwrap_or(1).max(1);
        let mut no_cut_iterations = 0usize;
        let mut no_obj_improvement_iterations = 0usize;
        let mut executed_iterations = 0usize;
        let mut stop_reason: Option<BendersStopReason> = None;
        let mut iteration_snapshots: Vec<BendersIterationSnapshot> = Vec::new();
        let begin = Instant::now();

        for iter in 0..max_iterations {
            executed_iterations = iter + 1;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }
            emit_benders_progress(
                &options,
                "master",
                executed_iterations,
                max_iterations,
                0.0,
                begin.elapsed(),
                None,
                None,
                false,
            )?;
            // 求解主问题 / Solve master problem
            let master_report = self
                .solve_master_report_with_options(master_model, &cuts, &options)
                .await?;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }

            let master_solution = match master_solution_from_report(&master_report) {
                Ok(solution) => solution,
                Err(_error) if master_report.has_incumbent() => {
                    return partial_master_solution_from_report(
                        &master_report,
                        executed_iterations,
                        cuts.len(),
                        no_cut_iterations,
                        no_obj_improvement_iterations,
                        iteration_snapshots,
                    );
                }
                Err(error) => return Err(error),
            };
            let master_obj = master_objective_from_report(&master_report)?;

            if (master_obj - prev_obj).abs() < tolerance {
                no_obj_improvement_iterations += 1;
            } else {
                no_obj_improvement_iterations = 0;
            }
            prev_obj = master_obj;

            // 求解子问题 / Solve sub-problem
            let cuts_before = cuts.len();
            let sub_result = self
                .solve_sub_with_options(sub_model, &master_solution, &options)
                .await?;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }
            let sub_model_fingerprint = linear_model_fingerprint(sub_model)?;
            let subproblem_report;
            let mut subproblem_certificate = None;

            match sub_result {
                LinearSubResult::Feasible(feasible) => {
                    // 更新最优解 / Update best solution
                    subproblem_report = feasible.report.clone();
                    let (candidate, _) =
                        apply_master_bound(feasible.result.clone(), &master_report);
                    best_solution = Some(candidate);
                    best_solution_iterations = Some(iter + 1);

                    // 只有通过统一最优 LP/dual 证书的结果才能产生 exact cut。
                    // Only a result with a verified unified optimal-LP/dual certificate may produce an exact cut.
                    let certified = certified_linear_feasible_result_for_model(
                        &feasible,
                        &sub_model_fingerprint,
                    );
                    if certified {
                        subproblem_certificate = Some("subproblem-optimality");
                        if let Some(new_cuts) = feasible.cuts {
                            cuts.extend(new_cuts);
                        }
                    } else {
                        stop_reason = Some(BendersStopReason::SubproblemTermination(
                            subproblem_termination_reason(feasible.report.as_ref()),
                        ));
                    }
                }
                LinearSubResult::Infeasible(infeasible) => {
                    subproblem_report = infeasible.report.clone();
                    // 只有 verified Farkas/evidence 才能产生可行性切割。
                    // Only verified Farkas/evidence may produce a feasibility cut.
                    if certified_linear_infeasible_result_for_model(
                        &infeasible,
                        &sub_model_fingerprint,
                    ) {
                        subproblem_certificate = Some("subproblem-infeasibility");
                        if let Some(new_cuts) = infeasible.cuts {
                            cuts.extend(new_cuts);
                        }
                    } else {
                        return Err(CoreError::contract_error(
                            "Benders infeasible subproblem lacks a verified Farkas certificate",
                        ));
                    }
                }
            }
            let cuts_added = cuts.len().saturating_sub(cuts_before);

            if cuts.len() > cuts_before {
                no_cut_iterations = 0;
            } else {
                no_cut_iterations += 1;
            }
            iteration_snapshots.push(BendersIterationSnapshot {
                iteration: iter + 1,
                master_obj,
                master_gap: master_report.statistics.relative_gap,
                master_bound: master_report.statistics.best_bound_value,
                cuts_added,
                total_cuts: cuts.len(),
                no_cut_iterations,
                no_obj_improvement_iterations,
                master_attempt_id: format!("benders/iteration/{}/master", iter + 1),
                subproblem_attempt_id: Some(format!("benders/iteration/{}/subproblem", iter + 1)),
                master_report: Some(master_report.clone()),
                subproblem_report: subproblem_report.clone(),
                bound_valid: master_report.is_optimal()
                    && master_report.statistics.best_bound_value.is_some(),
                proof_reference: benders_proof_reference(
                    &master_report,
                    subproblem_report.as_ref(),
                    subproblem_certificate,
                ),
                stop_reason,
            });
            emit_benders_progress(
                &options,
                "subproblem",
                executed_iterations,
                max_iterations,
                100.0,
                begin.elapsed(),
                Some(master_obj),
                master_report.statistics.best_bound_value,
                false,
            )?;
            if matches!(
                stop_reason,
                Some(BendersStopReason::SubproblemTermination(_))
            ) {
                break;
            }
            if let Some(window) = max_stall_iterations
                && no_cut_iterations >= window
            {
                stop_reason = Some(BendersStopReason::CutStall);
                update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
                log::info!(
                    "Benders decomposition stopped at iteration {} due to cut stagnation window {}",
                    iter + 1,
                    window
                );
                break;
            }

            if no_obj_improvement_iterations >= objective_stall_iterations {
                stop_reason = Some(BendersStopReason::ObjectiveStall);
                update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
                log::info!(
                    "Benders decomposition stopped at iteration {} due to objective stall window {}",
                    iter + 1,
                    objective_stall_iterations
                );
                break;
            }
        }

        if stop_reason.is_none() && executed_iterations >= max_iterations && max_iterations > 0 {
            stop_reason = Some(BendersStopReason::IterationLimit);
            update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
        }

        let best_objective = best_solution.as_ref().map(|solution| solution.obj);
        let best_bound = best_solution
            .as_ref()
            .and_then(|solution| solution.possible_best_obj);
        emit_benders_progress(
            &options,
            "completed",
            executed_iterations,
            max_iterations,
            100.0,
            begin.elapsed(),
            best_objective,
            best_bound,
            true,
        )?;

        match best_solution {
            Some(solution) => {
                let runtime_metrics = build_benders_runtime_metrics(
                    executed_iterations,
                    best_solution_iterations,
                    cuts.len(),
                    no_cut_iterations,
                    no_obj_improvement_iterations,
                    stop_reason,
                    iteration_snapshots,
                );
                Ok(match best_solution_iterations {
                    Some(iterations) => solution
                        .with_benders_iterations(iterations)
                        .with_benders_runtime_metrics(runtime_metrics),
                    None => solution.with_benders_runtime_metrics(runtime_metrics),
                })
            }
            None => Err(CoreError::Solver(SolverError::NoSolution)),
        }
    }

    /// 执行 Benders 分解迭代（同步）/ Execute Benders decomposition iteration (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve(
        &self,
        master_model: &LinearTriadModel,
        sub_model: &LinearTriadModel,
        max_iterations: usize,
        tolerance: f64,
        max_stall_iterations: Option<usize>,
        objective_stall_iterations: Option<usize>,
    ) -> Result<FeasibleSolution> {
        let mut options =
            FrameworkSolveOptions::default().with_iterations(max_iterations, tolerance);
        options.max_stall_iterations = max_stall_iterations;
        options.objective_stall_iterations = objective_stall_iterations;
        self.solve_core_with_options(master_model, sub_model, options)
    }

    /// 带统一选项的线性 Benders 核心循环 / Linear Benders core loop with unified options.
    #[cfg(not(feature = "async"))]
    fn solve_core_with_options(
        &self,
        master_model: &LinearTriadModel,
        sub_model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        let max_iterations = options.max_iterations;
        let tolerance = options.tolerance;
        let max_stall_iterations = options.max_stall_iterations;
        let objective_stall_iterations = options.objective_stall_iterations;
        let mut cuts = Vec::new();
        let mut best_solution: Option<FeasibleSolution> = None;
        let mut best_solution_iterations: Option<usize> = None;
        let mut prev_obj = f64::NEG_INFINITY;
        let max_stall_iterations = max_stall_iterations.filter(|window| *window > 0);
        let objective_stall_iterations = objective_stall_iterations.unwrap_or(1).max(1);
        let mut no_cut_iterations = 0usize;
        let mut no_obj_improvement_iterations = 0usize;
        let mut executed_iterations = 0usize;
        let mut stop_reason: Option<BendersStopReason> = None;
        let mut iteration_snapshots: Vec<BendersIterationSnapshot> = Vec::new();
        let begin = Instant::now();

        for iter in 0..max_iterations {
            executed_iterations = iter + 1;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }
            emit_benders_progress(
                &options,
                "master",
                executed_iterations,
                max_iterations,
                0.0,
                begin.elapsed(),
                None,
                None,
                false,
            )?;
            // 求解主问题 / Solve master problem
            let master_report =
                self.solve_master_report_with_options(master_model, &cuts, &options)?;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }

            let master_solution = match master_solution_from_report(&master_report) {
                Ok(solution) => solution,
                Err(_error) if master_report.has_incumbent() => {
                    return partial_master_solution_from_report(
                        &master_report,
                        executed_iterations,
                        cuts.len(),
                        no_cut_iterations,
                        no_obj_improvement_iterations,
                        iteration_snapshots,
                    );
                }
                Err(error) => return Err(error),
            };
            let master_obj = master_objective_from_report(&master_report)?;

            if (master_obj - prev_obj).abs() < tolerance {
                no_obj_improvement_iterations += 1;
            } else {
                no_obj_improvement_iterations = 0;
            }
            prev_obj = master_obj;

            // 求解子问题 / Solve sub-problem
            let cuts_before = cuts.len();
            let sub_result = self.solve_sub_with_options(sub_model, &master_solution, &options)?;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }
            let sub_model_fingerprint = linear_model_fingerprint(sub_model)?;
            let subproblem_report;
            let mut subproblem_certificate = None;

            match sub_result {
                LinearSubResult::Feasible(feasible) => {
                    // 更新最优解 / Update best solution
                    subproblem_report = feasible.report.clone();
                    let (candidate, _) =
                        apply_master_bound(feasible.result.clone(), &master_report);
                    best_solution = Some(candidate);
                    best_solution_iterations = Some(iter + 1);

                    // 只有通过统一最优 LP/dual 证书的结果才能产生 exact cut。
                    // Only a result with a verified unified optimal-LP/dual certificate may produce an exact cut.
                    let certified = certified_linear_feasible_result_for_model(
                        &feasible,
                        &sub_model_fingerprint,
                    );
                    if certified {
                        subproblem_certificate = Some("subproblem-optimality");
                        if let Some(new_cuts) = feasible.cuts {
                            cuts.extend(new_cuts);
                        }
                    } else {
                        stop_reason = Some(BendersStopReason::SubproblemTermination(
                            subproblem_termination_reason(feasible.report.as_ref()),
                        ));
                    }
                }
                LinearSubResult::Infeasible(infeasible) => {
                    subproblem_report = infeasible.report.clone();
                    // 只有 verified Farkas/evidence 才能产生可行性切割。
                    // Only verified Farkas/evidence may produce a feasibility cut.
                    if certified_linear_infeasible_result_for_model(
                        &infeasible,
                        &sub_model_fingerprint,
                    ) {
                        subproblem_certificate = Some("subproblem-infeasibility");
                        if let Some(new_cuts) = infeasible.cuts {
                            cuts.extend(new_cuts);
                        }
                    } else {
                        return Err(CoreError::contract_error(
                            "Benders infeasible subproblem lacks a verified Farkas certificate",
                        ));
                    }
                }
            }
            let cuts_added = cuts.len().saturating_sub(cuts_before);

            if cuts.len() > cuts_before {
                no_cut_iterations = 0;
            } else {
                no_cut_iterations += 1;
            }
            iteration_snapshots.push(BendersIterationSnapshot {
                iteration: iter + 1,
                master_obj,
                master_gap: master_report.statistics.relative_gap,
                master_bound: master_report.statistics.best_bound_value,
                cuts_added,
                total_cuts: cuts.len(),
                no_cut_iterations,
                no_obj_improvement_iterations,
                master_attempt_id: format!("benders/iteration/{}/master", iter + 1),
                subproblem_attempt_id: Some(format!("benders/iteration/{}/subproblem", iter + 1)),
                master_report: Some(master_report.clone()),
                subproblem_report: subproblem_report.clone(),
                bound_valid: master_report.is_optimal()
                    && master_report.statistics.best_bound_value.is_some(),
                proof_reference: benders_proof_reference(
                    &master_report,
                    subproblem_report.as_ref(),
                    subproblem_certificate,
                ),
                stop_reason,
            });
            emit_benders_progress(
                &options,
                "subproblem",
                executed_iterations,
                max_iterations,
                100.0,
                begin.elapsed(),
                Some(master_obj),
                master_report.statistics.best_bound_value,
                false,
            )?;
            if matches!(
                stop_reason,
                Some(BendersStopReason::SubproblemTermination(_))
            ) {
                break;
            }
            if let Some(window) = max_stall_iterations
                && no_cut_iterations >= window
            {
                stop_reason = Some(BendersStopReason::CutStall);
                update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
                log::info!(
                    "Benders decomposition stopped at iteration {} due to cut stagnation window {}",
                    iter + 1,
                    window
                );
                break;
            }

            if no_obj_improvement_iterations >= objective_stall_iterations {
                stop_reason = Some(BendersStopReason::ObjectiveStall);
                update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
                log::info!(
                    "Benders decomposition stopped at iteration {} due to objective stall window {}",
                    iter + 1,
                    objective_stall_iterations
                );
                break;
            }
        }

        if stop_reason.is_none() && executed_iterations >= max_iterations && max_iterations > 0 {
            stop_reason = Some(BendersStopReason::IterationLimit);
            update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
        }

        let best_objective = best_solution.as_ref().map(|solution| solution.obj);
        let best_bound = best_solution
            .as_ref()
            .and_then(|solution| solution.possible_best_obj);
        emit_benders_progress(
            &options,
            "completed",
            executed_iterations,
            max_iterations,
            100.0,
            begin.elapsed(),
            best_objective,
            best_bound,
            true,
        )?;

        match best_solution {
            Some(solution) => {
                let runtime_metrics = build_benders_runtime_metrics(
                    executed_iterations,
                    best_solution_iterations,
                    cuts.len(),
                    no_cut_iterations,
                    no_obj_improvement_iterations,
                    stop_reason,
                    iteration_snapshots,
                );
                Ok(match best_solution_iterations {
                    Some(iterations) => solution
                        .with_benders_iterations(iterations)
                        .with_benders_runtime_metrics(runtime_metrics),
                    None => solution.with_benders_runtime_metrics(runtime_metrics),
                })
            }
            None => Err(CoreError::Solver(SolverError::NoSolution)),
        }
    }
}

/// 二次切割 / Quadratic Cut
///
/// 扩展线性切割，支持二次项。
/// Extends linear cut to support quadratic terms.
#[derive(Debug, Clone)]
pub struct QuadraticCut {
    /// 基础线性切割 / Base linear cut
    pub linear: LinearCut,
    /// 二次项系数 / Quadratic coefficients
    pub quadratic_coefficients: Vec<((usize, usize), f64)>,
}

impl QuadraticCut {
    /// 创建新的二次切割 / Create new quadratic cut
    pub fn new(
        name: impl Into<String>,
        linear_coefficients: Vec<(usize, f64)>,
        quadratic_coefficients: Vec<((usize, usize), f64)>,
        rhs: f64,
        sense: CutSense,
    ) -> Self {
        Self {
            linear: LinearCut::new(name, linear_coefficients, rhs, sense),
            quadratic_coefficients,
        }
    }
}

/// 二次子问题结果 / Quadratic Sub-problem Result
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum QuadraticSubResult {
    /// 可行解 / Feasible solution
    Feasible(QuadraticFeasibleResult),
    /// 不可行解 / Infeasible solution
    Infeasible(QuadraticInfeasibleResult),
}

/// 二次子问题结果（typed）/ Quadratic sub-problem result (typed)
#[derive(Debug, Clone)]
pub enum QuadraticSubResultV<V>
where
    V: SolveValue,
{
    /// 可行解 / Feasible solution
    Feasible(QuadraticFeasibleResultV<V>),
    /// 不可行解 / Infeasible solution
    Infeasible(QuadraticInfeasibleResultV<V>),
}

/// 二次可行结果 / Quadratic Feasible Result
#[derive(Debug, Clone)]
pub struct QuadraticFeasibleResult {
    /// 线性可行结果 / Linear feasible result
    pub linear: LinearFeasibleResult,
    /// 二次切割 / Quadratic cuts
    pub quadratic_cuts: Option<Vec<QuadraticCut>>,
}

/// 二次可行结果（typed）/ Quadratic feasible result (typed)
#[derive(Debug, Clone)]
pub struct QuadraticFeasibleResultV<V>
where
    V: SolveValue,
{
    /// 线性可行结果 / Linear feasible result
    pub linear: LinearFeasibleResultV<V>,
    /// 二次切割 / Quadratic cuts
    pub quadratic_cuts: Option<Vec<QuadraticCut>>,
}

impl QuadraticFeasibleResult {
    /// 创建新的二次可行结果 / Create new quadratic feasible result
    pub fn new(result: LinearFeasibleResult) -> Self {
        Self {
            linear: result,
            quadratic_cuts: None,
        }
    }

    /// 添加二次切割 / Add quadratic cuts
    pub fn with_quadratic_cuts(mut self, cuts: Vec<QuadraticCut>) -> Self {
        self.quadratic_cuts = Some(cuts);
        self
    }

    /// 转换为 typed 可行结果 / Convert into typed feasible result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<QuadraticFeasibleResultV<V>>
    where
        V: SolveValue,
    {
        Ok(QuadraticFeasibleResultV {
            linear: self.linear.try_into_typed(policy)?,
            quadratic_cuts: self.quadratic_cuts,
        })
    }
}

/// 二次不可行结果 / Quadratic Infeasible Result
#[derive(Debug, Clone)]
pub struct QuadraticInfeasibleResult {
    /// 线性不可行结果 / Linear infeasible result
    pub linear: LinearInfeasibleResult,
    /// 二次切割 / Quadratic cuts
    pub quadratic_cuts: Option<Vec<QuadraticCut>>,
}

/// 二次不可行结果（typed）/ Quadratic infeasible result (typed)
#[derive(Debug, Clone)]
pub struct QuadraticInfeasibleResultV<V>
where
    V: SolveValue,
{
    /// 线性不可行结果 / Linear infeasible result
    pub linear: LinearInfeasibleResultV<V>,
    /// 二次切割 / Quadratic cuts
    pub quadratic_cuts: Option<Vec<QuadraticCut>>,
}

impl QuadraticInfeasibleResult {
    /// 创建新的二次不可行结果 / Create new quadratic infeasible result
    pub fn new(result: LinearInfeasibleResult) -> Self {
        Self {
            linear: result,
            quadratic_cuts: None,
        }
    }

    /// 添加二次切割 / Add quadratic cuts
    pub fn with_quadratic_cuts(mut self, cuts: Vec<QuadraticCut>) -> Self {
        self.quadratic_cuts = Some(cuts);
        self
    }

    /// 转换为 typed 不可行结果 / Convert into typed infeasible result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<QuadraticInfeasibleResultV<V>>
    where
        V: SolveValue,
    {
        Ok(QuadraticInfeasibleResultV {
            linear: self.linear.try_into_typed(policy)?,
            quadratic_cuts: self.quadratic_cuts,
        })
    }
}

impl QuadraticSubResult {
    /// 转换为 typed 子结果 / Convert into typed sub result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<QuadraticSubResultV<V>>
    where
        V: SolveValue,
    {
        match self {
            QuadraticSubResult::Feasible(feasible) => Ok(QuadraticSubResultV::Feasible(
                feasible.try_into_typed(policy)?,
            )),
            QuadraticSubResult::Infeasible(infeasible) => Ok(QuadraticSubResultV::Infeasible(
                infeasible.try_into_typed(policy)?,
            )),
        }
    }
}

/// 二次 Benders 分解求解器 trait / Quadratic Benders Decomposition Solver Trait
///
/// 在 `LinearBendersDecompositionSolver` 基础上扩展二次主问题/子问题接口。
/// Extends `LinearBendersDecompositionSolver` with quadratic master/sub interfaces.
///
/// 推荐应用层入口：`solve_meta_quadratic` 与 `solve_meta_quadratic_with_options`。
/// Recommended application-facing entries are `solve_meta_quadratic` and `solve_meta_quadratic_with_options`.
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait QuadraticBendersDecompositionSolver: LinearBendersDecompositionSolver {
    /// 求解二次主问题 / Solve quadratic master problem
    #[cfg(feature = "async")]
    async fn solve_master_quadratic(
        &self,
        model: &QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
    ) -> Result<SolverOutput>;

    /// 求解二次主问题（同步）/ Solve quadratic master problem (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_master_quadratic(
        &self,
        model: &QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
    ) -> Result<SolverOutput>;

    /// 以统一报告求解二次主问题 / Solve the quadratic master problem as a unified report.
    #[cfg(feature = "async")]
    async fn solve_master_quadratic_report(
        &self,
        model: &QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
    ) -> Result<SolveReport<f64>> {
        self.solve_master_quadratic(model, linear_cuts, quadratic_cuts)
            .await?
            .try_into_solve_report()
    }

    /// 以统一报告求解二次主问题（同步）/ Solve the quadratic master as a unified report (sync).
    #[cfg(not(feature = "async"))]
    fn solve_master_quadratic_report(
        &self,
        model: &QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
    ) -> Result<SolveReport<f64>> {
        self.solve_master_quadratic(model, linear_cuts, quadratic_cuts)?
            .try_into_solve_report()
    }

    /// 使用统一选项求解二次 master report / Solve the quadratic master report with unified options.
    #[cfg(feature = "async")]
    async fn solve_master_quadratic_report_with_options(
        &self,
        model: &QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
        _options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_master_quadratic_report(model, linear_cuts, quadratic_cuts)
            .await
    }

    /// 使用统一选项求解二次 master report / Solve the quadratic master report with unified options.
    #[cfg(not(feature = "async"))]
    fn solve_master_quadratic_report_with_options(
        &self,
        model: &QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
        _options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_master_quadratic_report(model, linear_cuts, quadratic_cuts)
    }

    /// 求解二次子问题 / Solve quadratic sub-problem
    #[cfg(feature = "async")]
    async fn solve_sub_quadratic(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
    ) -> Result<QuadraticSubResult>;

    /// 求解二次子问题（typed，async）/
    /// Solve quadratic sub-problem with typed output (async)
    #[cfg(feature = "async")]
    fn solve_sub_quadratic_typed<'a, V>(
        &'a self,
        model: &'a QuadraticTetradModel,
        master_solution: &'a [f64],
        policy: SolveValueConversionPolicy,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<QuadraticSubResultV<V>>> + Send + 'a>,
    >
    where
        V: SolveValue,
    {
        Box::pin(async move {
            self.solve_sub_quadratic(model, master_solution)
                .await?
                .try_into_typed(policy)
        })
    }

    /// 求解二次子问题（同步）/ Solve quadratic sub-problem (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_sub_quadratic(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
    ) -> Result<QuadraticSubResult>;

    /// 求解二次子问题（typed，同步）/
    /// Solve quadratic sub-problem with typed output (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_sub_quadratic_typed<V>(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
        policy: SolveValueConversionPolicy,
    ) -> Result<QuadraticSubResultV<V>>
    where
        V: SolveValue,
    {
        self.solve_sub_quadratic(model, master_solution)?
            .try_into_typed(policy)
    }

    /// 使用统一选项求解二次子问题 / Solve the quadratic sub-problem with unified options.
    #[cfg(not(feature = "async"))]
    fn solve_sub_quadratic_with_options(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
        _options: &FrameworkSolveOptions,
    ) -> Result<QuadraticSubResult> {
        self.solve_sub_quadratic(model, master_solution)
    }

    /// 使用统一选项求解二次子问题 / Solve the quadratic sub-problem with unified options.
    #[cfg(feature = "async")]
    async fn solve_sub_quadratic_with_options(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
        _options: &FrameworkSolveOptions,
    ) -> Result<QuadraticSubResult> {
        self.solve_sub_quadratic(model, master_solution).await
    }

    /// 使用 MetaModel 执行二次 Benders 分解（async）/
    /// Execute quadratic Benders decomposition from MetaModel (async)
    #[cfg(feature = "async")]
    fn solve_meta_quadratic<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_quadratic_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行二次 Benders 并返回统一报告 / Execute quadratic Benders and return a unified report.
    #[cfg(feature = "async")]
    fn solve_report_quadratic<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SolveReport<V>>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_report_quadratic_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用参数对象执行二次 Benders 并返回统一报告 / Execute quadratic Benders with options and return a unified report.
    #[cfg(feature = "async")]
    fn solve_report_quadratic_with_options<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SolveReport<V>>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        let cancel_options = options.clone();
        let solve_future =
            self.solve_meta_quadratic_with_options(master_meta_model, sub_meta_model, options);
        let solver_name = self.name().to_owned();
        Box::pin(async move {
            match solve_future.await {
                Ok(solution) => {
                    convert_report_value(solution.to_solve_report(&solver_name)?, policy)
                }
                Err(_error) if benders_cancelled(&cancel_options) => convert_report_value(
                    benders_cancelled_report(&solver_name, &cancel_options)?,
                    policy,
                ),
                Err(error) => Err(error),
            }
        })
    }

    /// 使用 MetaModel 执行二次 Benders 分解（简化参数对象，async）/
    /// Execute quadratic Benders decomposition from MetaModel with simplified options object (async)
    #[cfg(feature = "async")]
    fn solve_meta_quadratic_with_options<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let master_mechanism_model = match master_meta_model
            .try_to_mechanism_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_mechanism_model = match sub_meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let master_mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &master_mechanism_model,
            options.value_conversion_policy,
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &sub_mechanism_model,
            options.value_conversion_policy,
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let master_model = match master_mechanism_model
            .try_into_quadratic_tetrad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_model = match sub_mechanism_model
            .try_into_quadratic_tetrad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        Box::pin(async move {
            self.solve_quadratic_with_options(&master_model, &sub_model, options)
                .await
        })
    }

    /// 使用 MetaModel 执行二次 Benders 分解并返回 typed 结果（async）/
    /// Execute quadratic Benders decomposition from MetaModel and return typed result (async)
    #[cfg(feature = "async")]
    fn solve_meta_quadratic_typed<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<FeasibleSolutionV<V>>> + Send + 'a>,
    >
    where
        V: SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_quadratic_typed_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行二次 Benders 分解并返回 typed 结果（参数对象，async）/
    /// Execute quadratic Benders decomposition from MetaModel and return typed result with options (async)
    #[cfg(feature = "async")]
    fn solve_meta_quadratic_typed_with_options<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<FeasibleSolutionV<V>>> + Send + 'a>,
    >
    where
        V: SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        let solve_future =
            self.solve_meta_quadratic_with_options(master_meta_model, sub_meta_model, options);
        Box::pin(async move { solve_future.await?.try_into_typed(policy) })
    }

    /// 使用 MetaModel 执行二次 Benders 分解（同步）/
    /// Execute quadratic Benders decomposition from MetaModel (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_quadratic<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
    ) -> Result<FeasibleSolution>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_quadratic_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行二次 Benders 并返回统一报告 / Execute quadratic Benders and return a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_report_quadratic<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
    ) -> Result<SolveReport<V>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_report_quadratic_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用参数对象执行二次 Benders 并返回统一报告 / Execute quadratic Benders with options and return a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_report_quadratic_with_options<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<V>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        let cancel_options = options.clone();
        match self.solve_meta_quadratic_with_options(master_meta_model, sub_meta_model, options) {
            Ok(solution) => convert_report_value(solution.to_solve_report(self.name())?, policy),
            Err(_error) if benders_cancelled(&cancel_options) => convert_report_value(
                benders_cancelled_report(self.name(), &cancel_options)?,
                policy,
            ),
            Err(error) => Err(error),
        }
    }

    /// 使用 MetaModel 执行二次 Benders 分解（简化参数对象，同步）/
    /// Execute quadratic Benders decomposition from MetaModel with simplified options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_quadratic_with_options<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let master_mechanism_model = master_meta_model
            .try_to_mechanism_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            )?;
        let sub_mechanism_model = sub_meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        let master_mechanism_model = ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &master_mechanism_model,
            options.value_conversion_policy,
        )?;
        let sub_mechanism_model = ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &sub_mechanism_model,
            options.value_conversion_policy,
        )?;
        let master_model = master_mechanism_model
            .try_into_quadratic_tetrad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            )?;
        let sub_model = sub_mechanism_model.try_into_quadratic_tetrad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        self.solve_quadratic_with_options(&master_model, &sub_model, options)
    }

    /// 使用 MetaModel 执行二次 Benders 分解并返回 typed 结果（同步）/
    /// Execute quadratic Benders decomposition from MetaModel and return typed result (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_quadratic_typed<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
    ) -> Result<FeasibleSolutionV<V>>
    where
        V: SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_quadratic_typed_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行二次 Benders 分解并返回 typed 结果（参数对象，同步）/
    /// Execute quadratic Benders decomposition from MetaModel and return typed result with options (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_quadratic_typed_with_options<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolutionV<V>>
    where
        V: SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        self.solve_meta_quadratic_with_options(master_meta_model, sub_meta_model, options)?
            .try_into_typed(policy)
    }

    /// 使用参数对象执行二次 Benders 分解（async）/
    /// Execute quadratic Benders decomposition with options object (async)
    #[cfg(feature = "async")]
    fn solve_quadratic_with_options<'a>(
        &'a self,
        master_model: &'a QuadraticTetradModel,
        sub_model: &'a QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    {
        Box::pin(async move {
            self.solve_quadratic_with_options_impl(master_model, sub_model, options)
                .await
        })
    }

    /// 使用统一选项执行二次 Benders 迭代 / Execute quadratic Benders iterations with unified options.
    #[cfg(feature = "async")]
    async fn solve_quadratic_with_options_impl(
        &self,
        master_model: &QuadraticTetradModel,
        sub_model: &QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve_quadratic_core_with_options(master_model, sub_model, options)
            .await
    }

    /// 使用参数对象执行二次 Benders 分解（同步）/
    /// Execute quadratic Benders decomposition with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_quadratic_with_options(
        &self,
        master_model: &QuadraticTetradModel,
        sub_model: &QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve_quadratic_with_options_impl(master_model, sub_model, options)
    }

    /// 使用统一选项执行二次 Benders 迭代 / Execute quadratic Benders iterations with unified options.
    #[cfg(not(feature = "async"))]
    fn solve_quadratic_with_options_impl(
        &self,
        master_model: &QuadraticTetradModel,
        sub_model: &QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve_quadratic_core_with_options(master_model, sub_model, options)
    }

    /// 执行二次 Benders 分解迭代 / Execute quadratic Benders decomposition iteration
    #[cfg(feature = "async")]
    async fn solve_quadratic(
        &self,
        master_model: &QuadraticTetradModel,
        sub_model: &QuadraticTetradModel,
        max_iterations: usize,
        tolerance: f64,
        max_stall_iterations: Option<usize>,
        objective_stall_iterations: Option<usize>,
    ) -> Result<FeasibleSolution> {
        let mut options =
            FrameworkSolveOptions::default().with_iterations(max_iterations, tolerance);
        options.max_stall_iterations = max_stall_iterations;
        options.objective_stall_iterations = objective_stall_iterations;
        self.solve_quadratic_core_with_options(master_model, sub_model, options)
            .await
    }

    /// 带统一选项的二次 Benders 核心循环 / Quadratic Benders core loop with unified options.
    #[cfg(feature = "async")]
    async fn solve_quadratic_core_with_options(
        &self,
        master_model: &QuadraticTetradModel,
        sub_model: &QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        let max_iterations = options.max_iterations;
        let tolerance = options.tolerance;
        let max_stall_iterations = options.max_stall_iterations;
        let objective_stall_iterations = options.objective_stall_iterations;
        let mut linear_cuts = Vec::new();
        let mut quadratic_cuts = Vec::new();
        let mut best_solution: Option<FeasibleSolution> = None;
        let mut best_solution_iterations: Option<usize> = None;
        let mut prev_obj = f64::NEG_INFINITY;
        let max_stall_iterations = max_stall_iterations.filter(|window| *window > 0);
        let objective_stall_iterations = objective_stall_iterations.unwrap_or(1).max(1);
        let mut no_cut_iterations = 0usize;
        let mut no_obj_improvement_iterations = 0usize;
        let mut executed_iterations = 0usize;
        let mut stop_reason: Option<BendersStopReason> = None;
        let mut iteration_snapshots: Vec<BendersIterationSnapshot> = Vec::new();
        let begin = Instant::now();

        for iter in 0..max_iterations {
            executed_iterations = iter + 1;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }
            emit_benders_progress(
                &options,
                "master",
                executed_iterations,
                max_iterations,
                0.0,
                begin.elapsed(),
                None,
                None,
                false,
            )?;
            let master_report = self
                .solve_master_quadratic_report_with_options(
                    master_model,
                    &linear_cuts,
                    &quadratic_cuts,
                    &options,
                )
                .await?;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }
            let master_solution = match master_solution_from_report(&master_report) {
                Ok(solution) => solution,
                Err(_error) if master_report.has_incumbent() => {
                    return partial_master_solution_from_report(
                        &master_report,
                        executed_iterations,
                        linear_cuts.len() + quadratic_cuts.len(),
                        no_cut_iterations,
                        no_obj_improvement_iterations,
                        iteration_snapshots,
                    );
                }
                Err(error) => return Err(error),
            };
            let master_obj = master_objective_from_report(&master_report)?;

            if (master_obj - prev_obj).abs() < tolerance {
                no_obj_improvement_iterations += 1;
            } else {
                no_obj_improvement_iterations = 0;
            }
            prev_obj = master_obj;

            let cuts_before = linear_cuts.len() + quadratic_cuts.len();
            let sub_result = self
                .solve_sub_quadratic_with_options(sub_model, &master_solution, &options)
                .await?;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }
            let sub_model_fingerprint = quadratic_model_fingerprint(sub_model)?;
            let subproblem_report;
            let mut subproblem_certificate = None;
            match sub_result {
                QuadraticSubResult::Feasible(feasible) => {
                    subproblem_report = feasible.linear.report.clone();
                    let (candidate, _) =
                        apply_master_bound(feasible.linear.result.clone(), &master_report);
                    best_solution = Some(candidate);
                    best_solution_iterations = Some(iter + 1);
                    if certified_linear_feasible_result_for_model(
                        &feasible.linear,
                        &sub_model_fingerprint,
                    ) {
                        subproblem_certificate = Some("subproblem-optimality");
                        if let Some(new_linear_cuts) = feasible.linear.cuts {
                            linear_cuts.extend(new_linear_cuts);
                        }
                        if let Some(new_quadratic_cuts) = feasible.quadratic_cuts {
                            quadratic_cuts.extend(new_quadratic_cuts);
                        }
                    } else {
                        stop_reason = Some(BendersStopReason::SubproblemTermination(
                            subproblem_termination_reason(feasible.linear.report.as_ref()),
                        ));
                    }
                }
                QuadraticSubResult::Infeasible(infeasible) => {
                    subproblem_report = infeasible.linear.report.clone();
                    if certified_linear_infeasible_result_for_model(
                        &infeasible.linear,
                        &sub_model_fingerprint,
                    ) {
                        subproblem_certificate = Some("subproblem-infeasibility");
                        if let Some(new_linear_cuts) = infeasible.linear.cuts {
                            linear_cuts.extend(new_linear_cuts);
                        }
                        if let Some(new_quadratic_cuts) = infeasible.quadratic_cuts {
                            quadratic_cuts.extend(new_quadratic_cuts);
                        }
                    } else {
                        return Err(CoreError::contract_error(
                            "quadratic Benders infeasible subproblem lacks a verified Farkas certificate",
                        ));
                    }
                }
            }
            let total_cuts_now = linear_cuts.len() + quadratic_cuts.len();
            let cuts_added = total_cuts_now.saturating_sub(cuts_before);

            if total_cuts_now > cuts_before {
                no_cut_iterations = 0;
            } else {
                no_cut_iterations += 1;
            }
            iteration_snapshots.push(BendersIterationSnapshot {
                iteration: iter + 1,
                master_obj,
                master_gap: master_report.statistics.relative_gap,
                master_bound: master_report.statistics.best_bound_value,
                cuts_added,
                total_cuts: total_cuts_now,
                no_cut_iterations,
                no_obj_improvement_iterations,
                master_attempt_id: format!("benders/iteration/{}/master", iter + 1),
                subproblem_attempt_id: Some(format!("benders/iteration/{}/subproblem", iter + 1)),
                master_report: Some(master_report.clone()),
                subproblem_report: subproblem_report.clone(),
                bound_valid: master_report.is_optimal()
                    && master_report.statistics.best_bound_value.is_some(),
                proof_reference: benders_proof_reference(
                    &master_report,
                    subproblem_report.as_ref(),
                    subproblem_certificate,
                ),
                stop_reason,
            });
            emit_benders_progress(
                &options,
                "subproblem",
                executed_iterations,
                max_iterations,
                100.0,
                begin.elapsed(),
                Some(master_obj),
                master_report.statistics.best_bound_value,
                false,
            )?;
            if matches!(
                stop_reason,
                Some(BendersStopReason::SubproblemTermination(_))
            ) {
                break;
            }
            if let Some(window) = max_stall_iterations
                && no_cut_iterations >= window
            {
                stop_reason = Some(BendersStopReason::CutStall);
                update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
                log::info!(
                    "Quadratic Benders decomposition stopped at iteration {} due to cut stagnation window {}",
                    iter + 1,
                    window
                );
                break;
            }

            if no_obj_improvement_iterations >= objective_stall_iterations {
                stop_reason = Some(BendersStopReason::ObjectiveStall);
                update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
                log::info!(
                    "Quadratic Benders decomposition stopped at iteration {} due to objective stall window {}",
                    iter + 1,
                    objective_stall_iterations
                );
                break;
            }
        }

        if stop_reason.is_none() && executed_iterations >= max_iterations && max_iterations > 0 {
            stop_reason = Some(BendersStopReason::IterationLimit);
            update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
        }

        let best_objective = best_solution.as_ref().map(|solution| solution.obj);
        let best_bound = best_solution
            .as_ref()
            .and_then(|solution| solution.possible_best_obj);
        emit_benders_progress(
            &options,
            "completed",
            executed_iterations,
            max_iterations,
            100.0,
            begin.elapsed(),
            best_objective,
            best_bound,
            true,
        )?;

        match best_solution {
            Some(solution) => {
                let runtime_metrics = build_benders_runtime_metrics(
                    executed_iterations,
                    best_solution_iterations,
                    linear_cuts.len() + quadratic_cuts.len(),
                    no_cut_iterations,
                    no_obj_improvement_iterations,
                    stop_reason,
                    iteration_snapshots,
                );
                Ok(match best_solution_iterations {
                    Some(iterations) => solution
                        .with_benders_iterations(iterations)
                        .with_benders_runtime_metrics(runtime_metrics),
                    None => solution.with_benders_runtime_metrics(runtime_metrics),
                })
            }
            None => Err(CoreError::Solver(SolverError::NoSolution)),
        }
    }

    /// 执行二次 Benders 分解迭代（同步）/ Execute quadratic Benders decomposition iteration (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_quadratic(
        &self,
        master_model: &QuadraticTetradModel,
        sub_model: &QuadraticTetradModel,
        max_iterations: usize,
        tolerance: f64,
        max_stall_iterations: Option<usize>,
        objective_stall_iterations: Option<usize>,
    ) -> Result<FeasibleSolution> {
        let mut options =
            FrameworkSolveOptions::default().with_iterations(max_iterations, tolerance);
        options.max_stall_iterations = max_stall_iterations;
        options.objective_stall_iterations = objective_stall_iterations;
        self.solve_quadratic_core_with_options(master_model, sub_model, options)
    }

    /// 带统一选项的二次 Benders 核心循环 / Quadratic Benders core loop with unified options.
    #[cfg(not(feature = "async"))]
    fn solve_quadratic_core_with_options(
        &self,
        master_model: &QuadraticTetradModel,
        sub_model: &QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        let max_iterations = options.max_iterations;
        let tolerance = options.tolerance;
        let max_stall_iterations = options.max_stall_iterations;
        let objective_stall_iterations = options.objective_stall_iterations;
        let mut linear_cuts = Vec::new();
        let mut quadratic_cuts = Vec::new();
        let mut best_solution: Option<FeasibleSolution> = None;
        let mut best_solution_iterations: Option<usize> = None;
        let mut prev_obj = f64::NEG_INFINITY;
        let max_stall_iterations = max_stall_iterations.filter(|window| *window > 0);
        let objective_stall_iterations = objective_stall_iterations.unwrap_or(1).max(1);
        let mut no_cut_iterations = 0usize;
        let mut no_obj_improvement_iterations = 0usize;
        let mut executed_iterations = 0usize;
        let mut stop_reason: Option<BendersStopReason> = None;
        let mut iteration_snapshots: Vec<BendersIterationSnapshot> = Vec::new();
        let begin = Instant::now();

        for iter in 0..max_iterations {
            executed_iterations = iter + 1;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }
            emit_benders_progress(
                &options,
                "master",
                executed_iterations,
                max_iterations,
                0.0,
                begin.elapsed(),
                None,
                None,
                false,
            )?;
            let master_report = self.solve_master_quadratic_report_with_options(
                master_model,
                &linear_cuts,
                &quadratic_cuts,
                &options,
            )?;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }
            let master_solution = match master_solution_from_report(&master_report) {
                Ok(solution) => solution,
                Err(_error) if master_report.has_incumbent() => {
                    return partial_master_solution_from_report(
                        &master_report,
                        executed_iterations,
                        linear_cuts.len() + quadratic_cuts.len(),
                        no_cut_iterations,
                        no_obj_improvement_iterations,
                        iteration_snapshots,
                    );
                }
                Err(error) => return Err(error),
            };
            let master_obj = master_objective_from_report(&master_report)?;

            if (master_obj - prev_obj).abs() < tolerance {
                no_obj_improvement_iterations += 1;
            } else {
                no_obj_improvement_iterations = 0;
            }
            prev_obj = master_obj;

            let cuts_before = linear_cuts.len() + quadratic_cuts.len();
            let sub_result =
                self.solve_sub_quadratic_with_options(sub_model, &master_solution, &options)?;
            if benders_cancelled(&options) {
                return Err(benders_cancel_error());
            }
            let sub_model_fingerprint = quadratic_model_fingerprint(sub_model)?;
            let subproblem_report;
            let mut subproblem_certificate = None;
            match sub_result {
                QuadraticSubResult::Feasible(feasible) => {
                    subproblem_report = feasible.linear.report.clone();
                    let (candidate, _) =
                        apply_master_bound(feasible.linear.result.clone(), &master_report);
                    best_solution = Some(candidate);
                    best_solution_iterations = Some(iter + 1);
                    if certified_linear_feasible_result_for_model(
                        &feasible.linear,
                        &sub_model_fingerprint,
                    ) {
                        subproblem_certificate = Some("subproblem-optimality");
                        if let Some(new_linear_cuts) = feasible.linear.cuts {
                            linear_cuts.extend(new_linear_cuts);
                        }
                        if let Some(new_quadratic_cuts) = feasible.quadratic_cuts {
                            quadratic_cuts.extend(new_quadratic_cuts);
                        }
                    } else {
                        stop_reason = Some(BendersStopReason::SubproblemTermination(
                            subproblem_termination_reason(feasible.linear.report.as_ref()),
                        ));
                    }
                }
                QuadraticSubResult::Infeasible(infeasible) => {
                    subproblem_report = infeasible.linear.report.clone();
                    if certified_linear_infeasible_result_for_model(
                        &infeasible.linear,
                        &sub_model_fingerprint,
                    ) {
                        subproblem_certificate = Some("subproblem-infeasibility");
                        if let Some(new_linear_cuts) = infeasible.linear.cuts {
                            linear_cuts.extend(new_linear_cuts);
                        }
                        if let Some(new_quadratic_cuts) = infeasible.quadratic_cuts {
                            quadratic_cuts.extend(new_quadratic_cuts);
                        }
                    } else {
                        return Err(CoreError::contract_error(
                            "quadratic Benders infeasible subproblem lacks a verified Farkas certificate",
                        ));
                    }
                }
            }
            let total_cuts_now = linear_cuts.len() + quadratic_cuts.len();
            let cuts_added = total_cuts_now.saturating_sub(cuts_before);

            if total_cuts_now > cuts_before {
                no_cut_iterations = 0;
            } else {
                no_cut_iterations += 1;
            }
            iteration_snapshots.push(BendersIterationSnapshot {
                iteration: iter + 1,
                master_obj,
                master_gap: master_report.statistics.relative_gap,
                master_bound: master_report.statistics.best_bound_value,
                cuts_added,
                total_cuts: total_cuts_now,
                no_cut_iterations,
                no_obj_improvement_iterations,
                master_attempt_id: format!("benders/iteration/{}/master", iter + 1),
                subproblem_attempt_id: Some(format!("benders/iteration/{}/subproblem", iter + 1)),
                master_report: Some(master_report.clone()),
                subproblem_report: subproblem_report.clone(),
                bound_valid: master_report.is_optimal()
                    && master_report.statistics.best_bound_value.is_some(),
                proof_reference: benders_proof_reference(
                    &master_report,
                    subproblem_report.as_ref(),
                    subproblem_certificate,
                ),
                stop_reason,
            });
            emit_benders_progress(
                &options,
                "subproblem",
                executed_iterations,
                max_iterations,
                100.0,
                begin.elapsed(),
                Some(master_obj),
                master_report.statistics.best_bound_value,
                false,
            )?;
            if matches!(
                stop_reason,
                Some(BendersStopReason::SubproblemTermination(_))
            ) {
                break;
            }
            if let Some(window) = max_stall_iterations
                && no_cut_iterations >= window
            {
                stop_reason = Some(BendersStopReason::CutStall);
                update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
                log::info!(
                    "Quadratic Benders decomposition stopped at iteration {} due to cut stagnation window {}",
                    iter + 1,
                    window
                );
                break;
            }

            if no_obj_improvement_iterations >= objective_stall_iterations {
                stop_reason = Some(BendersStopReason::ObjectiveStall);
                update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
                log::info!(
                    "Quadratic Benders decomposition stopped at iteration {} due to objective stall window {}",
                    iter + 1,
                    objective_stall_iterations
                );
                break;
            }
        }

        if stop_reason.is_none() && executed_iterations >= max_iterations && max_iterations > 0 {
            stop_reason = Some(BendersStopReason::IterationLimit);
            update_last_benders_snapshot_stop_reason(&mut iteration_snapshots, stop_reason);
        }

        let best_objective = best_solution.as_ref().map(|solution| solution.obj);
        let best_bound = best_solution
            .as_ref()
            .and_then(|solution| solution.possible_best_obj);
        emit_benders_progress(
            &options,
            "completed",
            executed_iterations,
            max_iterations,
            100.0,
            begin.elapsed(),
            best_objective,
            best_bound,
            true,
        )?;

        match best_solution {
            Some(solution) => {
                let runtime_metrics = build_benders_runtime_metrics(
                    executed_iterations,
                    best_solution_iterations,
                    linear_cuts.len() + quadratic_cuts.len(),
                    no_cut_iterations,
                    no_obj_improvement_iterations,
                    stop_reason,
                    iteration_snapshots,
                );
                Ok(match best_solution_iterations {
                    Some(iterations) => solution
                        .with_benders_iterations(iterations)
                        .with_benders_runtime_metrics(runtime_metrics),
                    None => solution.with_benders_runtime_metrics(runtime_metrics),
                })
            }
            None => Err(CoreError::Solver(SolverError::NoSolution)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::error::SolverErrorClass;
    use ospf_rust_core::model::intermediate::{
        BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
    };
    use ospf_rust_core::model::{ConstraintRelation, MetaModel};
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::{ModelBuildingStage, ModelBuildingStatusCallback};
    use ospf_rust_core::solver::{
        CancellationOrigin, ProblemStatus, SolveFingerprints, SolveHandle, SolveProgressReporter,
        SolveProgressSnapshot, SolveProof, SolveReport, SolveSolution, SolveStage, SolverStatus,
        TerminationReason,
    };
    use ospf_rust_core::variable::ContinuousVariableItem;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    fn verified_master_report(objective: f64, values: Vec<f64>) -> SolveReport<f64> {
        let mut solution = SolveSolution::vector(values);
        solution.objective = Some(objective);
        solution.objective_value = Some(objective);
        SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(solution)
            .proof(SolveProof::optimality())
            .build()
            .expect("test master report should satisfy the report contract")
    }

    fn verified_linear_subproblem_report(
        model: &LinearTriadModel,
        objective: f64,
        values: Vec<f64>,
    ) -> SolveReport<f64> {
        let mut solution = SolveSolution::vector(values);
        solution.objective = Some(objective);
        solution.objective_value = Some(objective);
        solution.dual_solution = Some(vec![0.0]);
        SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(solution)
            .proof(SolveProof::optimality())
            .fingerprints(SolveFingerprints {
                model: Some(linear_model_fingerprint(model).expect("linear model fingerprint")),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("test linear subproblem report should satisfy the report contract")
    }

    fn verified_quadratic_subproblem_report(
        model: &QuadraticTetradModel,
        objective: f64,
        values: Vec<f64>,
    ) -> SolveReport<f64> {
        let mut solution = SolveSolution::vector(values);
        solution.objective = Some(objective);
        solution.objective_value = Some(objective);
        solution.dual_solution = Some(vec![0.0]);
        SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(solution)
            .proof(SolveProof::optimality())
            .fingerprints(SolveFingerprints {
                model: Some(
                    quadratic_model_fingerprint(model).expect("quadratic model fingerprint"),
                ),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("test quadratic subproblem report should satisfy the report contract")
    }

    struct NonOptimalMasterBendersSolver {
        sub_calls: AtomicUsize,
    }

    impl NonOptimalMasterBendersSolver {
        fn new() -> Self {
            Self {
                sub_calls: AtomicUsize::new(0),
            }
        }
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl LinearBendersDecompositionSolver for NonOptimalMasterBendersSolver {
        fn name(&self) -> &str {
            "non_optimal_master_benders"
        }

        #[cfg(feature = "async")]
        async fn solve_master(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolverOutput> {
            Ok(SolverOutput::new(SolverStatus::TimeLimit)
                .with_objective(1.0)
                .with_solution(vec![1.0]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_master(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolverOutput> {
            Ok(SolverOutput::new(SolverStatus::TimeLimit)
                .with_objective(1.0)
                .with_solution(vec![1.0]))
        }

        #[cfg(feature = "async")]
        async fn solve_sub(
            &self,
            _model: &LinearTriadModel,
            _master_solution: &[f64],
        ) -> Result<LinearSubResult> {
            self.sub_calls.fetch_add(1, Ordering::SeqCst);
            Ok(LinearSubResult::Feasible(
                LinearFeasibleResult::new(
                    FeasibleSolution::new(1.0, vec![1.0]),
                    LinearDualSolution::new(vec![0.0], Vec::new()),
                )
                .with_report(verified_linear_subproblem_report(
                    _model,
                    1.0,
                    vec![1.0],
                )),
            ))
        }

        #[cfg(not(feature = "async"))]
        fn solve_sub(
            &self,
            _model: &LinearTriadModel,
            _master_solution: &[f64],
        ) -> Result<LinearSubResult> {
            self.sub_calls.fetch_add(1, Ordering::SeqCst);
            Ok(LinearSubResult::Feasible(
                LinearFeasibleResult::new(
                    FeasibleSolution::new(1.0, vec![1.0]),
                    LinearDualSolution::new(vec![0.0], Vec::new()),
                )
                .with_report(verified_linear_subproblem_report(
                    _model,
                    1.0,
                    vec![1.0],
                )),
            ))
        }
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl QuadraticBendersDecompositionSolver for NonOptimalMasterBendersSolver {
        #[cfg(feature = "async")]
        async fn solve_master_quadratic(
            &self,
            _model: &QuadraticTetradModel,
            _linear_cuts: &[LinearCut],
            _quadratic_cuts: &[QuadraticCut],
        ) -> Result<SolverOutput> {
            Ok(SolverOutput::new(SolverStatus::TimeLimit)
                .with_objective(1.0)
                .with_solution(vec![1.0]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_master_quadratic(
            &self,
            _model: &QuadraticTetradModel,
            _linear_cuts: &[LinearCut],
            _quadratic_cuts: &[QuadraticCut],
        ) -> Result<SolverOutput> {
            Ok(SolverOutput::new(SolverStatus::TimeLimit)
                .with_objective(1.0)
                .with_solution(vec![1.0]))
        }

        #[cfg(feature = "async")]
        async fn solve_master_quadratic_report(
            &self,
            _model: &QuadraticTetradModel,
            _linear_cuts: &[LinearCut],
            _quadratic_cuts: &[QuadraticCut],
        ) -> Result<SolveReport<f64>> {
            let mut solution = SolveSolution::vector(vec![1.0]);
            solution.objective = Some(1.0);
            solution.objective_value = Some(1.0);
            SolveReport::builder(ProblemStatus::Feasible, TerminationReason::TimeLimit)
                .solution(solution)
                .build()
        }

        #[cfg(not(feature = "async"))]
        fn solve_master_quadratic_report(
            &self,
            _model: &QuadraticTetradModel,
            _linear_cuts: &[LinearCut],
            _quadratic_cuts: &[QuadraticCut],
        ) -> Result<SolveReport<f64>> {
            let mut solution = SolveSolution::vector(vec![1.0]);
            solution.objective = Some(1.0);
            solution.objective_value = Some(1.0);
            SolveReport::builder(ProblemStatus::Feasible, TerminationReason::TimeLimit)
                .solution(solution)
                .build()
        }

        #[cfg(feature = "async")]
        async fn solve_sub_quadratic(
            &self,
            _model: &QuadraticTetradModel,
            _master_solution: &[f64],
        ) -> Result<QuadraticSubResult> {
            self.sub_calls.fetch_add(1, Ordering::SeqCst);
            Ok(QuadraticSubResult::Feasible(QuadraticFeasibleResult::new(
                LinearFeasibleResult::new(
                    FeasibleSolution::new(1.0, vec![1.0]),
                    LinearDualSolution::new(vec![0.0], Vec::new()),
                )
                .with_report(verified_quadratic_subproblem_report(
                    _model,
                    1.0,
                    vec![1.0],
                )),
            )))
        }

        #[cfg(not(feature = "async"))]
        fn solve_sub_quadratic(
            &self,
            _model: &QuadraticTetradModel,
            _master_solution: &[f64],
        ) -> Result<QuadraticSubResult> {
            self.sub_calls.fetch_add(1, Ordering::SeqCst);
            Ok(QuadraticSubResult::Feasible(QuadraticFeasibleResult::new(
                LinearFeasibleResult::new(
                    FeasibleSolution::new(1.0, vec![1.0]),
                    LinearDualSolution::new(vec![0.0], Vec::new()),
                )
                .with_report(verified_quadratic_subproblem_report(
                    _model,
                    1.0,
                    vec![1.0],
                )),
            )))
        }
    }

    struct UnverifiedInfeasibleSubproblemBendersSolver {
        sub_calls: AtomicUsize,
    }

    impl UnverifiedInfeasibleSubproblemBendersSolver {
        fn new() -> Self {
            Self {
                sub_calls: AtomicUsize::new(0),
            }
        }
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl LinearBendersDecompositionSolver for UnverifiedInfeasibleSubproblemBendersSolver {
        fn name(&self) -> &str {
            "unverified_infeasible_subproblem_benders"
        }

        #[cfg(feature = "async")]
        async fn solve_master(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![1.0]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_master(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![1.0]))
        }

        #[cfg(feature = "async")]
        async fn solve_master_report(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolveReport<f64>> {
            Ok(verified_master_report(1.0, vec![1.0]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_master_report(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolveReport<f64>> {
            Ok(verified_master_report(1.0, vec![1.0]))
        }

        #[cfg(feature = "async")]
        async fn solve_sub(
            &self,
            _model: &LinearTriadModel,
            _master_solution: &[f64],
        ) -> Result<LinearSubResult> {
            self.sub_calls.fetch_add(1, Ordering::SeqCst);
            Ok(LinearSubResult::Infeasible(
                LinearInfeasibleResult::new(LinearDualSolution::new(vec![1.0], Vec::new()))
                    .with_cuts(vec![LinearCut::ge("unverified-cut", vec![(0, 1.0)], 0.0)]),
            ))
        }

        #[cfg(not(feature = "async"))]
        fn solve_sub(
            &self,
            _model: &LinearTriadModel,
            _master_solution: &[f64],
        ) -> Result<LinearSubResult> {
            self.sub_calls.fetch_add(1, Ordering::SeqCst);
            Ok(LinearSubResult::Infeasible(
                LinearInfeasibleResult::new(LinearDualSolution::new(vec![1.0], Vec::new()))
                    .with_cuts(vec![LinearCut::ge("unverified-cut", vec![(0, 1.0)], 0.0)]),
            ))
        }
    }

    #[test]
    fn test_linear_cut() {
        let cut = LinearCut::le("test_cut", vec![(0, 1.0), (1, 2.0)], 10.0);
        assert_eq!(cut.name, "test_cut");
        assert_eq!(cut.rhs, 10.0);
        assert_eq!(cut.sense, CutSense::LessOrEqual);
    }

    #[test]
    fn test_quadratic_cut() {
        let cut = QuadraticCut::new(
            "quad_cut",
            vec![(0, 1.0)],
            vec![((0, 1), 2.0)],
            5.0,
            CutSense::LessOrEqual,
        );
        assert_eq!(cut.linear.name, "quad_cut");
        assert_eq!(cut.quadratic_coefficients.len(), 1);
    }

    #[test]
    fn subproblem_cuts_require_verified_reports() {
        let legacy_feasible = LinearFeasibleResult::new(
            FeasibleSolution::new(1.0, vec![1.0]),
            LinearDualSolution::new(vec![0.0], Vec::new()),
        )
        .with_cuts(vec![LinearCut::ge("legacy_cut", vec![(0, 1.0)], 0.0)]);
        assert!(!certified_linear_feasible_result(&legacy_feasible));

        let mut solution = SolveSolution::vector(vec![1.0]);
        solution.objective = Some(1.0);
        solution.objective_value = Some(1.0);
        solution.dual_solution = Some(vec![0.0]);
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(solution)
            .proof(SolveProof::optimality())
            .build()
            .expect("verified feasible subproblem report should be valid");
        let certified_feasible = legacy_feasible.with_report(report);
        assert!(certified_linear_feasible_result(&certified_feasible));

        let legacy_infeasible =
            LinearInfeasibleResult::new(LinearDualSolution::new(vec![1.0], Vec::new())).with_cuts(
                vec![LinearCut::ge("legacy_farkas_cut", vec![(0, 1.0)], 0.0)],
            );
        assert!(!certified_linear_infeasible_result(&legacy_infeasible));

        let mut proof = SolveProof::<f64>::infeasibility();
        proof.evidence = Some(vec![1.0]);
        let report = SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
            .proof(proof)
            .build()
            .expect("verified infeasible subproblem report should be valid");
        let certified_infeasible = legacy_infeasible.with_report(report);
        assert!(certified_linear_infeasible_result(&certified_infeasible));
    }

    #[test]
    fn subproblem_certificate_must_match_the_current_model_fingerprint() {
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("current-sub"));
        let expected_model = linear_model_fingerprint(&model).expect("model fingerprint");
        let mut wrong_model = expected_model.clone();
        wrong_model.value.push('x');

        let mut solution = SolveSolution::vector(vec![1.0]);
        solution.objective = Some(1.0);
        solution.objective_value = Some(1.0);
        solution.dual_solution = Some(vec![0.0]);
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(solution)
            .proof(SolveProof::optimality())
            .fingerprints(SolveFingerprints {
                model: Some(wrong_model),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("mismatched report should still be structurally valid");
        let result = LinearFeasibleResult::new(
            FeasibleSolution::new(1.0, vec![1.0]),
            LinearDualSolution::new(vec![0.0], Vec::new()),
        )
        .with_report(report)
        .with_cuts(vec![LinearCut::ge("must-not-apply", vec![(0, 1.0)], 0.0)]);

        assert!(!certified_linear_feasible_result_for_model(
            &result,
            &expected_model
        ));
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn unverified_infeasible_subproblem_is_a_contract_error_async() {
        let solver = UnverifiedInfeasibleSubproblemBendersSolver::new();
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new(
            "unverified_infeasible_subproblem_async",
        ));

        let error = solver
            .solve(&model, &model, 1, 1e-9, None, None)
            .await
            .expect_err("an unverified infeasible subproblem must stop Benders");

        assert_eq!(
            error.solver_error_class(),
            SolverErrorClass::InternalContract
        );
        assert_eq!(solver.sub_calls.load(Ordering::SeqCst), 1);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn unverified_infeasible_subproblem_is_a_contract_error() {
        let solver = UnverifiedInfeasibleSubproblemBendersSolver::new();
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new(
            "unverified_infeasible_subproblem",
        ));

        let error = solver
            .solve(&model, &model, 1, 1e-9, None, None)
            .expect_err("an unverified infeasible subproblem must stop Benders");

        assert_eq!(
            error.solver_error_class(),
            SolverErrorClass::InternalContract
        );
        assert_eq!(solver.sub_calls.load(Ordering::SeqCst), 1);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_non_optimal_master_preserves_incumbent_before_async_subproblem() {
        let solver = NonOptimalMasterBendersSolver::new();
        let model =
            LinearTriadModel::from_basic(BasicLinearTriadModel::new("non_optimal_master_async"));

        let result = solver.solve(&model, &model, 1, 1e-9, None, None).await;

        let result = result.expect("a master incumbent should be preserved");
        assert_eq!(result.obj, 1.0);
        assert_eq!(solver.sub_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_non_optimal_master_report_keeps_native_termination_async() {
        let solver = NonOptimalMasterBendersSolver::new();
        let model = build_meta_model("non_optimal_master_report_async");
        let snapshots = Arc::new(Mutex::new(Vec::<SolveProgressSnapshot>::new()));
        let snapshots_for_reporter = Arc::clone(&snapshots);
        let reporter: SolveProgressReporter = Arc::new(move |snapshot| {
            snapshots_for_reporter
                .lock()
                .expect("progress snapshot mutex should not be poisoned")
                .push(snapshot.clone());
            Ok(())
        });

        let report = solver
            .solve_report_with_options(
                &model,
                &model,
                FrameworkSolveOptions::default().with_progress_reporter(Some(reporter)),
            )
            .await
            .expect("a master incumbent should be projected to a report");

        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(report.termination_reason, TerminationReason::TimeLimit);
        assert!(!report.is_optimal());
        assert_eq!(solver.sub_calls.load(Ordering::SeqCst), 0);
        assert!(
            snapshots
                .lock()
                .expect("progress snapshots")
                .iter()
                .any(|snapshot| {
                    snapshot.stage == SolveStage::Combinatorial
                        && snapshot.stage_path
                            == vec![
                                "benders".to_owned(),
                                "iteration/1".to_owned(),
                                "master".to_owned(),
                            ]
                })
        );
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_non_optimal_quadratic_master_preserves_incumbent_before_async_subproblem() {
        let solver = NonOptimalMasterBendersSolver::new();
        let model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new(
            "non_optimal_quadratic_master_async",
        ));

        let result = solver
            .solve_quadratic(&model, &model, 1, 1e-9, None, None)
            .await
            .expect("a quadratic master incumbent should be preserved");

        assert_eq!(result.obj, 1.0);
        assert_eq!(solver.sub_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_pre_cancelled_benders_report_does_not_start_master() {
        let solver = NonOptimalMasterBendersSolver::new();
        let model = build_meta_model("pre_cancelled_benders");
        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::User));

        let report = solver
            .solve_report_with_options(
                &model,
                &model,
                FrameworkSolveOptions::default().with_cancellation_handle(Some(handle)),
            )
            .await
            .expect("pre-cancelled Benders should return a report");

        assert_eq!(report.termination_reason, TerminationReason::Cancelled);
        assert_eq!(solver.sub_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_non_optimal_master_preserves_incumbent_before_subproblem() {
        let solver = NonOptimalMasterBendersSolver::new();
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("non_optimal_master"));

        let result = solver.solve(&model, &model, 1, 1e-9, None, None);

        let result = result.expect("a master incumbent should be preserved");
        assert_eq!(result.obj, 1.0);
        assert_eq!(solver.sub_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_non_optimal_master_report_keeps_native_termination() {
        let solver = NonOptimalMasterBendersSolver::new();
        let model = build_meta_model("non_optimal_master_report");
        let snapshots = Arc::new(Mutex::new(Vec::<SolveProgressSnapshot>::new()));
        let snapshots_for_reporter = Arc::clone(&snapshots);
        let reporter: SolveProgressReporter = Arc::new(move |snapshot| {
            snapshots_for_reporter
                .lock()
                .expect("progress snapshot mutex should not be poisoned")
                .push(snapshot.clone());
            Ok(())
        });

        let report = solver
            .solve_report_with_options(
                &model,
                &model,
                FrameworkSolveOptions::default().with_progress_reporter(Some(reporter)),
            )
            .expect("a master incumbent should be projected to a report");

        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(report.termination_reason, TerminationReason::TimeLimit);
        assert!(!report.is_optimal());
        assert_eq!(solver.sub_calls.load(Ordering::SeqCst), 0);
        assert!(
            snapshots
                .lock()
                .expect("progress snapshots")
                .iter()
                .any(|snapshot| {
                    snapshot.stage == SolveStage::Combinatorial
                        && snapshot.stage_path
                            == vec![
                                "benders".to_owned(),
                                "iteration/1".to_owned(),
                                "master".to_owned(),
                            ]
                })
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_non_optimal_quadratic_master_preserves_incumbent_before_subproblem() {
        let solver = NonOptimalMasterBendersSolver::new();
        let model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new(
            "non_optimal_quadratic_master",
        ));

        let result = solver
            .solve_quadratic(&model, &model, 1, 1e-9, None, None)
            .expect("a quadratic master incumbent should be preserved");

        assert_eq!(result.obj, 1.0);
        assert_eq!(solver.sub_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_pre_cancelled_benders_report_does_not_start_master() {
        let solver = NonOptimalMasterBendersSolver::new();
        let model = build_meta_model("pre_cancelled_benders");
        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::User));

        let report = solver
            .solve_report_with_options(
                &model,
                &model,
                FrameworkSolveOptions::default().with_cancellation_handle(Some(handle)),
            )
            .expect("pre-cancelled Benders should return a report");

        assert_eq!(report.termination_reason, TerminationReason::Cancelled);
        assert_eq!(solver.sub_calls.load(Ordering::SeqCst), 0);
    }

    struct MockQuadraticBendersSolver {
        call_index: AtomicUsize,
        verified_subproblem: bool,
    }

    impl MockQuadraticBendersSolver {
        fn new() -> Self {
            Self {
                call_index: AtomicUsize::new(0),
                verified_subproblem: true,
            }
        }

        fn new_unverified_subproblem() -> Self {
            Self {
                call_index: AtomicUsize::new(0),
                verified_subproblem: false,
            }
        }

        fn next_quadratic_master_objective(&self) -> f64 {
            let index = self.call_index.fetch_add(1, Ordering::SeqCst);
            match index {
                0 => 10.0,
                1 => 5.0,
                _ => 5.0,
            }
        }
    }

    fn build_meta_model(name: &str) -> MetaModel<f64> {
        let mut model = MetaModel::<f64>::new(name);
        let x = ContinuousVariableItem::auto(&format!("{}_x", name));
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                &format!("{}_c", name),
            )
            .unwrap();
        model
    }

    #[cfg(not(feature = "async"))]
    fn build_non_finite_meta_model(name: &str) -> MetaModel<f64> {
        let mut model = MetaModel::<f64>::new(name);
        let x = ContinuousVariableItem::auto(&format!("{}_x", name));
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, f64::NAN)],
                ConstraintRelation::LessEqual,
                1.0,
                &format!("{}_non_finite_c", name),
            )
            .unwrap();
        model
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl LinearBendersDecompositionSolver for MockQuadraticBendersSolver {
        fn name(&self) -> &str {
            "mock_quadratic_benders"
        }

        #[cfg(feature = "async")]
        async fn solve_master(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(0.0, vec![0.0]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_master(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(0.0, vec![0.0]))
        }

        #[cfg(feature = "async")]
        async fn solve_master_report(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolveReport<f64>> {
            Ok(verified_master_report(0.0, vec![0.0]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_master_report(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolveReport<f64>> {
            Ok(verified_master_report(0.0, vec![0.0]))
        }

        #[cfg(feature = "async")]
        async fn solve_sub(
            &self,
            model: &LinearTriadModel,
            _master_solution: &[f64],
        ) -> Result<LinearSubResult> {
            let result = LinearFeasibleResult::new(
                FeasibleSolution::new(0.0, vec![0.0]),
                LinearDualSolution::new(vec![0.0], Vec::new()),
            );
            let result = if self.verified_subproblem {
                result.with_report(verified_linear_subproblem_report(model, 0.0, vec![0.0]))
            } else {
                result
            };
            Ok(LinearSubResult::Feasible(result))
        }

        #[cfg(not(feature = "async"))]
        fn solve_sub(
            &self,
            model: &LinearTriadModel,
            _master_solution: &[f64],
        ) -> Result<LinearSubResult> {
            let result = LinearFeasibleResult::new(
                FeasibleSolution::new(0.0, vec![0.0]),
                LinearDualSolution::new(vec![0.0], Vec::new()),
            );
            let result = if self.verified_subproblem {
                result.with_report(verified_linear_subproblem_report(model, 0.0, vec![0.0]))
            } else {
                result
            };
            Ok(LinearSubResult::Feasible(result))
        }
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl QuadraticBendersDecompositionSolver for MockQuadraticBendersSolver {
        #[cfg(feature = "async")]
        async fn solve_master_quadratic(
            &self,
            _model: &QuadraticTetradModel,
            _linear_cuts: &[LinearCut],
            _quadratic_cuts: &[QuadraticCut],
        ) -> Result<SolverOutput> {
            let objective = self.next_quadratic_master_objective();
            Ok(SolverOutput::optimal(objective, vec![objective]))
        }

        #[cfg(feature = "async")]
        async fn solve_master_quadratic_report(
            &self,
            _model: &QuadraticTetradModel,
            _linear_cuts: &[LinearCut],
            _quadratic_cuts: &[QuadraticCut],
        ) -> Result<SolveReport<f64>> {
            let objective = self.next_quadratic_master_objective();
            Ok(verified_master_report(objective, vec![objective]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_master_quadratic(
            &self,
            _model: &QuadraticTetradModel,
            _linear_cuts: &[LinearCut],
            _quadratic_cuts: &[QuadraticCut],
        ) -> Result<SolverOutput> {
            let objective = self.next_quadratic_master_objective();
            Ok(SolverOutput::optimal(objective, vec![objective]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_master_quadratic_report(
            &self,
            _model: &QuadraticTetradModel,
            _linear_cuts: &[LinearCut],
            _quadratic_cuts: &[QuadraticCut],
        ) -> Result<SolveReport<f64>> {
            let objective = self.next_quadratic_master_objective();
            Ok(verified_master_report(objective, vec![objective]))
        }

        #[cfg(feature = "async")]
        async fn solve_sub_quadratic(
            &self,
            model: &QuadraticTetradModel,
            master_solution: &[f64],
        ) -> Result<QuadraticSubResult> {
            let objective = master_solution.first().copied().unwrap_or(0.0);
            let result = LinearFeasibleResult::new(
                FeasibleSolution::new(objective, master_solution.to_vec()),
                LinearDualSolution::new(vec![0.0], Vec::new()),
            );
            let result = if self.verified_subproblem {
                result.with_report(verified_quadratic_subproblem_report(
                    model,
                    objective,
                    master_solution.to_vec(),
                ))
            } else {
                result
            };
            Ok(QuadraticSubResult::Feasible(QuadraticFeasibleResult::new(
                result,
            )))
        }

        #[cfg(not(feature = "async"))]
        fn solve_sub_quadratic(
            &self,
            model: &QuadraticTetradModel,
            master_solution: &[f64],
        ) -> Result<QuadraticSubResult> {
            let objective = master_solution.first().copied().unwrap_or(0.0);
            let result = LinearFeasibleResult::new(
                FeasibleSolution::new(objective, master_solution.to_vec()),
                LinearDualSolution::new(vec![0.0], Vec::new()),
            );
            let result = if self.verified_subproblem {
                result.with_report(verified_quadratic_subproblem_report(
                    model,
                    objective,
                    master_solution.to_vec(),
                ))
            } else {
                result
            };
            Ok(QuadraticSubResult::Feasible(QuadraticFeasibleResult::new(
                result,
            )))
        }
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn unverified_feasible_subproblem_returns_incomplete_report_async() {
        let solver = MockQuadraticBendersSolver::new_unverified_subproblem();
        let model = build_meta_model("unverified_feasible_subproblem_async");

        let report = solver
            .solve_report_with_options(
                &model,
                &model,
                FrameworkSolveOptions::new().with_iterations(10, 1e-9),
            )
            .await
            .expect("an incumbent should be preserved when the subproblem proof is unavailable");

        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(report.termination_reason, TerminationReason::BackendFailure);
        assert!(!report.is_optimal());
        assert!(
            report
                .diagnostics
                .warnings
                .iter()
                .any(|warning| warning.code == "IncompleteBendersProof")
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn unverified_feasible_subproblem_returns_incomplete_report() {
        let solver = MockQuadraticBendersSolver::new_unverified_subproblem();
        let model = build_meta_model("unverified_feasible_subproblem");

        let report = solver
            .solve_report_with_options(
                &model,
                &model,
                FrameworkSolveOptions::new().with_iterations(10, 1e-9),
            )
            .expect("an incumbent should be preserved when the subproblem proof is unavailable");

        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(report.termination_reason, TerminationReason::BackendFailure);
        assert!(!report.is_optimal());
        assert!(
            report
                .diagnostics
                .warnings
                .iter()
                .any(|warning| warning.code == "IncompleteBendersProof")
        );
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_quadratic_benders_trait_default_solve_converges() {
        let solver = MockQuadraticBendersSolver::new();
        let master_model =
            QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("master"));
        let sub_model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("sub"));

        let result = solver
            .solve_quadratic(&master_model, &sub_model, 10, 1e-9, None, None)
            .await
            .expect("quadratic benders solve should converge");

        assert!((result.obj - 5.0).abs() <= 1e-9);
        assert_eq!(result.solution, vec![5.0]);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_quadratic_benders_trait_default_solve_converges() {
        let solver = MockQuadraticBendersSolver::new();
        let master_model =
            QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("master"));
        let sub_model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("sub"));

        let result = solver
            .solve_quadratic(&master_model, &sub_model, 10, 1e-9, None, None)
            .expect("quadratic benders solve should converge");

        assert!((result.obj - 5.0).abs() <= 1e-9);
        assert_eq!(result.solution, vec![5.0]);
        assert!(result.benders_iterations.is_some());
        assert!(result.benders_runtime_metrics.is_some());
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_meta_model_shortcut_builds_then_delegates() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta");
        let stages = Arc::new(Mutex::new(Vec::new()));
        let stages_for_callback = stages.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            stages_for_callback.lock().unwrap().push(status.stage);
            Ok(())
        });

        let options = FrameworkSolveOptions::new()
            .with_building_callback(Some(callback))
            .with_iterations(5, 1e-9);
        let result = solver
            .solve_meta_with_options(&master_meta_model, &sub_meta_model, options)
            .expect("linear benders meta-model shortcut should succeed");

        assert!((result.obj - 0.0).abs() <= 1e-9);
        let stages = stages.lock().unwrap();
        assert!(stages.contains(&ModelBuildingStage::RegisterTokens));
        assert!(stages.contains(&ModelBuildingStage::FlattenLinearModel));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_meta_model_with_options_builds_then_delegates() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta_options");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_options");
        let stages = Arc::new(Mutex::new(Vec::new()));
        let stages_for_callback = stages.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            stages_for_callback.lock().unwrap().push(status.stage);
            Ok(())
        });

        let options = FrameworkSolveOptions::new()
            .with_building_callback(Some(callback))
            .with_iterations(5, 1e-9);
        let result = solver
            .solve_meta_with_options(&master_meta_model, &sub_meta_model, options)
            .expect("linear benders meta-model options shortcut should succeed");

        assert!((result.obj - 0.0).abs() <= 1e-9);
        let stages = stages.lock().unwrap();
        assert!(stages.contains(&ModelBuildingStage::RegisterTokens));
        assert!(stages.contains(&ModelBuildingStage::FlattenLinearModel));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_sub_result_try_into_typed_works() {
        let raw = LinearSubResult::Feasible(LinearFeasibleResult::new(
            FeasibleSolution::new(2.0, vec![1.0, 3.0]),
            LinearDualSolution::new(vec![0.5], vec![-0.25]),
        ));
        let typed = raw
            .try_into_typed::<f64>(ospf_rust_core::solver::SolveValueConversionPolicy::Strict)
            .expect("typed linear sub result conversion should succeed");
        match typed {
            LinearSubResultV::Feasible(feasible) => {
                assert!((feasible.result.obj - 2.0).abs() <= 1e-9);
                assert_eq!(feasible.result.solution.len(), 2);
                assert_eq!(feasible.dual_solution.constraints.len(), 1);
            }
            LinearSubResultV::Infeasible(_) => panic!("expected feasible typed sub result"),
        }
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_solve_sub_typed_shortcut_works() {
        let solver = MockQuadraticBendersSolver::new();
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("linear_sub_typed"));
        let typed = solver
            .solve_sub_typed::<f64>(
                &model,
                &[0.0],
                ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
            )
            .expect("typed linear sub shortcut should succeed");
        match typed {
            LinearSubResultV::Feasible(feasible) => {
                assert!((feasible.result.obj - 0.0).abs() <= 1e-9);
            }
            LinearSubResultV::Infeasible(_) => panic!("expected feasible typed linear sub result"),
        }
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_objective_stall_window_stops_early() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_objective_stall_master");
        let sub_meta_model = build_meta_model("linear_benders_objective_stall_sub");
        let options = FrameworkSolveOptions::new()
            .with_iterations(10, 1e-9)
            .with_objective_stall_iterations(2);

        let result = solver
            .solve_meta_with_options(&master_meta_model, &sub_meta_model, options)
            .expect("linear benders objective stall should return best feasible solution");

        assert_eq!(result.benders_iterations, Some(3));
        let runtime_metrics = result
            .benders_runtime_metrics
            .as_ref()
            .expect("runtime metrics should be set");
        assert_eq!(runtime_metrics.executed_iterations, 3);
        assert_eq!(
            runtime_metrics.stop_reason,
            Some(BendersStopReason::ObjectiveStall)
        );
        assert_eq!(runtime_metrics.iteration_snapshots.len(), 3);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_cut_stall_window_stops_early() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_cut_stall_master");
        let sub_meta_model = build_meta_model("linear_benders_cut_stall_sub");
        let options = FrameworkSolveOptions::new()
            .with_iterations(10, 1e-9)
            .with_stall_iterations(2)
            .with_objective_stall_iterations(10);

        let result = solver
            .solve_meta_with_options(&master_meta_model, &sub_meta_model, options)
            .expect("linear benders cut stall should return best feasible solution");

        assert_eq!(result.benders_iterations, Some(2));
        let runtime_metrics = result
            .benders_runtime_metrics
            .as_ref()
            .expect("runtime metrics should be set");
        assert_eq!(runtime_metrics.executed_iterations, 2);
        assert_eq!(
            runtime_metrics.stop_reason,
            Some(BendersStopReason::CutStall)
        );
        assert_eq!(runtime_metrics.iteration_snapshots.len(), 2);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn benders_iteration_snapshot_retains_reports_and_bound_contract() {
        let solver = MockQuadraticBendersSolver::new();
        let model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new(
            "benders_snapshot_contract",
        ));

        let result = solver
            .solve_quadratic(&model, &model, 1, 1e-9, None, None)
            .expect("one verified Benders iteration should return an incumbent");
        let metrics = result
            .benders_runtime_metrics
            .as_ref()
            .expect("Benders metrics should retain the iteration snapshot");
        let snapshot = metrics
            .iteration_snapshots
            .first()
            .expect("one iteration snapshot should be recorded");

        assert_eq!(snapshot.master_attempt_id, "benders/iteration/1/master");
        assert_eq!(
            snapshot.subproblem_attempt_id.as_deref(),
            Some("benders/iteration/1/subproblem")
        );
        assert!(
            snapshot
                .master_report
                .as_ref()
                .is_some_and(|report| { report.is_optimal() && report.proof.is_some() })
        );
        assert!(
            snapshot
                .subproblem_report
                .as_ref()
                .is_some_and(|report| report.is_optimal() && report.proof.is_some())
        );
        assert_eq!(
            snapshot.proof_reference.as_deref(),
            Some("subproblem-optimality")
        );
        assert!(!snapshot.bound_valid);
        assert_eq!(
            snapshot.stop_reason,
            Some(BendersStopReason::IterationLimit)
        );
        assert!(result.possible_best_obj.is_none());
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn benders_iteration_snapshot_retains_reports_and_bound_contract_async() {
        let solver = MockQuadraticBendersSolver::new();
        let model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new(
            "benders_snapshot_contract_async",
        ));

        let result = solver
            .solve_quadratic(&model, &model, 1, 1e-9, None, None)
            .await
            .expect("one verified Benders iteration should return an incumbent");
        let metrics = result
            .benders_runtime_metrics
            .as_ref()
            .expect("Benders metrics should retain the iteration snapshot");
        let snapshot = metrics
            .iteration_snapshots
            .first()
            .expect("one iteration snapshot should be recorded");

        assert_eq!(snapshot.master_attempt_id, "benders/iteration/1/master");
        assert_eq!(
            snapshot.subproblem_attempt_id.as_deref(),
            Some("benders/iteration/1/subproblem")
        );
        assert!(
            snapshot
                .master_report
                .as_ref()
                .is_some_and(|report| { report.is_optimal() && report.proof.is_some() })
        );
        assert!(
            snapshot
                .subproblem_report
                .as_ref()
                .is_some_and(|report| report.is_optimal() && report.proof.is_some())
        );
        assert_eq!(
            snapshot.proof_reference.as_deref(),
            Some("subproblem-optimality")
        );
        assert!(!snapshot.bound_valid);
        assert_eq!(
            snapshot.stop_reason,
            Some(BendersStopReason::IterationLimit)
        );
        assert!(result.possible_best_obj.is_none());
    }

    #[cfg(feature = "async")]
    fn assert_send<T: Send>(_: &T) {}

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_linear_benders_meta_model_shortcut_returns_send_future() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta_async_send");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_async_send");

        let future = solver.solve_meta(&master_meta_model, &sub_meta_model);
        assert_send(&future);

        let result = future
            .await
            .expect("linear benders meta-model shortcut should succeed");
        assert!((result.obj - 0.0).abs() <= 1e-9);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_linear_benders_meta_model_with_options_returns_send_future() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta_async_send_options");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_async_send_options");
        let options = FrameworkSolveOptions::new().with_iterations(5, 1e-9);

        let future = solver.solve_meta_with_options(&master_meta_model, &sub_meta_model, options);
        assert_send(&future);

        let result = future
            .await
            .expect("linear benders meta-model options shortcut should succeed");
        assert!((result.obj - 0.0).abs() <= 1e-9);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_quadratic_benders_meta_model_shortcut_returns_send_future() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("quadratic_benders_master_meta");
        let sub_meta_model = build_meta_model("quadratic_benders_sub_meta");

        let future = solver.solve_meta_quadratic(&master_meta_model, &sub_meta_model);
        assert_send(&future);

        let result = future
            .await
            .expect("quadratic benders meta-model shortcut should succeed");
        assert!((result.obj - 5.0).abs() <= 1e-9);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_quadratic_benders_meta_model_with_options_returns_send_future() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("quadratic_benders_master_meta_options_send");
        let sub_meta_model = build_meta_model("quadratic_benders_sub_meta_options_send");
        let options = FrameworkSolveOptions::new().with_iterations(5, 1e-9);

        let future =
            solver.solve_meta_quadratic_with_options(&master_meta_model, &sub_meta_model, options);
        assert_send(&future);

        let result = future
            .await
            .expect("quadratic benders meta-model options shortcut should succeed");
        assert!((result.obj - 5.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_meta_model_simplified_alias_builds_then_delegates() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta_alias");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_alias");

        let result = solver
            .solve_meta(&master_meta_model, &sub_meta_model)
            .expect("linear benders simplified alias should succeed");
        assert!((result.obj - 0.0).abs() <= 1e-9);
        assert!(result.benders_iterations.is_some());
        assert!(result.benders_runtime_metrics.is_some());
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_meta_model_shortcut_rejects_non_finite_value() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model =
            build_non_finite_meta_model("linear_benders_master_meta_non_finite");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_non_finite");
        let options = FrameworkSolveOptions::new().with_value_conversion_policy(
            ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
        );

        let error = solver
            .solve_meta_with_options(&master_meta_model, &sub_meta_model, options)
            .expect_err(
                "strict mode should reject non-finite conversion in linear benders meta path",
            );
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::NonFinite(_))
        ));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_meta_typed_shortcut_builds_then_delegates() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta_typed");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_typed");

        let result = solver
            .solve_meta_typed_with_options(
                &master_meta_model,
                &sub_meta_model,
                FrameworkSolveOptions::new().with_iterations(5, 1e-9),
            )
            .expect("linear benders typed meta shortcut should succeed");
        assert!((result.obj - 0.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_quadratic_benders_meta_model_shortcut_rejects_non_finite_value() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model =
            build_non_finite_meta_model("quadratic_benders_master_meta_non_finite");
        let sub_meta_model = build_meta_model("quadratic_benders_sub_meta_non_finite");
        let options = FrameworkSolveOptions::new().with_value_conversion_policy(
            ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
        );

        let error = solver
            .solve_meta_quadratic_with_options(&master_meta_model, &sub_meta_model, options)
            .expect_err(
                "strict mode should reject non-finite conversion in quadratic benders meta path",
            );
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::NonFinite(_))
        ));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_quadratic_sub_result_try_into_typed_works() {
        let raw =
            QuadraticSubResult::Feasible(QuadraticFeasibleResult::new(LinearFeasibleResult::new(
                FeasibleSolution::new(4.0, vec![2.0]),
                LinearDualSolution::new(vec![1.0], vec![0.0]),
            )));
        let typed = raw
            .try_into_typed::<f64>(ospf_rust_core::solver::SolveValueConversionPolicy::Strict)
            .expect("typed quadratic sub result conversion should succeed");
        match typed {
            QuadraticSubResultV::Feasible(feasible) => {
                assert!((feasible.linear.result.obj - 4.0).abs() <= 1e-9);
                assert_eq!(feasible.linear.result.solution.len(), 1);
            }
            QuadraticSubResultV::Infeasible(_) => panic!("expected feasible typed sub result"),
        }
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_quadratic_benders_solve_sub_typed_shortcut_works() {
        let solver = MockQuadraticBendersSolver::new();
        let model =
            QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("quadratic_sub_typed"));
        let typed = solver
            .solve_sub_quadratic_typed::<f64>(
                &model,
                &[3.0],
                ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
            )
            .expect("typed quadratic sub shortcut should succeed");
        match typed {
            QuadraticSubResultV::Feasible(feasible) => {
                assert!((feasible.linear.result.obj - 3.0).abs() <= 1e-9);
            }
            QuadraticSubResultV::Infeasible(_) => {
                panic!("expected feasible typed quadratic sub result")
            }
        }
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_quadratic_benders_meta_typed_shortcut_builds_then_delegates() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("quadratic_benders_master_meta_typed");
        let sub_meta_model = build_meta_model("quadratic_benders_sub_meta_typed");

        let result = solver
            .solve_meta_quadratic_typed_with_options(
                &master_meta_model,
                &sub_meta_model,
                FrameworkSolveOptions::new().with_iterations(5, 1e-9),
            )
            .expect("quadratic benders typed meta shortcut should succeed");
        assert!((result.obj - 5.0).abs() <= 1e-9);
    }
}
