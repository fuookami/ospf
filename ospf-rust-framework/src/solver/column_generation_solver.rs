//! 列生成求解器定义
//! Column Generation Solver Definitions
//!
//! 本模块提供列生成求解器的 trait 定义。
//! This module provides trait definitions for column generation solvers.

use super::FrameworkSolveOptions;
use ospf_rust_core::error::{CoreError, Result, SolverError};
#[cfg(all(feature = "nightly", not(feature = "async")))]
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::{
    CombinatorialSolveReport, FeasibleSolverOutput, ProblemStatus, ProgressValue, SolveDiagnostics,
    SolveHandle, SolveIssue, SolveIterationSnapshot, SolveProgressSnapshot, SolveProof,
    SolveReport, SolveSolution, SolveStage, SolveStatistics, SolveTrace, SolveValue,
    SolveValueConversionPolicy, SolveWarning, SolverOutput, SolverProvenance, TerminationReason,
    cancelled_solve_report, convert_report_value,
};
use std::sync::Arc;
use std::time::Duration;

/// 求解状态回调 / Solving Status Callback
pub type SolvingStatusCallback = Arc<dyn Fn(&SolvingStatus) -> Result<()> + Send + Sync>;

/// 注册状态回调 / Registration Status Callback
pub type RegistrationStatusCallback = Arc<dyn Fn(&RegistrationStatus) -> Result<()> + Send + Sync>;

/// 发布组合算法进度 / Publish progress for a combinatorial algorithm.
///
/// 该 helper 统一 stage、层级路径和 reporter 错误传播，避免 Benders、列生成和
/// Branch-and-Price 各自定义终态或进度格式。
/// This helper centralizes stage paths and reporter error propagation so Benders,
/// column generation, and Branch-and-Price do not define parallel progress formats.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_combinatorial_progress(
    options: &FrameworkSolveOptions,
    attempt_id: impl Into<String>,
    stage: SolveStage,
    stage_path: Vec<String>,
    stage_progress: ProgressValue,
    overall_progress: ProgressValue,
    elapsed: Duration,
    objective_value: Option<f64>,
    best_bound: Option<f64>,
    relative_gap: Option<f64>,
    terminal: bool,
) -> Result<()> {
    let Some(reporter) = options.progress_reporter.as_ref() else {
        return Ok(());
    };
    let snapshot = SolveProgressSnapshot::new(
        attempt_id,
        stage,
        stage_path,
        stage_progress,
        overall_progress,
        elapsed,
        objective_value,
        best_bound,
        relative_gap,
        terminal,
    )?;
    reporter(&snapshot)
}

/// 发布列生成外层进度 / Publish progress for the column-generation outer loop.
///
/// 子 solver 仍可以发布自己的 backend stage；这里的固定路径标识 wrapper 的
/// attempt 和聚合完成点，避免把 backend 进度误当成算法层进度。
/// Child solvers may still publish backend stages; this fixed path identifies the
/// wrapper attempt and aggregate completion without conflating backend and algorithm progress.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_column_generation_progress(
    options: &FrameworkSolveOptions,
    attempt_index: Option<usize>,
    phase: &str,
    stage_progress: ProgressValue,
    overall_progress: ProgressValue,
    elapsed: Duration,
    objective_value: Option<f64>,
    best_bound: Option<f64>,
    relative_gap: Option<f64>,
    terminal: bool,
) -> Result<()> {
    let (stage, stage_path) = if terminal {
        (
            SolveStage::Completed,
            vec!["column-generation".to_owned(), "completed".to_owned()],
        )
    } else {
        let attempt_index = attempt_index.unwrap_or(0);
        (
            SolveStage::Combinatorial,
            vec![
                "column-generation".to_owned(),
                format!("attempt/{attempt_index}"),
                phase.to_owned(),
            ],
        )
    };
    emit_combinatorial_progress(
        options,
        options.name.as_deref().unwrap_or("column-generation"),
        stage,
        stage_path,
        stage_progress,
        overall_progress,
        elapsed,
        objective_value,
        best_bound,
        relative_gap,
        terminal,
    )
}

fn pre_cancelled_report(
    solver_name: &str,
    options: &FrameworkSolveOptions,
) -> Result<Option<CombinatorialSolveReport<f64>>> {
    let Some(handle) = options.cancellation_handle.as_ref() else {
        return Ok(None);
    };
    if !handle.is_cancelled() {
        return Ok(None);
    }
    let report = cancelled_solve_report(
        SolverProvenance {
            solver_id: solver_name.to_owned(),
            backend_name: "framework-combinatorial".to_owned(),
            ..SolverProvenance::default()
        },
        handle,
    )?;
    Ok(Some(CombinatorialSolveReport::single(
        report,
        format!("{}#0", solver_name),
    )))
}

/// 求解状态 / Solving Status
#[derive(Debug, Clone)]
pub struct SolvingStatus {
    /// 求解器名称 / Solver name
    pub solver: String,
    /// 求解器索引 / Solver index
    pub solver_index: usize,
    /// 当前目标值 / Current objective value
    pub obj: f64,
    /// 当前下界 / Current lower bound
    pub lower_bound: Option<f64>,
    /// 当前上界 / Current upper bound
    pub upper_bound: Option<f64>,
    /// 当前 Gap / Current gap
    pub gap: Option<f64>,
    /// 已用时间 / Elapsed time
    pub elapsed: Duration,
    /// 节点数（MIP）/ Node count (MIP)
    pub node_count: Option<usize>,
}

impl SolvingStatus {
    /// 创建新的求解状态 / Create new solving status
    pub fn new(solver: impl Into<String>, solver_index: usize, obj: f64) -> Self {
        Self {
            solver: solver.into(),
            solver_index,
            obj,
            lower_bound: None,
            upper_bound: None,
            gap: None,
            elapsed: Duration::ZERO,
            node_count: None,
        }
    }
}

/// 注册状态 / Registration Status
#[derive(Debug, Clone)]
pub struct RegistrationStatus {
    /// 求解器名称 / Solver name
    pub solver: String,
    /// 变量数量 / Variable count
    pub variable_count: usize,
    /// 约束数量 / Constraint count
    pub constraint_count: usize,
    /// 非零元素数量 / Non-zero element count
    pub non_zero_count: usize,
}

impl RegistrationStatus {
    /// 创建新的注册状态 / Create new registration status
    pub fn new(
        solver: impl Into<String>,
        variable_count: usize,
        constraint_count: usize,
        non_zero_count: usize,
    ) -> Self {
        Self {
            solver: solver.into(),
            variable_count,
            constraint_count,
            non_zero_count,
        }
    }
}

/// 可行解 / Feasible Solution
#[derive(Debug, Clone)]
pub struct FeasibleSolution {
    /// 目标值 / Objective value
    pub obj: f64,
    /// 解向量 / Solution vector
    pub solution: Vec<f64>,
    /// 求解时间 / Solve time
    pub time: Duration,
    /// 可能的最优目标值 / Possible best objective
    pub possible_best_obj: Option<f64>,
    /// Gap
    pub gap: f64,
    /// Benders 迭代次数 / Benders iteration count
    pub benders_iterations: Option<usize>,
    /// Benders 运行时指标 / Benders runtime metrics
    pub benders_runtime_metrics: Option<BendersRuntimeMetrics>,
    /// 产生该兼容解的原始统一报告 / Original unified report that produced this compatibility solution.
    ///
    /// 旧调用方仍可以只使用字段投影；report-first 入口使用该快照保留
    /// provenance、fingerprint、dual、诊断和 backend 统计。
    /// Legacy callers may continue using the field projection; report-first entries use this
    /// snapshot to retain provenance, fingerprints, duals, diagnostics, and backend statistics.
    pub source_report: Option<SolveReport<f64>>,
}

/// 泛型可行解（typed 输出）/ Generic feasible solution (typed output)
#[derive(Debug, Clone)]
pub struct FeasibleSolutionV<V>
where
    V: SolveValue,
{
    /// 目标值 / Objective value
    pub obj: V,
    /// 解向量 / Solution vector
    pub solution: Vec<V>,
    /// 求解时间 / Solve time
    pub time: Duration,
    /// 可能的最优目标值 / Possible best objective
    pub possible_best_obj: Option<V>,
    /// Gap
    pub gap: f64,
    /// Benders 迭代次数 / Benders iteration count
    pub benders_iterations: Option<usize>,
    /// Benders 运行时指标 / Benders runtime metrics
    pub benders_runtime_metrics: Option<BendersRuntimeMetrics>,
}

/// Benders 停止原因 / Benders stop reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BendersStopReason {
    /// 达到最大迭代上限 / Reached max iteration limit
    IterationLimit,
    /// 命中 cut 停滞窗口 / Hit cut stall window
    CutStall,
    /// 命中目标改进停滞窗口 / Hit objective-improvement stall window
    ObjectiveStall,
    /// master 有 incumbent 但未形成可继续迭代的最优证明 / Master has an incumbent but no proof allowing another iteration
    MasterTermination(ospf_rust_core::solver::TerminationReason),
    /// 子问题有 incumbent 但未形成可生成 cut 的可靠证明 / Subproblem has an incumbent but no reliable proof for generating a cut
    SubproblemTermination(ospf_rust_core::solver::TerminationReason),
}

/// Benders 运行时指标 / Benders runtime metrics
#[derive(Debug, Clone)]
pub struct BendersRuntimeMetrics {
    /// 实际执行迭代轮数 / Executed iteration count
    pub executed_iterations: usize,
    /// 最优可行解出现轮次 / Iteration index where best feasible solution appeared
    pub best_solution_iteration: Option<usize>,
    /// 累计新增切割数 / Total newly added cuts
    pub total_cuts: usize,
    /// 结束时连续无新 cut 轮数 / Consecutive no-cut iterations at stop
    pub no_cut_iterations: usize,
    /// 结束时连续目标改进不足轮数 / Consecutive objective-stall iterations at stop
    pub no_obj_improvement_iterations: usize,
    /// 触发停止原因 / Stop reason
    pub stop_reason: Option<BendersStopReason>,
    /// 逐轮快照 / Per-iteration snapshots
    pub iteration_snapshots: Vec<BendersIterationSnapshot>,
}

/// Benders 逐轮快照 / Benders per-iteration snapshot
#[derive(Debug, Clone, Default)]
pub struct BendersIterationSnapshot {
    /// 当前轮次（从 1 开始）/ Iteration index (1-based)
    pub iteration: usize,
    /// 当前 master 目标值 / Current master objective value
    pub master_obj: f64,
    /// 当前 master gap（若可用）/ Current master gap (if available)
    pub master_gap: Option<f64>,
    /// 当前 master 有效下界（若可用）/ Current master best bound (if available)
    pub master_bound: Option<f64>,
    /// 本轮新增 cut 数 / Newly added cuts in this iteration
    pub cuts_added: usize,
    /// 截止本轮累计 cut 数 / Total cuts up to this iteration
    pub total_cuts: usize,
    /// 本轮结束时连续无新 cut 轮数 / Consecutive no-cut iterations at end of this iteration
    pub no_cut_iterations: usize,
    /// 本轮结束时连续目标停滞轮数 / Consecutive objective-stall iterations at end of this iteration
    pub no_obj_improvement_iterations: usize,
    /// 本轮 master attempt 身份 / Master-attempt identity for this iteration.
    pub master_attempt_id: String,
    /// 本轮 subproblem attempt 身份 / Subproblem-attempt identity for this iteration.
    pub subproblem_attempt_id: Option<String>,
    /// 本轮 master 原始报告 / Original master report for this iteration.
    pub master_report: Option<SolveReport<f64>>,
    /// 本轮 subproblem 原始报告 / Original subproblem report for this iteration.
    pub subproblem_report: Option<SolveReport<f64>>,
    /// 本轮 bound 是否可供外层算法使用 / Whether this iteration exposes a valid outer bound.
    pub bound_valid: bool,
    /// 本轮证书引用 / Certificate reference for this iteration.
    pub proof_reference: Option<String>,
    /// 本轮确定的停止原因 / Stop reason determined for this iteration.
    pub stop_reason: Option<BendersStopReason>,
}

/// 组合报告的选择策略 / Selection policy for a combinatorial report.
#[derive(Debug, Clone, Copy)]
pub(crate) enum CombinatorialSelection {
    /// 按 solver 列表顺序选择首个可用报告 / Select the first usable report by solver order.
    First,
    /// 按并行 wrapper 首次接受的 worker 选择报告 / Select the worker accepted first by a parallel wrapper.
    FirstCompleted(usize),
    /// 按目标方向选择最优报告，并以 solver 顺序打破平局 / Select the best report and break ties by solver order.
    Best(ObjectiveCategory),
}

/// 串行组合求解的 fallback 策略 / Fallback policy for serial combinatorial solving.
///
/// 取消、外部中断和超时始终终止当前 fallback 链；该策略只控制已形成数学终态
/// （例如 verified infeasible 或 unbounded）时是否继续尝试后续 backend。
/// Cancellation, interruption, and timeout always terminate the current fallback chain;
/// this policy only controls whether a mathematical terminal conclusion such as verified
/// infeasibility or unboundedness allows later backends to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CombinatorialFallbackPolicy {
    /// 遇到可靠终态立即停止 / Stop at a reliable terminal conclusion.
    #[default]
    StopOnCertifiedTerminal,
    /// 遇到可靠终态仍尝试后续 backend / Continue after a reliable terminal conclusion.
    ContinueOnTerminal,
}

impl CombinatorialFallbackPolicy {
    /// 判断错误是否应停止 fallback / Decide whether an error stops fallback.
    pub(crate) fn should_stop_on_error(&self, error: &CoreError) -> bool {
        match error {
            CoreError::Solver(
                SolverError::Cancelled(_) | SolverError::Interrupted(_) | SolverError::Timeout(_),
            ) => true,
            CoreError::Solver(SolverError::Infeasible | SolverError::Unbounded) => {
                matches!(self, Self::StopOnCertifiedTerminal)
            }
            _ => false,
        }
    }

    /// 判断报告是否应停止 fallback / Decide whether a report stops fallback.
    pub(crate) fn should_stop_on_report(&self, report: &SolveReport<f64>) -> bool {
        if report_is_cancellation_terminal(report) || report.has_incumbent() {
            return true;
        }
        matches!(self, Self::StopOnCertifiedTerminal) && report_is_selectable(report)
    }
}

pub(crate) fn report_is_selectable(report: &SolveReport<f64>) -> bool {
    if report.has_incumbent() {
        return true;
    }

    // 只有带 verified certificate 的终态才能阻止 fallback；兼容 facade 的裸
    // `Infeasible`/`Unbounded` 状态不能被当作数学结论消费。
    // Only terminal states with a verified certificate may stop fallback; a bare
    // `Infeasible`/`Unbounded` status from the compatibility facade is not consumable.
    match report.problem_status {
        ProblemStatus::Infeasible => {
            ospf_rust_core::solver::require_infeasibility_certificate(report).is_ok()
        }
        ProblemStatus::Unbounded | ProblemStatus::InfeasibleOrUnbounded => {
            report.proof.as_ref().is_some_and(|proof| {
                proof.status == ospf_rust_core::solver::ProofStatus::Verified
                    && matches!(
                        proof.reliability,
                        ospf_rust_core::solver::ProofReliability::Exact
                            | ospf_rust_core::solver::ProofReliability::Reliable
                    )
                    && proof.completeness == ospf_rust_core::solver::ProofCompleteness::Complete
                    && proof.kind
                        == match report.problem_status {
                            ProblemStatus::Unbounded => {
                                ospf_rust_core::solver::ProofKind::Unboundedness
                            }
                            ProblemStatus::InfeasibleOrUnbounded => {
                                ospf_rust_core::solver::ProofKind::InfeasibleOrUnbounded
                            }
                            _ => unreachable!("terminal proof branch is exhaustive"),
                        }
                    && report.termination_reason
                        == ospf_rust_core::solver::TerminationReason::Completed
            })
        }
        ProblemStatus::Feasible | ProblemStatus::Unknown => false,
    }
}

fn report_is_cancellation_terminal(report: &SolveReport<f64>) -> bool {
    matches!(
        report.termination_reason,
        TerminationReason::Cancelled | TerminationReason::Interrupted
    )
}

pub(crate) fn aggregate_attempt_id(wrapper_name: &str) -> String {
    format!("aggregate:{}:{}", wrapper_name.len(), wrapper_name)
}

pub(crate) fn child_attempt_id(
    aggregate_id: &str,
    solver_name: &str,
    solver_index: usize,
) -> String {
    format!(
        "{}/attempt:{}:{}:{}",
        aggregate_id,
        solver_name.len(),
        solver_name,
        solver_index
    )
}

/// 组合求解器的内部 attempt 载荷 / Internal attempt payload for combinatorial solvers.
///
/// 并行 worker 在发布结果前冻结完成时间；serial 调用方可以省略该值并沿用调用时钟。
/// Parallel workers freeze completion time before publishing a result; serial callers may omit it
/// and use the aggregation clock.
pub(crate) struct CombinatorialAttempt {
    /// solver 在 wrapper 中的稳定索引 / Stable solver index within the wrapper.
    pub solver_index: usize,
    /// solver 名称 / Solver name.
    pub solver_name: String,
    /// solver 结果 / Solver result.
    pub result: Result<SolveReport<f64>>,
    /// worker 完成时间 / Worker completion time.
    pub completed_at_epoch_ms: Option<u64>,
}

impl CombinatorialAttempt {
    #[cfg(test)]
    pub(crate) fn from_result(
        solver_index: usize,
        solver_name: String,
        result: Result<SolveReport<f64>>,
    ) -> Self {
        Self {
            solver_index,
            solver_name,
            result,
            completed_at_epoch_ms: None,
        }
    }
}

fn apply_attempt_completion(
    trace: &mut ospf_rust_core::solver::SolveAttemptTrace,
    completed_at_epoch_ms: Option<u64>,
) {
    let Some(completed_at_epoch_ms) = completed_at_epoch_ms else {
        return;
    };
    trace.completed_at_epoch_ms = Some(completed_at_epoch_ms);
    trace.started_at_epoch_ms = Some(
        completed_at_epoch_ms
            .saturating_sub(trace.elapsed.as_millis().min(u64::MAX as u128) as u64),
    );
}

/// 将取消的 backend 错误保留为结构化取消报告 / Preserve backend cancellation errors as structured reports.
pub(crate) fn preserve_cancelled_attempts(
    attempts: Vec<(usize, String, Result<SolveReport<f64>>)>,
    child_handles: &[SolveHandle],
) -> Vec<CombinatorialAttempt> {
    attempts
        .into_iter()
        .map(|(solver_index, solver_name, result)| {
            let handle = child_handles.get(solver_index);
            let result = if handle.is_some_and(SolveHandle::cancellation_preceded_completion) {
                let handle = handle.expect("cancellation order was checked above");
                cancelled_solve_report(
                    SolverProvenance {
                        solver_id: solver_name.clone(),
                        backend_name: "framework-combinatorial".to_owned(),
                        ..SolverProvenance::default()
                    },
                    handle,
                )
            } else {
                result
            };
            CombinatorialAttempt {
                solver_index,
                solver_name,
                result,
                completed_at_epoch_ms: handle.and_then(SolveHandle::completed_at_epoch_ms),
            }
        })
        .collect()
}

/// 聚合组合求解 attempts / Aggregate combinatorial solve attempts.
#[cfg(test)]
pub(crate) fn aggregate_combinatorial_reports(
    wrapper_name: &str,
    selection: CombinatorialSelection,
    attempts: Vec<(usize, String, Result<SolveReport<f64>>)>,
) -> Result<CombinatorialSolveReport<f64>> {
    aggregate_combinatorial_reports_with_metadata(
        wrapper_name,
        selection,
        attempts
            .into_iter()
            .map(|(solver_index, solver_name, result)| {
                CombinatorialAttempt::from_result(solver_index, solver_name, result)
            })
            .collect(),
    )
}

/// 聚合带完成快照的组合求解 attempts / Aggregate combinatorial attempts with completion snapshots.
pub(crate) fn aggregate_combinatorial_reports_with_metadata(
    wrapper_name: &str,
    selection: CombinatorialSelection,
    mut attempts: Vec<CombinatorialAttempt>,
) -> Result<CombinatorialSolveReport<f64>> {
    attempts.sort_by_key(|attempt| attempt.solver_index);

    let mut traces = Vec::with_capacity(attempts.len());
    let mut reports = Vec::new();
    let mut failure_issues = Vec::new();
    let aggregate_id = aggregate_attempt_id(wrapper_name);

    for attempt in attempts {
        let solver_index = attempt.solver_index;
        let solver_name = attempt.solver_name;
        let completion = attempt.completed_at_epoch_ms;
        let result = attempt.result;
        let attempt_id = child_attempt_id(&aggregate_id, &solver_name, solver_index);
        match result {
            Ok(report) => {
                report.validate()?;
                if !report_is_selectable(&report) && !report_is_cancellation_terminal(&report) {
                    failure_issues.push(SolveIssue::new(
                        "AttemptNotSelectable",
                        format!(
                            "attempt `{}` did not provide an incumbent or a verified terminal proof",
                            attempt_id
                        ),
                    ));
                }
                let mut trace = ospf_rust_core::solver::SolveAttemptTrace::from_report_with_parent(
                    attempt_id.clone(),
                    Some(aggregate_id.clone()),
                    report.provenance.clone(),
                    &report,
                    report.statistics.solve_time,
                );
                apply_attempt_completion(&mut trace, completion);
                traces.push(trace);
                reports.push((solver_index, attempt_id, report));
            }
            Err(error) => {
                let issue = SolveIssue::new("AttemptFailed", error.to_string());
                failure_issues.push(issue.clone());
                let mut trace = ospf_rust_core::solver::SolveAttemptTrace::failed_with_parent(
                    attempt_id,
                    Some(aggregate_id.clone()),
                    SolverProvenance {
                        solver_id: solver_name,
                        backend_name: "framework-combinatorial".to_owned(),
                        ..SolverProvenance::default()
                    },
                    issue,
                    Duration::ZERO,
                );
                apply_attempt_completion(&mut trace, completion);
                traces.push(trace);
            }
        }
    }

    let selected = match selection {
        CombinatorialSelection::First => reports
            .iter()
            .find(|(_, _, report)| report_is_selectable(report)),
        CombinatorialSelection::FirstCompleted(solver_index) => reports
            .iter()
            .find(|(index, _, report)| *index == solver_index && report_is_selectable(report))
            .or_else(|| {
                reports
                    .iter()
                    .find(|(_, _, report)| report_is_selectable(report))
            }),
        CombinatorialSelection::Best(objective_category) => {
            let mut candidates = reports
                .iter()
                .filter(|(_, _, report)| report.has_incumbent())
                .collect::<Vec<_>>();
            candidates.sort_by(|left, right| {
                let left_objective = left
                    .2
                    .solution
                    .as_ref()
                    .and_then(|solution| solution.objective_value.or(solution.objective));
                let right_objective = right
                    .2
                    .solution
                    .as_ref()
                    .and_then(|solution| solution.objective_value.or(solution.objective));
                let ordering = match (left_objective, right_objective) {
                    (Some(left), Some(right)) => match objective_category {
                        ObjectiveCategory::Minimum => left.total_cmp(&right),
                        ObjectiveCategory::Maximum => right.total_cmp(&left),
                    },
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                };
                ordering.then_with(|| left.0.cmp(&right.0))
            });
            candidates.into_iter().next()
        }
    };
    // 取消/中断没有数学 incumbent，但仍是必须保留的正常执行终态。
    // Cancellation/interruption has no mathematical incumbent, but remains a
    // publishable execution terminal that must not be rewritten as BackendFailure.
    let selected = selected.or_else(|| {
        reports
            .iter()
            .find(|(_, _, report)| report_is_cancellation_terminal(report))
    });

    let (report, selected_attempt_id, selection_reason) =
        if let Some((_, attempt_id, report)) = selected {
            let mut report = report.clone();
            report
                .diagnostics
                .extensions
                .insert("combinatorial.wrapper".to_owned(), wrapper_name.to_owned());
            report.diagnostics.extensions.insert(
                "combinatorial.aggregateAttemptId".to_owned(),
                aggregate_id.clone(),
            );
            report.diagnostics.extensions.insert(
                "combinatorial.attemptCount".to_owned(),
                traces.len().to_string(),
            );
            report.diagnostics.issues.extend(failure_issues);
            report.validate()?;
            (
                report,
                Some(attempt_id.clone()),
                Some(format!("selected attempt {}", attempt_id)),
            )
        } else {
            let mut diagnostics = SolveDiagnostics {
                issues: failure_issues,
                ..SolveDiagnostics::default()
            };
            diagnostics
                .extensions
                .insert("combinatorial.wrapper".to_owned(), wrapper_name.to_owned());
            diagnostics
                .extensions
                .insert("combinatorial.aggregateAttemptId".to_owned(), aggregate_id);
            diagnostics.extensions.insert(
                "combinatorial.attemptCount".to_owned(),
                traces.len().to_string(),
            );
            let report =
                SolveReport::builder(ProblemStatus::Unknown, TerminationReason::BackendFailure)
                    .diagnostics(diagnostics)
                    .provenance(SolverProvenance {
                        solver_id: wrapper_name.to_owned(),
                        backend_name: "framework-combinatorial".to_owned(),
                        ..SolverProvenance::default()
                    })
                    .build()?;
            (
                report,
                None,
                Some("no attempt produced a usable report".to_owned()),
            )
        };

    let aggregate = CombinatorialSolveReport {
        report,
        attempts: traces,
        selected_attempt_id,
        selection_reason,
    };
    aggregate.validate()?;
    Ok(aggregate)
}

impl FeasibleSolution {
    /// 创建新的可行解 / Create new feasible solution
    pub fn new(obj: f64, solution: Vec<f64>) -> Self {
        Self {
            obj,
            solution,
            time: Duration::ZERO,
            possible_best_obj: None,
            gap: 0.0,
            benders_iterations: None,
            benders_runtime_metrics: None,
            source_report: None,
        }
    }

    /// 从求解输出创建 / Create from solver output
    pub fn from_output(output: &SolverOutput) -> Option<Self> {
        Self::try_from_output(output, SolveValueConversionPolicy::Strict).ok()
    }

    /// 从求解输出提取 core typed 可行输出 / Extract core typed feasible output from solver output
    pub(crate) fn try_feasible_typed_output_from_output(
        output: &SolverOutput,
        policy: SolveValueConversionPolicy,
    ) -> Result<FeasibleSolverOutput<f64>> {
        let report = output.clone().try_into_solve_report()?;
        if !report.has_incumbent() {
            if matches!(
                output.status,
                ospf_rust_core::solver::SolverStatus::Feasible
                    | ospf_rust_core::solver::SolverStatus::Optimal
            ) && output.solution.is_none()
            {
                return Err(CoreError::Solver(SolverError::ContractViolation(
                    "solver returned feasible status but missing solution vector".to_owned(),
                )));
            }
            return Err(CoreError::Solver(SolverError::ContractViolation(format!(
                "solver output status {:?} is not feasible",
                output.status
            ))));
        }
        output.clone().try_into_feasible_typed::<f64>(policy)
    }

    /// 从求解输出创建（可报错）/ Create from solver output (fallible)
    pub fn try_from_output(
        output: &SolverOutput,
        policy: SolveValueConversionPolicy,
    ) -> Result<Self> {
        let source_report = output.clone().try_into_solve_report()?;
        let typed_output = Self::try_feasible_typed_output_from_output(output, policy).map_err(
            |err| match err {
                CoreError::Solver(SolverError::NoSolution) => {
                    CoreError::Solver(SolverError::ContractViolation(
                        "solver returned feasible status but missing solution vector".into(),
                    ))
                }
                other => other,
            },
        )?;
        let obj = typed_output.objective_value.ok_or_else(|| {
            CoreError::Solver(SolverError::ContractViolation(
                "solver returned feasible status but missing objective value".into(),
            ))
        })?;
        Ok(Self {
            obj,
            solution: typed_output.solution,
            time: typed_output.solve_time,
            possible_best_obj: typed_output.best_bound,
            gap: typed_output.mip_gap.unwrap_or(0.0),
            benders_iterations: None,
            benders_runtime_metrics: None,
            source_report: Some(source_report),
        })
    }

    /// 绑定产生该兼容解的统一报告 / Attach the unified report that produced this compatibility solution.
    pub fn with_source_report(mut self, report: SolveReport<f64>) -> Self {
        self.source_report = Some(report);
        self
    }

    /// 设置 Benders 迭代次数 / Set Benders iteration count
    pub fn with_benders_iterations(mut self, iterations: usize) -> Self {
        self.benders_iterations = Some(iterations);
        self
    }

    /// 设置 Benders 运行时指标 / Set Benders runtime metrics
    pub fn with_benders_runtime_metrics(mut self, metrics: BendersRuntimeMetrics) -> Self {
        self.benders_runtime_metrics = Some(metrics);
        self
    }

    /// 转换为泛型可行解 / Convert into generic feasible solution
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<FeasibleSolutionV<V>>
    where
        V: SolveValue,
    {
        Ok(FeasibleSolutionV {
            obj: V::from_f64_with_policy(self.obj, policy)?,
            solution: self
                .solution
                .into_iter()
                .map(|value| V::from_f64_with_policy(value, policy))
                .collect::<Result<Vec<V>>>()?,
            time: self.time,
            possible_best_obj: self
                .possible_best_obj
                .map(|value| V::from_f64_with_policy(value, policy))
                .transpose()?,
            gap: self.gap,
            benders_iterations: self.benders_iterations,
            benders_runtime_metrics: self.benders_runtime_metrics,
        })
    }

    /// 从 core 输出直接构建泛型可行解 / Build generic feasible solution from core output
    pub fn try_from_output_typed<V>(
        output: &SolverOutput,
        policy: SolveValueConversionPolicy,
    ) -> Result<FeasibleSolutionV<V>>
    where
        V: SolveValue,
    {
        let typed_output = output.clone().try_into_feasible_typed::<V>(policy)?;
        FeasibleSolutionV::try_from_typed_output(typed_output)
    }

    /// 从统一报告兼容投影为 framework 可行解 / Project a unified report into the legacy framework solution.
    pub fn try_from_report(report: &SolveReport<f64>) -> Result<Self> {
        report.validate()?;
        if matches!(
            report.termination_reason,
            TerminationReason::Cancelled | TerminationReason::Interrupted
        ) {
            let error = match report.termination_reason {
                TerminationReason::Cancelled => SolverError::Cancelled("REPORT".to_owned()),
                TerminationReason::Interrupted => {
                    SolverError::Interrupted("UserInterrupt".to_owned())
                }
                _ => unreachable!("cancellation branch is exhaustive"),
            };
            return Err(CoreError::Solver(error));
        }
        if matches!(
            report.problem_status,
            ProblemStatus::Infeasible
                | ProblemStatus::Unbounded
                | ProblemStatus::InfeasibleOrUnbounded
        ) {
            return Err(CoreError::Solver(SolverError::ContractViolation(format!(
                "report problem status {:?} cannot be projected to FeasibleSolution",
                report.problem_status
            ))));
        }
        let solution = report.solution.as_ref().ok_or_else(|| {
            CoreError::Solver(SolverError::ContractViolation(
                "report has no incumbent solution".to_owned(),
            ))
        })?;
        let objective = solution
            .objective_value
            .or(solution.objective)
            .ok_or_else(|| {
                CoreError::Solver(SolverError::ContractViolation(
                    "report incumbent has no objective value".to_owned(),
                ))
            })?;
        Ok(Self {
            obj: objective,
            solution: solution.values.clone(),
            time: report.statistics.solve_time,
            possible_best_obj: report.statistics.best_bound_value,
            gap: report.statistics.relative_gap.unwrap_or(0.0),
            benders_iterations: (report.trace.total_iterations > 0)
                .then_some(report.trace.total_iterations),
            benders_runtime_metrics: None,
            source_report: Some(report.clone()),
        })
    }

    /// 转换为统一报告 / Convert the framework solution into a unified report.
    pub fn to_solve_report(&self, solver_name: &str) -> Result<SolveReport<f64>> {
        let mut projected = self.to_legacy_solve_report(solver_name)?;
        if let Some(source_report) = &self.source_report {
            source_report.validate()?;
            if self.benders_runtime_metrics.is_none() {
                let mut report = source_report.clone();
                if let Some(solution) = report.solution.as_mut() {
                    solution.values = self.solution.clone();
                    solution.objective = Some(self.obj);
                    solution.objective_value = Some(self.obj);
                }
                report.validate()?;
                return Ok(report);
            }

            // Benders 的子问题报告只证明子问题，不证明整个分解问题。
            // A Benders subproblem report certifies only the subproblem, not the full decomposition.
            projected.provenance = source_report.provenance.clone();
            projected.fingerprints = source_report.fingerprints.clone();
            projected.model_mapping = source_report.model_mapping.clone();
            projected.warnings.extend(source_report.warnings.clone());
            let mut diagnostics = source_report.diagnostics.clone();
            diagnostics
                .warnings
                .extend(projected.diagnostics.warnings.clone());
            diagnostics
                .issues
                .extend(projected.diagnostics.issues.clone());
            diagnostics
                .extensions
                .extend(projected.diagnostics.extensions.clone());
            projected.diagnostics = diagnostics;
            if let (Some(source_solution), Some(projected_solution)) =
                (source_report.solution.as_ref(), projected.solution.as_mut())
            {
                projected_solution.value = source_solution.value;
                projected_solution.stable_values = source_solution.stable_values.clone();
                projected_solution.dual_solution = source_solution.dual_solution.clone();
                projected_solution.quadratic_dual_solution =
                    source_solution.quadratic_dual_solution.clone();
                projected_solution.pool = source_solution.pool.clone();
            }
            projected.proof = None;
            projected.solution_presence = ospf_rust_core::solver::SolutionPresence::Incumbent;
            projected.validate()?;
        }
        Ok(projected)
    }

    fn to_legacy_solve_report(&self, solver_name: &str) -> Result<SolveReport<f64>> {
        let runtime_metrics = self.benders_runtime_metrics.as_ref();
        // bound 是报告的唯一 gap 来源，忽略 legacy 字段中可能残留的默认值。
        // The bound is the sole source of report gap; ignore a stale legacy default.
        let relative_gap = self
            .possible_best_obj
            .map(|best_bound| (self.obj - best_bound).abs() / self.obj.abs().max(1.0));
        let absolute_gap = self
            .possible_best_obj
            .map(|best_bound| (self.obj - best_bound).abs());
        let statistics = SolveStatistics {
            solve_time: self.time,
            iterations: self
                .benders_iterations
                .or_else(|| runtime_metrics.map(|metrics| metrics.executed_iterations)),
            best_bound_value: self.possible_best_obj,
            absolute_gap,
            relative_gap,
            ..SolveStatistics::default()
        };
        let mut diagnostics = SolveDiagnostics::default();
        let mut trace = SolveTrace {
            global_lower_bound: self.possible_best_obj,
            upper_bound: Some(self.obj),
            relative_gap,
            total_iterations: self
                .benders_iterations
                .or_else(|| runtime_metrics.map(|metrics| metrics.executed_iterations))
                .unwrap_or_default(),
            elapsed: self.time,
            ..SolveTrace::default()
        };
        let termination_reason = if let Some(metrics) = runtime_metrics {
            trace.generated_columns = metrics.total_cuts;
            diagnostics.extensions.insert(
                "benders.executedIterations".to_owned(),
                metrics.executed_iterations.to_string(),
            );
            diagnostics.extensions.insert(
                "benders.bestSolutionIteration".to_owned(),
                metrics
                    .best_solution_iteration
                    .map(|iteration| iteration.to_string())
                    .unwrap_or_else(|| "none".to_owned()),
            );
            diagnostics.extensions.insert(
                "benders.totalCuts".to_owned(),
                metrics.total_cuts.to_string(),
            );
            diagnostics.extensions.insert(
                "benders.noCutIterations".to_owned(),
                metrics.no_cut_iterations.to_string(),
            );
            diagnostics.extensions.insert(
                "benders.noObjectiveImprovementIterations".to_owned(),
                metrics.no_obj_improvement_iterations.to_string(),
            );
            diagnostics.extensions.insert(
                "benders.iterationSnapshotCount".to_owned(),
                metrics.iteration_snapshots.len().to_string(),
            );
            trace.iteration_snapshots = metrics
                .iteration_snapshots
                .iter()
                .map(|snapshot| SolveIterationSnapshot {
                    iteration: snapshot.iteration,
                    stage: "benders/master-subproblem".to_owned(),
                    objective_value: Some(snapshot.master_obj),
                    best_bound: snapshot
                        .bound_valid
                        .then_some(snapshot.master_bound)
                        .flatten(),
                    relative_gap: snapshot
                        .bound_valid
                        .then_some(snapshot.master_gap)
                        .flatten(),
                    items_added: snapshot.cuts_added,
                    items_total: snapshot.total_cuts,
                    proof_reference: snapshot.proof_reference.clone(),
                    master_attempt_id: Some(snapshot.master_attempt_id.clone()),
                    subproblem_attempt_id: snapshot.subproblem_attempt_id.clone(),
                    master_problem_status: snapshot
                        .master_report
                        .as_ref()
                        .map(|report| report.problem_status),
                    master_termination_reason: snapshot
                        .master_report
                        .as_ref()
                        .map(|report| report.termination_reason),
                    subproblem_problem_status: snapshot
                        .subproblem_report
                        .as_ref()
                        .map(|report| report.problem_status),
                    subproblem_termination_reason: snapshot
                        .subproblem_report
                        .as_ref()
                        .map(|report| report.termination_reason),
                    bound_valid: Some(snapshot.bound_valid),
                })
                .collect();
            match metrics.stop_reason {
                Some(BendersStopReason::IterationLimit) => TerminationReason::IterationLimit,
                Some(BendersStopReason::CutStall | BendersStopReason::ObjectiveStall) => {
                    TerminationReason::StallNodeLimit
                }
                Some(BendersStopReason::MasterTermination(reason)) => reason,
                Some(BendersStopReason::SubproblemTermination(reason)) => {
                    diagnostics.extensions.insert(
                        "benders.incompleteProof".to_owned(),
                        "subproblem-optimality-proof-unavailable".to_owned(),
                    );
                    diagnostics.warnings.push(SolveWarning::new(
                        "IncompleteBendersProof",
                        format!(
                            "subproblem did not provide a reliable optimality proof; termination={reason:?}"
                        ),
                    ));
                    reason
                }
                None => TerminationReason::Completed,
            }
        } else {
            TerminationReason::Completed
        };
        SolveReport::builder(ProblemStatus::Feasible, termination_reason)
            .solution(SolveSolution {
                value: None,
                values: self.solution.clone(),
                stable_values: Default::default(),
                objective: Some(self.obj),
                objective_value: Some(self.obj),
                dual_solution: None,
                quadratic_dual_solution: None,
                pool: Vec::new(),
            })
            .statistics(statistics)
            .diagnostics(diagnostics)
            .trace(trace)
            .provenance(SolverProvenance {
                solver_id: solver_name.to_owned(),
                backend_name: "framework-legacy-adapter".to_owned(),
                ..SolverProvenance::default()
            })
            .build()
    }
}

impl<V> FeasibleSolutionV<V>
where
    V: SolveValue,
{
    /// 从 core typed 输出创建 / Create from core typed output
    pub fn try_from_typed_output(output: FeasibleSolverOutput<V>) -> Result<Self> {
        let obj = output.objective_value.ok_or_else(|| {
            CoreError::Solver(SolverError::ContractViolation(
                "typed feasible output missing objective value".to_string(),
            ))
        })?;
        Ok(Self {
            obj,
            solution: output.solution,
            time: output.solve_time,
            possible_best_obj: output.best_bound,
            gap: output.mip_gap.unwrap_or(0.0),
            benders_iterations: None,
            benders_runtime_metrics: None,
        })
    }
}

/// 线性对偶解 / Linear Dual Solution
#[derive(Debug, Clone, Default)]
pub struct LinearDualSolution {
    /// 约束对偶值 / Constraint dual values
    pub constraints: Vec<f64>,
    /// 变量对偶值 / Variable dual values
    pub variables: Vec<f64>,
}

/// 泛型线性对偶解 / Generic linear dual solution
#[derive(Debug, Clone, Default)]
pub struct LinearDualSolutionV<V>
where
    V: SolveValue,
{
    /// 约束对偶值 / Constraint dual values
    pub constraints: Vec<V>,
    /// 变量对偶值 / Variable dual values
    pub variables: Vec<V>,
}

impl LinearDualSolution {
    /// 创建新的对偶解 / Create new dual solution
    pub fn new(constraints: Vec<f64>, variables: Vec<f64>) -> Self {
        Self {
            constraints,
            variables,
        }
    }

    /// 转换为元对偶解 / Convert to meta dual solution
    pub fn to_meta(&self) -> MetaDualSolution {
        MetaDualSolution {
            constraints: self
                .constraints
                .iter()
                .enumerate()
                .map(|(i, &v)| (i, v))
                .collect(),
            symbols: Vec::new(),
            variables: self
                .variables
                .iter()
                .enumerate()
                .map(|(i, &v)| (i, v))
                .collect(),
        }
    }

    /// 转换为泛型对偶解 / Convert into generic dual solution
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<LinearDualSolutionV<V>>
    where
        V: SolveValue,
    {
        Ok(LinearDualSolutionV {
            constraints: self
                .constraints
                .into_iter()
                .map(|value| V::from_f64_with_policy(value, policy))
                .collect::<Result<Vec<V>>>()?,
            variables: self
                .variables
                .into_iter()
                .map(|value| V::from_f64_with_policy(value, policy))
                .collect::<Result<Vec<V>>>()?,
        })
    }
}

/// 元对偶解 / Meta Dual Solution
#[derive(Debug, Clone, Default)]
pub struct MetaDualSolution {
    /// 约束对偶值 / Constraint dual values
    pub constraints: Vec<(usize, f64)>,
    /// 符号对偶值 / Symbol dual values
    pub symbols: Vec<(usize, f64)>,
    /// 变量对偶值 / Variable dual values
    pub variables: Vec<(usize, f64)>,
}

/// LP 求解结果 / LP Solving Result
#[derive(Debug, Clone)]
pub struct LPResult {
    /// 求解结果 / Solver result
    pub result: FeasibleSolution,
    /// 对偶解 / Dual solution
    pub dual_solution: LinearDualSolution,
    /// core 或 backend 生成的原始统一报告 / Original unified report produced by core or backend.
    pub report: Option<SolveReport<f64>>,
}

/// 泛型 LP 求解结果 / Generic LP solving result
#[derive(Debug, Clone)]
pub struct LPResultV<V>
where
    V: SolveValue,
{
    /// 求解结果 / Solver result
    pub result: FeasibleSolutionV<V>,
    /// 对偶解 / Dual solution
    pub dual_solution: LinearDualSolutionV<V>,
    /// typed 统一求解报告 / Typed unified solve report.
    pub report: Option<SolveReport<V>>,
}

impl LPResult {
    /// 创建新的 LP 结果 / Create new LP result
    pub fn new(result: FeasibleSolution, dual_solution: LinearDualSolution) -> Self {
        Self {
            result,
            dual_solution,
            report: None,
        }
    }

    /// 绑定原始统一报告 / Attach the original unified report.
    pub fn with_report(mut self, report: SolveReport<f64>) -> Self {
        self.result = self.result.clone().with_source_report(report.clone());
        self.report = Some(report);
        self
    }

    /// 转换为泛型 LP 结果 / Convert into generic LP result
    pub fn try_into_typed<V>(self, policy: SolveValueConversionPolicy) -> Result<LPResultV<V>>
    where
        V: SolveValue,
    {
        Ok(LPResultV {
            result: self.result.try_into_typed(policy)?,
            dual_solution: self.dual_solution.try_into_typed(policy)?,
            report: self
                .report
                .map(|report| convert_report_value(report, policy))
                .transpose()?,
        })
    }

    /// 转换为统一 LP 报告 / Convert the LP result into a unified LP report.
    pub fn to_solve_report(&self, solver_name: &str) -> Result<SolveReport<f64>> {
        if let Some(mut report) = self.report.clone() {
            if let Some(solution) = report.solution.as_mut() {
                solution.dual_solution = Some(self.dual_solution.constraints.clone());
            }
            report.validate()?;
            return Ok(report);
        }
        let mut report = self.result.to_solve_report(solver_name)?;
        if let Some(solution) = report.solution.as_mut() {
            solution.dual_solution = Some(self.dual_solution.constraints.clone());
        }
        report.validate()?;
        Ok(report)
    }

    /// 从已验证的统一 LP 报告投影为旧 LP 结果 / Project a verified unified LP report into the legacy LP result.
    pub fn try_from_solve_report(report: &SolveReport<f64>) -> Result<Self> {
        ospf_rust_core::solver::require_optimal_lp_certificate(report).map_err(|error| {
            CoreError::Solver(SolverError::ContractViolation(format!(
                "LP report is not certifying: {}",
                error
            )))
        })?;
        let result = FeasibleSolution::try_from_report(report)?;
        let dual_solution = report
            .solution
            .as_ref()
            .and_then(|solution| solution.dual_solution.clone())
            .ok_or_else(|| {
                CoreError::Solver(SolverError::ContractViolation(
                    "certifying LP report has no dual solution".to_owned(),
                ))
            })?;
        Ok(
            Self::new(result, LinearDualSolution::new(dual_solution, Vec::new()))
                .with_report(report.clone()),
        )
    }
}

/// Nightly: 可调用求解器包装器 / Nightly: callable solver wrapper
#[cfg(all(feature = "nightly", not(feature = "async")))]
#[derive(Clone)]
pub struct MetaSolveFn<'a, S>
where
    S: ColumnGenerationSolver + ?Sized,
{
    solver: &'a S,
    options: FrameworkSolveOptions,
}

#[cfg(all(feature = "nightly", not(feature = "async")))]
impl<'a, S> MetaSolveFn<'a, S>
where
    S: ColumnGenerationSolver + ?Sized,
{
    /// 创建可调用包装器 / Create callable wrapper
    pub fn new(solver: &'a S) -> Self {
        Self {
            solver,
            options: FrameworkSolveOptions::default(),
        }
    }

    /// 设置参数对象 / Set options object
    pub fn with_options(mut self, options: FrameworkSolveOptions) -> Self {
        self.options = options;
        self
    }
}

#[cfg(all(feature = "nightly", not(feature = "async")))]
impl<'a, S> FnOnce<(&MetaModel<f64>,)> for MetaSolveFn<'a, S>
where
    S: ColumnGenerationSolver + ?Sized,
{
    type Output = Result<FeasibleSolution>;

    extern "rust-call" fn call_once(self, args: (&MetaModel<f64>,)) -> Self::Output {
        self.solver.solve_with_options(args.0, self.options)
    }
}

#[cfg(all(feature = "nightly", not(feature = "async")))]
impl<'a, S> FnMut<(&MetaModel<f64>,)> for MetaSolveFn<'a, S>
where
    S: ColumnGenerationSolver + ?Sized,
{
    extern "rust-call" fn call_mut(&mut self, args: (&MetaModel<f64>,)) -> Self::Output {
        self.solver.solve_with_options(args.0, self.options.clone())
    }
}

#[cfg(all(feature = "nightly", not(feature = "async")))]
impl<'a, S> Fn<(&MetaModel<f64>,)> for MetaSolveFn<'a, S>
where
    S: ColumnGenerationSolver + ?Sized,
{
    extern "rust-call" fn call(&self, args: (&MetaModel<f64>,)) -> Self::Output {
        self.solver.solve_with_options(args.0, self.options.clone())
    }
}

/// 列生成求解器 trait / Column Generation Solver Trait
///
/// 定义列生成算法的求解器接口。
/// Defines solver interface for column generation algorithms.
///
/// 推荐应用层只记忆两类入口：
/// - `solve(&meta_model)`（最短路径）
/// - `solve_with_options(&meta_model, options)`（需要模型外参数时）
///
/// The recommended application-facing entries are:
/// - `solve(&meta_model)` (shortest path)
/// - `solve_with_options(&meta_model, options)` (when non-model arguments are needed)
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait ColumnGenerationSolver: Send + Sync {
    /// 获取求解器名称 / Get solver name
    fn name(&self) -> &str;

    /// Nightly: 获取可调用包装器 / Nightly: get callable wrapper
    #[cfg(all(feature = "nightly", not(feature = "async")))]
    fn as_fn<'a>(&'a self) -> MetaSolveFn<'a, Self>
    where
        Self: Sized,
    {
        MetaSolveFn::new(self)
    }

    /// Nightly: 获取带参数可调用包装器 / Nightly: get callable wrapper with options
    #[cfg(all(feature = "nightly", not(feature = "async")))]
    fn as_fn_with_options<'a>(&'a self, options: FrameworkSolveOptions) -> MetaSolveFn<'a, Self>
    where
        Self: Sized,
    {
        MetaSolveFn::new(self).with_options(options)
    }

    /// 以统一报告求解 MetaModel / Solve a MetaModel and return a unified report.
    #[cfg(feature = "async")]
    fn solve_report<'a, V>(
        &'a self,
        meta_model: &'a ospf_rust_core::model::MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SolveReport<V>>> + Send + 'a>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_report_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 以统一报告和参数对象求解 MetaModel / Solve a MetaModel with options and return a unified report.
    #[cfg(feature = "async")]
    fn solve_report_with_options<'a, V>(
        &'a self,
        meta_model: &'a ospf_rust_core::model::MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SolveReport<V>>> + Send + 'a>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        let mechanism_model = match meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(error) => return Box::pin(std::future::ready(Err(error))),
        };
        let mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &mechanism_model,
            policy,
        ) {
            Ok(model) => model,
            Err(error) => return Box::pin(std::future::ready(Err(error))),
        };
        let triad_model = match mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(error) => return Box::pin(std::future::ready(Err(error))),
        };
        let options = if options.name.is_some() {
            options
        } else {
            options.with_name(self.name())
        };
        Box::pin(async move {
            let report = self
                .solve_milp_report_with_options(&triad_model, options)
                .await?;
            convert_report_value(report, policy)
        })
    }

    /// 以统一报告求解 MILP / Solve a MILP and return a unified report.
    #[cfg(feature = "async")]
    async fn solve_milp_combinatorial_report_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = pre_cancelled_report(self.name(), &options)? {
            return Ok(report);
        }
        let solution = self.solve_milp_with_options(model, options).await?;
        let report = solution.to_solve_report(self.name())?;
        Ok(CombinatorialSolveReport::single(
            report,
            format!("{}#0", self.name()),
        ))
    }

    /// 以统一报告求解 MILP / Solve a MILP and return a unified report.
    #[cfg(feature = "async")]
    async fn solve_milp_report_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_milp_combinatorial_report_with_options(model, options)
            .await
            .map(|aggregate| aggregate.report)
    }

    /// 以统一报告求解 LP / Solve an LP and return a unified report.
    #[cfg(feature = "async")]
    async fn solve_lp_combinatorial_report_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = pre_cancelled_report(self.name(), &options)? {
            return Ok(report);
        }
        let result = self.solve_lp_with_options(model, options).await?;
        let report = result.to_solve_report(self.name())?;
        Ok(CombinatorialSolveReport::single(
            report,
            format!("{}#0", self.name()),
        ))
    }

    /// 以统一报告求解 LP / Solve an LP and return a unified report.
    #[cfg(feature = "async")]
    async fn solve_lp_report_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_lp_combinatorial_report_with_options(model, options)
            .await
            .map(|aggregate| aggregate.report)
    }

    /// 以统一报告求解 MetaModel 并转换 typed 结果 / Solve a MetaModel and convert its unified report to typed values.
    #[cfg(feature = "async")]
    fn solve_report_typed_with_options<'a, V>(
        &'a self,
        meta_model: &'a ospf_rust_core::model::MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SolveReport<V>>> + Send + 'a>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_report_with_options(meta_model, options)
    }

    /// 统一入口：求解 MetaModel（默认 MILP 路径）/ Unified entry: solve MetaModel (MILP by default)
    #[cfg(feature = "async")]
    fn solve<'a, V>(
        &'a self,
        meta_model: &'a ospf_rust_core::model::MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 统一入口：求解 MetaModel（参数对象，默认 MILP 路径）/
    /// Unified entry: solve MetaModel with options object (MILP by default)
    #[cfg(feature = "async")]
    fn solve_with_options<'a, V>(
        &'a self,
        meta_model: &'a ospf_rust_core::model::MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let mechanism_model = match meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &mechanism_model,
            options.value_conversion_policy,
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let triad_model = match mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let options = if options.name.is_some() {
            options
        } else {
            options.with_name(self.name())
        };
        Box::pin(async move { self.solve_milp_with_options(&triad_model, options).await })
    }

    /// 统一入口：求解 MetaModel 并返回 typed 可行解 /
    /// Unified entry: solve MetaModel and return typed feasible solution
    #[cfg(feature = "async")]
    fn solve_typed<'a, V>(
        &'a self,
        meta_model: &'a ospf_rust_core::model::MetaModel<V>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<FeasibleSolutionV<V>>> + Send + 'a>,
    >
    where
        Self: Sized,
        V: SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_typed_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 统一入口：求解 MetaModel 并返回 typed 可行解（参数对象）/
    /// Unified entry: solve MetaModel and return typed feasible solution (options object)
    #[cfg(feature = "async")]
    fn solve_typed_with_options<'a, V>(
        &'a self,
        meta_model: &'a ospf_rust_core::model::MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<FeasibleSolutionV<V>>> + Send + 'a>,
    >
    where
        Self: Sized,
        V: SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        let solve_future = self.solve_with_options(meta_model, options);
        Box::pin(async move { solve_future.await?.try_into_typed(policy) })
    }

    /// 求解 MILP（参数对象）/ Solve MILP with options object
    #[cfg(feature = "async")]
    async fn solve_milp_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution>;

    /// 以统一报告求解 MetaModel / Solve a MetaModel and return a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_report<V>(
        &self,
        meta_model: &ospf_rust_core::model::MetaModel<V>,
    ) -> Result<SolveReport<V>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_report_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 以统一报告和参数对象求解 MetaModel / Solve a MetaModel with options and return a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_report_with_options<V>(
        &self,
        meta_model: &ospf_rust_core::model::MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<V>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        let mechanism_model = meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        let mechanism_model =
            ospf_rust_core::solver::convert_mechanism_model_to_f64(&mechanism_model, policy)?;
        let triad_model = mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        let options = if options.name.is_some() {
            options
        } else {
            options.with_name(self.name())
        };
        let report = self.solve_milp_report_with_options(&triad_model, options)?;
        convert_report_value(report, policy)
    }

    /// 以统一报告求解 MILP / Solve a MILP and return a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_milp_combinatorial_report_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = pre_cancelled_report(self.name(), &options)? {
            return Ok(report);
        }
        let solution = self.solve_milp_with_options(model, options)?;
        let report = solution.to_solve_report(self.name())?;
        Ok(CombinatorialSolveReport::single(
            report,
            format!("{}#0", self.name()),
        ))
    }

    /// 以统一报告求解 MILP / Solve a MILP and return a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_milp_report_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_milp_combinatorial_report_with_options(model, options)
            .map(|aggregate| aggregate.report)
    }

    /// 以统一报告求解 LP / Solve an LP and return a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_lp_combinatorial_report_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = pre_cancelled_report(self.name(), &options)? {
            return Ok(report);
        }
        let result = self.solve_lp_with_options(model, options)?;
        let report = result.to_solve_report(self.name())?;
        Ok(CombinatorialSolveReport::single(
            report,
            format!("{}#0", self.name()),
        ))
    }

    /// 以统一报告求解 LP / Solve an LP and return a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_lp_report_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_lp_combinatorial_report_with_options(model, options)
            .map(|aggregate| aggregate.report)
    }

    /// 以统一报告求解 MetaModel 并转换 typed 结果 / Solve a MetaModel and convert its unified report to typed values.
    #[cfg(not(feature = "async"))]
    fn solve_report_typed_with_options<V>(
        &self,
        meta_model: &ospf_rust_core::model::MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<V>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_report_with_options(meta_model, options)
    }

    /// 统一入口：求解 MetaModel（默认 MILP 路径）/ Unified entry: solve MetaModel (MILP by default)
    #[cfg(not(feature = "async"))]
    fn solve<V>(&self, meta_model: &ospf_rust_core::model::MetaModel<V>) -> Result<FeasibleSolution>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 统一入口：求解 MetaModel（参数对象，默认 MILP 路径）/
    /// Unified entry: solve MetaModel with options object (MILP by default)
    #[cfg(not(feature = "async"))]
    fn solve_with_options<V>(
        &self,
        meta_model: &ospf_rust_core::model::MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let mechanism_model = meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        let mechanism_model = ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &mechanism_model,
            options.value_conversion_policy,
        )?;
        let triad_model = mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        let options = if options.name.is_some() {
            options
        } else {
            options.with_name(self.name())
        };
        self.solve_milp_with_options(&triad_model, options)
    }

    /// 统一入口：求解 MetaModel 并返回 typed 可行解 /
    /// Unified entry: solve MetaModel and return typed feasible solution
    #[cfg(not(feature = "async"))]
    fn solve_typed<V>(
        &self,
        meta_model: &ospf_rust_core::model::MetaModel<V>,
    ) -> Result<FeasibleSolutionV<V>>
    where
        Self: Sized,
        V: SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_typed_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 统一入口：求解 MetaModel 并返回 typed 可行解（参数对象）/
    /// Unified entry: solve MetaModel and return typed feasible solution (options object)
    #[cfg(not(feature = "async"))]
    fn solve_typed_with_options<V>(
        &self,
        meta_model: &ospf_rust_core::model::MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolutionV<V>>
    where
        Self: Sized,
        V: SolveValue + std::ops::Add<Output = V>,
    {
        let policy = options.value_conversion_policy;
        self.solve_with_options(meta_model, options)?
            .try_into_typed(policy)
    }

    /// 求解 MILP（参数对象，同步）/ Solve MILP with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_milp_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution>;

    /// 求解 LP（参数对象）/ Solve LP with options object
    #[cfg(feature = "async")]
    async fn solve_lp_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<LPResult>;

    /// 求解 LP（typed，参数对象）/ Solve LP with typed output (options object)
    #[cfg(feature = "async")]
    fn solve_lp_typed_with_options<'a, V>(
        &'a self,
        model: &'a ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LPResultV<V>>> + Send + 'a>>
    where
        Self: Sized,
        V: SolveValue,
    {
        let policy = options.value_conversion_policy;
        Box::pin(async move {
            self.solve_lp_with_options(model, options)
                .await?
                .try_into_typed(policy)
        })
    }

    /// 求解 LP（参数对象，同步）/ Solve LP with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_lp_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<LPResult>;

    /// 求解 LP（typed，参数对象，同步）/
    /// Solve LP with typed output and options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_lp_typed_with_options<V>(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<LPResultV<V>>
    where
        Self: Sized,
        V: SolveValue,
    {
        let policy = options.value_conversion_policy;
        self.solve_lp_with_options(model, options)?
            .try_into_typed(policy)
    }
}

/// 目标类别 / Objective Category
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ObjectiveCategory {
    /// 最小化 / Minimum
    #[default]
    Minimum,
    /// 最大化 / Maximum
    Maximum,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::error::{CoreError, SolverError};
    use ospf_rust_core::model::MetaModel;
    use ospf_rust_core::model::intermediate::LinearTriadModel;
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::{
        ConstraintRelation, MechanismModel, ModelBuildingStage, ModelBuildingStatusCallback,
    };
    use ospf_rust_core::solver::{
        AuditFingerprint, ProblemStatus, SolutionPresence, SolveFingerprints, SolveProof,
        SolveReport, SolveSolution, SolverOutput, SolverProvenance, SolverStatus,
        TerminationReason,
    };
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::variable::ContinuousVariableItem;
    #[cfg(not(feature = "async"))]
    use std::sync::{Arc, Mutex};

    struct MockColumnGenerationSolver;

    #[test]
    fn feasible_solution_from_output_maps_feasible_solution() {
        let output = SolverOutput::optimal(3.0, vec![1.0, 2.0]);
        let feasible = FeasibleSolution::from_output(&output).expect("should map feasible output");
        assert_eq!(feasible.obj, 3.0);
        assert_eq!(feasible.solution, vec![1.0, 2.0]);
    }

    #[test]
    fn feasible_solution_from_output_returns_none_without_solution_vector() {
        let output = SolverOutput::new(SolverStatus::Feasible).with_objective(1.0);
        let feasible = FeasibleSolution::from_output(&output);
        assert!(feasible.is_none());
    }

    #[test]
    fn feasible_solution_from_output_returns_none_without_objective_value() {
        let output = SolverOutput::new(SolverStatus::Feasible).with_solution(vec![1.0]);
        let feasible = FeasibleSolution::from_output(&output);
        assert!(feasible.is_none());
    }

    #[test]
    fn feasible_solution_try_from_output_reports_missing_solution_vector() {
        let output = SolverOutput::new(SolverStatus::Feasible).with_objective(1.0);
        let err = FeasibleSolution::try_from_output(&output, SolveValueConversionPolicy::Strict)
            .expect_err("missing solution vector should fail");
        assert!(err.to_string().contains("missing solution vector"));
    }

    #[test]
    fn feasible_solution_try_from_output_rejects_non_feasible_status() {
        let output = SolverOutput::new(SolverStatus::Infeasible);
        let err = FeasibleSolution::try_from_output(&output, SolveValueConversionPolicy::Strict)
            .expect_err("non-feasible status should fail");
        assert!(err.to_string().contains("not feasible"));
    }

    #[test]
    fn feasible_solution_report_projection_preserves_bound_and_incumbent() {
        let solution = FeasibleSolution {
            obj: 3.0,
            solution: vec![1.0, 2.0],
            time: Duration::from_millis(5),
            possible_best_obj: Some(2.0),
            gap: 1.0 / 3.0,
            benders_iterations: None,
            benders_runtime_metrics: None,
            source_report: None,
        };
        let report = solution
            .to_solve_report("fake-column-generation")
            .expect("legacy solution should project to a report");
        let projected = FeasibleSolution::try_from_report(&report)
            .expect("report should project back to legacy solution");

        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(projected.solution, vec![1.0, 2.0]);
        assert_eq!(projected.possible_best_obj, Some(2.0));
    }

    #[test]
    fn feasible_solution_report_projection_retains_native_metadata_and_dual() {
        let mut source_solution = SolveSolution::vector(vec![1.0, 2.0]);
        source_solution.objective = Some(3.0);
        source_solution.objective_value = Some(3.0);
        source_solution.dual_solution = Some(vec![4.0]);
        let model_fingerprint = AuditFingerprint {
            schema_version: "1.0".to_owned(),
            algorithm: "sha256".to_owned(),
            value: "source-model".to_owned(),
        };
        let source = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(source_solution)
            .proof(SolveProof::optimality())
            .provenance(SolverProvenance {
                solver_id: "native-solver".to_owned(),
                backend_name: "native-backend".to_owned(),
                ..SolverProvenance::default()
            })
            .fingerprints(SolveFingerprints {
                model: Some(model_fingerprint.clone()),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("source report should be valid");

        let projected = FeasibleSolution::try_from_report(&source)
            .expect("source report should project to a legacy solution")
            .to_solve_report("legacy-wrapper")
            .expect("legacy projection should retain the source report");

        assert_eq!(projected.provenance.solver_id, "native-solver");
        assert_eq!(projected.fingerprints.model, Some(model_fingerprint));
        assert_eq!(projected.proof, source.proof);
        assert_eq!(
            projected
                .solution
                .as_ref()
                .and_then(|solution| solution.dual_solution.clone()),
            Some(vec![4.0])
        );
    }

    #[test]
    fn benders_projection_does_not_promote_subproblem_optimality_proof() {
        let mut source_solution = SolveSolution::vector(vec![1.0]);
        source_solution.objective = Some(3.0);
        source_solution.objective_value = Some(3.0);
        source_solution.dual_solution = Some(vec![4.0]);
        let source = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(source_solution)
            .proof(SolveProof::optimality())
            .provenance(SolverProvenance {
                solver_id: "subproblem-solver".to_owned(),
                ..SolverProvenance::default()
            })
            .build()
            .expect("source report should be valid");

        let projected = FeasibleSolution::new(3.0, vec![1.0])
            .with_source_report(source)
            .with_benders_runtime_metrics(BendersRuntimeMetrics {
                executed_iterations: 1,
                best_solution_iteration: Some(1),
                total_cuts: 0,
                no_cut_iterations: 1,
                no_obj_improvement_iterations: 0,
                stop_reason: Some(BendersStopReason::IterationLimit),
                iteration_snapshots: Vec::new(),
            })
            .to_solve_report("benders-wrapper")
            .expect("Benders projection should remain a feasible incumbent");

        assert_eq!(projected.solution_presence, SolutionPresence::Incumbent);
        assert!(projected.proof.is_none());
        assert_eq!(
            projected.termination_reason,
            TerminationReason::IterationLimit
        );
        assert_eq!(projected.provenance.solver_id, "subproblem-solver");
        assert_eq!(
            projected
                .solution
                .as_ref()
                .and_then(|solution| solution.dual_solution.clone()),
            Some(vec![4.0])
        );
    }

    #[test]
    fn feasible_solution_report_recomputes_gap_from_bound() {
        let report = FeasibleSolution {
            obj: 3.0,
            solution: vec![1.0],
            time: Duration::ZERO,
            possible_best_obj: Some(2.0),
            gap: 0.0,
            benders_iterations: None,
            benders_runtime_metrics: None,
            source_report: None,
        }
        .to_solve_report("legacy-stale-gap")
        .expect("the report should derive a consistent gap from its bound");

        assert_eq!(report.statistics.relative_gap, Some(1.0 / 3.0));
        assert_eq!(report.trace.relative_gap, Some(1.0 / 3.0));
    }

    #[test]
    fn feasible_solution_without_bound_does_not_claim_zero_gap() {
        let report = FeasibleSolution::new(3.0, vec![1.0])
            .to_solve_report("legacy-no-bound")
            .expect("legacy feasible solution should remain representable");

        assert!(report.statistics.best_bound_value.is_none());
        assert!(report.statistics.absolute_gap.is_none());
        assert!(report.statistics.relative_gap.is_none());
        assert!(report.trace.global_lower_bound.is_none());
        assert!(report.trace.relative_gap.is_none());
    }

    #[test]
    fn feasible_solution_report_maps_benders_trace_and_stop_reason() {
        let solution = FeasibleSolution::new(7.0, vec![1.0]).with_benders_runtime_metrics(
            BendersRuntimeMetrics {
                executed_iterations: 4,
                best_solution_iteration: Some(3),
                total_cuts: 6,
                no_cut_iterations: 2,
                no_obj_improvement_iterations: 1,
                stop_reason: Some(BendersStopReason::CutStall),
                iteration_snapshots: Vec::new(),
            },
        );
        let report = solution
            .to_solve_report("benders")
            .expect("Benders solution should project to a report");

        assert_eq!(report.termination_reason, TerminationReason::StallNodeLimit);
        assert_eq!(report.trace.total_iterations, 4);
        assert_eq!(report.trace.generated_columns, 6);
        assert_eq!(
            report.diagnostics.extensions.get("benders.totalCuts"),
            Some(&"6".to_owned())
        );
    }

    #[test]
    fn combinatorial_report_selects_best_deterministically_and_keeps_failures() {
        let first = FeasibleSolution::new(3.0, vec![3.0])
            .to_solve_report("first")
            .expect("first report should build");
        let second = FeasibleSolution::new(1.0, vec![1.0])
            .to_solve_report("second")
            .expect("second report should build");
        let aggregate = aggregate_combinatorial_reports(
            "parallel",
            CombinatorialSelection::Best(ObjectiveCategory::Minimum),
            vec![
                (0, "first".to_owned(), Ok(first)),
                (
                    1,
                    "failed".to_owned(),
                    Err(ospf_rust_core::error::CoreError::Solver(
                        SolverError::SolveFailed("backend unavailable".to_owned()),
                    )),
                ),
                (2, "second".to_owned(), Ok(second)),
            ],
        )
        .expect("aggregate report should build");

        assert_eq!(
            aggregate.selected_attempt_id.as_deref(),
            Some(child_attempt_id(&aggregate_attempt_id("parallel"), "second", 2).as_str())
        );
        assert_eq!(aggregate.attempts.len(), 3);
        assert_eq!(
            aggregate.report.solution.as_ref().unwrap().objective,
            Some(1.0)
        );
        assert_eq!(aggregate.report.diagnostics.issues.len(), 1);
    }

    #[test]
    fn unverified_infeasible_report_does_not_stop_combinatorial_fallback() {
        let unverified =
            SolveReport::<f64>::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .build()
                .expect("unverified infeasible report should remain structurally valid");
        assert!(!report_is_selectable(&unverified));

        let feasible = FeasibleSolution::new(1.0, vec![1.0])
            .to_solve_report("fallback")
            .expect("fallback report should be valid");
        let aggregate = aggregate_combinatorial_reports(
            "unverified-infeasible",
            CombinatorialSelection::First,
            vec![
                (0, "unverified".to_owned(), Ok(unverified)),
                (1, "fallback".to_owned(), Ok(feasible)),
            ],
        )
        .expect("fallback aggregate should build");

        assert_eq!(aggregate.report.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            aggregate
                .selected_attempt_id
                .as_deref()
                .map(|id| id.contains("fallback")),
            Some(true)
        );
    }

    #[test]
    fn explicit_fallback_policy_controls_certified_terminal_reports() {
        let report =
            SolveReport::<f64>::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .proof(SolveProof::infeasibility())
                .build()
                .expect("verified infeasible report should be valid");

        assert!(
            CombinatorialFallbackPolicy::StopOnCertifiedTerminal.should_stop_on_report(&report)
        );
        assert!(!CombinatorialFallbackPolicy::ContinueOnTerminal.should_stop_on_report(&report));
        assert!(
            CombinatorialFallbackPolicy::StopOnCertifiedTerminal
                .should_stop_on_error(&CoreError::Solver(SolverError::Infeasible))
        );
        assert!(
            !CombinatorialFallbackPolicy::ContinueOnTerminal
                .should_stop_on_error(&CoreError::Solver(SolverError::Infeasible))
        );
        assert!(
            CombinatorialFallbackPolicy::ContinueOnTerminal.should_stop_on_error(
                &CoreError::Solver(SolverError::Cancelled("user".to_owned()))
            )
        );
    }

    #[test]
    fn aggregate_does_not_publish_unverified_terminal_when_all_attempts_fail_selection() {
        let unverified =
            SolveReport::<f64>::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .build()
                .expect("unverified infeasible report should remain structurally valid");

        let aggregate = aggregate_combinatorial_reports(
            "all-unverified",
            CombinatorialSelection::First,
            vec![(0, "unverified".to_owned(), Ok(unverified))],
        )
        .expect("aggregate should return a structured unknown report");

        assert_eq!(aggregate.report.problem_status, ProblemStatus::Unknown);
        assert_eq!(
            aggregate.report.termination_reason,
            TerminationReason::BackendFailure
        );
        assert!(!aggregate.report.has_incumbent());
        assert!(aggregate.selected_attempt_id.is_none());
        assert!(
            aggregate
                .report
                .diagnostics
                .issues
                .iter()
                .any(|issue| issue.code == "AttemptNotSelectable")
        );
    }

    #[test]
    fn first_completed_selection_keeps_parallel_linearization_point() {
        let late_lower_index = FeasibleSolution::new(10.0, vec![10.0])
            .to_solve_report("late")
            .expect("late report should be valid");
        let first_completed = FeasibleSolution::new(1.0, vec![1.0])
            .to_solve_report("first-completed")
            .expect("first completed report should be valid");

        let aggregate = aggregate_combinatorial_reports(
            "parallel-first",
            CombinatorialSelection::FirstCompleted(1),
            vec![
                (0, "late".to_owned(), Ok(late_lower_index)),
                (1, "first-completed".to_owned(), Ok(first_completed)),
            ],
        )
        .expect("parallel first aggregation should preserve the accepted worker");

        assert_eq!(
            aggregate.selected_attempt_id.as_deref(),
            Some(
                child_attempt_id(
                    &aggregate_attempt_id("parallel-first"),
                    "first-completed",
                    1,
                )
                .as_str(),
            )
        );
        assert_eq!(
            aggregate
                .report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective_value),
            Some(1.0)
        );
    }

    #[test]
    fn completed_backend_error_is_not_rewritten_by_late_loser_cancellation() {
        let handle = SolveHandle::new();
        handle.mark_completed();
        assert!(handle.cancel(ospf_rust_core::solver::CancellationOrigin::FrameworkLoser));

        let attempts = preserve_cancelled_attempts(
            vec![(
                0,
                "completed-error".to_owned(),
                Err(CoreError::Solver(SolverError::SolveFailed(
                    "backend failed before the late cancellation".to_owned(),
                ))),
            )],
            &[handle],
        );
        let aggregate = aggregate_combinatorial_reports_with_metadata(
            "late-cancellation",
            CombinatorialSelection::First,
            attempts,
        )
        .expect("a completed backend failure should remain an attempt failure");

        assert_eq!(
            aggregate.attempts[0].outcome,
            ospf_rust_core::solver::SolveAttemptOutcome::Failed
        );
        assert_eq!(
            aggregate.attempts[0]
                .error
                .as_ref()
                .map(|issue| issue.code.as_str()),
            Some("AttemptFailed")
        );
    }

    #[test]
    fn lp_report_keeps_native_optimality_proof_for_certificate_gate() {
        let mut solution = SolveSolution::vector(vec![1.0]);
        solution.objective = Some(2.0);
        solution.objective_value = Some(2.0);
        let core_report =
            SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
                .solution(solution)
                .proof(SolveProof::optimality())
                .build()
                .expect("native report should satisfy the certificate contract");
        let lp_report = LPResult::new(
            FeasibleSolution::new(2.0, vec![1.0]),
            LinearDualSolution::new(vec![4.0], Vec::new()),
        )
        .with_report(core_report)
        .to_solve_report("lp")
        .expect("LP report should retain the native proof");
        let certificate = ospf_rust_core::solver::require_optimal_lp_certificate(&lp_report)
            .expect("native LP proof should pass the certificate gate");

        assert_eq!(certificate.dual, &[4.0]);
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl ColumnGenerationSolver for MockColumnGenerationSolver {
        fn name(&self) -> &str {
            "mock_column_generation"
        }

        #[cfg(feature = "async")]
        async fn solve_milp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(
                model.num_variables() as f64,
                vec![0.0; model.num_variables()],
            ))
        }

        #[cfg(not(feature = "async"))]
        fn solve_milp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(
                model.num_variables() as f64,
                vec![0.0; model.num_variables()],
            ))
        }

        #[cfg(feature = "async")]
        async fn solve_lp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            Ok(LPResult::new(
                FeasibleSolution::new(
                    model.num_variables() as f64,
                    vec![0.0; model.num_variables()],
                ),
                LinearDualSolution::default(),
            ))
        }

        #[cfg(not(feature = "async"))]
        fn solve_lp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            Ok(LPResult::new(
                FeasibleSolution::new(
                    model.num_variables() as f64,
                    vec![0.0; model.num_variables()],
                ),
                LinearDualSolution::default(),
            ))
        }
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn solve_milp_with_options_solves_flattened_mechanism_model() {
        let solver = MockColumnGenerationSolver;
        let mechanism_model = MechanismModel::new("cg_mechanism");
        let stages = Arc::new(Mutex::new(Vec::new()));
        let stages_for_callback = stages.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            stages_for_callback.lock().unwrap().push(status.stage);
            Ok(())
        });
        let triad_model = mechanism_model
            .try_into_linear_triad_model_with_status_callback(Some(&callback))
            .expect("mechanism-model flatten should succeed");

        let options = FrameworkSolveOptions::new().with_name("cg_mechanism_case");
        let result = solver
            .solve_milp_with_options(&triad_model, options)
            .expect("mechanism-model milp solving should succeed");

        assert_eq!(result.obj, 0.0);
        let stages = stages.lock().unwrap();
        assert!(stages.contains(&ModelBuildingStage::FlattenLinearModel));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn solve_lp_with_options_solves_flattened_meta_model() {
        let solver = MockColumnGenerationSolver;
        let meta_model = MetaModel::<f64>::new("cg_meta");
        let stages = Arc::new(Mutex::new(Vec::new()));
        let stages_for_callback = stages.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            stages_for_callback.lock().unwrap().push(status.stage);
            Ok(())
        });
        let mechanism_model = meta_model
            .try_into_mechanism_model_with_status_callback(Some(&callback))
            .expect("meta-model build should succeed");
        let triad_model = mechanism_model
            .try_into_linear_triad_model_with_status_callback(Some(&callback))
            .expect("mechanism-model flatten should succeed");

        let options = FrameworkSolveOptions::new().with_name("cg_meta_case");
        let result = solver
            .solve_lp_with_options(&triad_model, options)
            .expect("meta-model lp solving should succeed");

        assert_eq!(result.result.obj, 0.0);
        let stages = stages.lock().unwrap();
        assert!(stages.contains(&ModelBuildingStage::RegisterTokens));
        assert!(stages.contains(&ModelBuildingStage::FlattenLinearModel));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn solve_report_with_options_returns_the_shared_report_contract() {
        let solver = MockColumnGenerationSolver;
        let meta_model = MetaModel::<f64>::new("cg_report_meta");
        let report = solver
            .solve_report_with_options(&meta_model, FrameworkSolveOptions::default())
            .expect("report-first MetaModel solve should succeed");

        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert!(report.has_incumbent());
        assert_eq!(report.incumbent(), None);
        assert_eq!(
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective),
            Some(0.0)
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn solve_with_options_shortcut_builds_then_delegates() {
        let solver = MockColumnGenerationSolver;
        let meta_model = MetaModel::<f64>::new("cg_meta_shortcut_options");
        let stages = Arc::new(Mutex::new(Vec::new()));
        let stages_for_callback = stages.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            stages_for_callback.lock().unwrap().push(status.stage);
            Ok(())
        });

        let options = FrameworkSolveOptions::new().with_building_callback(Some(callback));
        let result = solver
            .solve_with_options(&meta_model, options)
            .expect("meta-model options shortcut solving should succeed");

        assert_eq!(result.obj, 0.0);
        let stages = stages.lock().unwrap();
        assert!(stages.contains(&ModelBuildingStage::RegisterTokens));
        assert!(stages.contains(&ModelBuildingStage::FlattenLinearModel));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn solve_with_options_shortcut_rejects_non_finite_value_in_strict_mode() {
        let solver = MockColumnGenerationSolver;
        let mut meta_model = MetaModel::<f64>::new("cg_meta_non_finite");
        let x = ContinuousVariableItem::auto("cg_meta_non_finite_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, f64::NAN)],
                ConstraintRelation::LessEqual,
                1.0,
                "cg_meta_non_finite_c",
            )
            .unwrap();

        let options = FrameworkSolveOptions::new().with_value_conversion_policy(
            ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
        );
        let error = solver
            .solve_with_options(&meta_model, options)
            .expect_err("strict mode should reject non-finite conversion");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::NonFinite(_))
        ));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn solve_typed_with_options_supports_f64_output() {
        let solver = MockColumnGenerationSolver;
        let meta_model = MetaModel::<f64>::new("cg_meta_typed_f64");
        let options = FrameworkSolveOptions::new()
            .with_value_conversion_policy(SolveValueConversionPolicy::AllowRounding);
        let result = solver
            .solve_typed_with_options(&meta_model, options)
            .expect("typed solve should succeed");
        assert_eq!(result.obj, 0.0_f64);
        assert_eq!(result.solution.len(), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn solve_lp_typed_with_options_supports_f64_output() {
        let solver = MockColumnGenerationSolver;
        let meta_model = MetaModel::<f64>::new("cg_lp_typed_f64");
        let triad_model = meta_model
            .try_into_mechanism_model_with_status_callback(None)
            .and_then(|model| model.try_into_linear_triad_model_with_status_callback(None))
            .expect("meta-model flatten to triad should succeed");
        let options = FrameworkSolveOptions::new()
            .with_value_conversion_policy(SolveValueConversionPolicy::AllowRounding);
        let result = solver
            .solve_lp_typed_with_options::<f64>(&triad_model, options)
            .expect("typed lp solve should succeed");
        assert_eq!(result.result.obj, 0.0_f64);
        assert_eq!(result.result.solution.len(), 0);
    }

    #[cfg(all(feature = "nightly", not(feature = "async")))]
    #[test]
    fn solve_fn_shortcut_supports_meta_model_call() {
        let solver = MockColumnGenerationSolver;
        let meta_model = MetaModel::<f64>::new("cg_meta_fn");
        let callable = solver.as_fn();
        let result = callable(&meta_model).expect("nightly callable shortcut should succeed");
        assert_eq!(result.obj, 0.0);
    }

    #[cfg(feature = "async")]
    fn assert_send<T: Send>(_: &T) {}

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn solve_with_options_returns_send_future() {
        let solver = MockColumnGenerationSolver;
        let meta_model = MetaModel::<f64>::new("cg_meta_async_send_milp");
        let options = FrameworkSolveOptions::new().with_name("cg_meta_async_send_milp_case");

        let future = solver.solve_with_options(&meta_model, options);
        assert_send(&future);

        let result = future
            .await
            .expect("meta-model milp async solving should succeed");
        assert_eq!(result.obj, 0.0);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn solve_lp_with_options_returns_send_future() {
        let solver = MockColumnGenerationSolver;
        let meta_model = MetaModel::<f64>::new("cg_meta_async_send_lp");
        let options = FrameworkSolveOptions::new().with_name("cg_meta_async_send_lp_case");
        let triad_model = meta_model
            .try_into_mechanism_model_with_status_callback(None)
            .and_then(|model| model.try_into_linear_triad_model_with_status_callback(None))
            .expect("meta-model flatten to triad should succeed");

        let future = solver.solve_lp_with_options(&triad_model, options);
        assert_send(&future);

        let result = future
            .await
            .expect("meta-model lp async solving should succeed");
        assert_eq!(result.result.obj, 0.0);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn solve_shortcut_returns_send_future() {
        let solver = MockColumnGenerationSolver;
        let meta_model = MetaModel::<f64>::new("cg_meta_async_shortcut");

        let future = solver.solve(&meta_model);
        assert_send(&future);

        let result = future
            .await
            .expect("meta-model shortcut async solving should succeed");
        assert_eq!(result.obj, 0.0);
    }
}
