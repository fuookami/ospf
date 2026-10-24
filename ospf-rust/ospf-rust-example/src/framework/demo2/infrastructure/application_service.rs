//! 三条航空业务路径的共享编排，不维护另一套分解模型。 / Shared orchestration for three aviation paths, without a second decomposition model.

use std::time::{Duration, Instant};
use ospf_rust_core::error::{CoreError, Result, SolverErrorClass};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{LinearSolver, ProblemStatus, SolveFingerprints, SolveHandle, SolveOptions, SolveReport, TerminationReason,
    linear_model_fingerprint, linear_model_mapping, solvers::GurobiSolver};
use ospf_rust_framework::solver::FixedRecourseConfig;
use ospf_rust_framework_bpp::application::{DiscreteLoadingApplicationService, DiscreteLoadingBendersApplicationService,
    DiscreteLoadingBendersOptions, DiscreteLoadingCertificationService, DiscreteLoadingSolveOptions, DiscreteLoadingSolveResult,
    PartitionColumnGenerationApplicationService, RegisteredDiscreteLoading};
use ospf_rust_quantities::unit::derived::Kilogram;
use super::dto::{AircraftTypeInput, Demo2Request, Demo2Response, RunningHeartBeatDto, SourceMilpFinishingPolicy};
use crate::framework::demo2::domain::{SolveMode, append_core_feasibility_diagnostics,
    append_decomposition_quality_notes, demo2_response, no_solution_response, push_solver_path_note, resolve_solve_mode_for_size};
use crate::framework::demo2::domain::shared::loading_context::{Demo2LoadingContext, invalid};
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

/// 统一完整模型工厂、求解策略及结果提取。 / Unified complete-model factory, solve strategy and result extraction.
pub struct Demo2ApplicationService;

/// 执行回调与最终渲染控制；回调不进入可恢复后端错误边界。 / Execution callbacks and final render control; callbacks remain outside recoverable backend-error boundaries.
pub struct Demo2ExecutionOptions<'a> {
    /// 外部取消句柄。 / External cancellation handle.
    pub handle: &'a SolveHandle,
    /// 是否生成最终渲染。 / Whether to produce final rendering.
    pub with_render: bool,
    /// 阶段心跳，错误直接传播。 / Phase heartbeat; errors propagate directly.
    pub running: Option<&'a dyn Fn(&RunningHeartBeatDto) -> Result<()>>,
    /// 最终响应回调，包括无解终态。 / Final-response callback, including no-solution terminals.
    pub finished: Option<&'a dyn Fn(&Demo2Response) -> Result<()>>,
}

impl Demo2ApplicationService {
    /// 带回调与渲染选项的原生执行入口。 / Native execution entry with callback and render options.
    pub fn solve_with_execution_options(
        request: Demo2Request,
        mode: Demo2PipelineMode,
        options: Demo2ExecutionOptions<'_>,
    ) -> Result<Demo2Response> {
        Self::solve_with_options(request, mode, options, |model, remaining, handle| {
            GurobiSolver::new().solve_linear_report_with_options(model, &SolveOptions {
                time_limit: Some(remaining), cancellation_handle: Some(handle), ..Default::default()
            })
        })
    }

    /// 使用 Gurobi，全部调用共用取消句柄及剩余时限。 / Use Gurobi with a shared cancellation handle and remaining deadline across calls.
    pub fn solve(
        request: Demo2Request,
        mode: Demo2PipelineMode,
        handle: &SolveHandle,
    ) -> Result<Demo2Response> {
        Self::solve_with(request, mode, handle, |model, remaining, handle| {
            GurobiSolver::new().solve_linear_report_with_options(model, &SolveOptions {
                time_limit: Some(remaining), cancellation_handle: Some(handle), ..Default::default()
            })
        })
    }

    /// 可注入后端的完整入口；回调错误原样传播，不触发错误类别不明的回退。
    /// Complete backend-injectable entry; callback errors propagate without unclassified fallback.
    pub fn solve_with<S>(
        request: Demo2Request,
        mode: Demo2PipelineMode,
        handle: &SolveHandle,
        mut solve: S,
    ) -> Result<Demo2Response>
    where
        S: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
    {
        Self::solve_with_options(request, mode, Demo2ExecutionOptions { handle, with_render: true, running: None, finished: None }, &mut solve)
    }

    /// 使用真实最终源解调用完成通知及渲染，不在 fallback 内重复触发通知。
    /// Notify completion and render from the actual final source solution, never repeating notifications inside fallback.
    pub fn solve_with_options<S>(
        mut request: Demo2Request,
        mode: Demo2PipelineMode,
        options: Demo2ExecutionOptions<'_>,
        solve: S,
    ) -> Result<Demo2Response>
    where
        S: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
    {
        let started = Instant::now();
        if let Some(running) = options.running { running(&RunningHeartBeatDto { task_id: "demo2".into(), progress: 0.0, status: "Building".into() })?; }
        request.loading.time_limit = request.loading.time_limit.saturating_sub(started.elapsed());
        let mut response = Self::solve_internal(request, mode, options.handle, solve)?;
        if !options.with_render { response.render = None; }
        if let Some(running) = options.running { running(&RunningHeartBeatDto { task_id: "demo2".into(), progress: 1.0, status: response.status.clone() })?; }
        if let Some(finished) = options.finished { finished(&response)?; }
        Ok(response)
    }

    fn solve_internal<S>(
        request: Demo2Request,
        mode: Demo2PipelineMode,
        handle: &SolveHandle,
        mut solve: S,
    ) -> Result<Demo2Response>
    where
        S: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
    {
        let started = Instant::now();
        let finishing = request.solve_policy.source_milp_finishing == SourceMilpFinishingPolicy::IfUnproven;
        if finishing && (!request.loading.acceleration_time_ratio.is_finite() || request.loading.acceleration_time_ratio <= 0.0
            || request.loading.acceleration_time_ratio >= 1.0) { return Err(invalid("acceleration time ratio must be strictly between zero and one")); }
        if request.loading.partition_column_generation && request.solve_policy.prefer_benders {
            return Err(invalid("partition CG and preferred Benders are mutually exclusive"));
        }
        if matches!(request.aircraft_type, AircraftTypeInput::Unknown) {
            return Ok(no_solution_response("UnsupportedAircraft", vec!["unsupported aircraft type".into()]));
        }
        if request.solve_policy.prefer_benders && (request.benders_adaptive.max_iterations == 0
            || !request.benders_adaptive.tolerance.is_finite() || request.benders_adaptive.tolerance <= 0.0
            || request.benders_adaptive.tolerance >= 0.5)
        { return Err(invalid("invalid Benders iterations or tolerance; input errors cannot fall back")); }
        if handle.is_cancelled() || request.loading.time_limit.is_zero() {
            let reason = if handle.is_cancelled() { TerminationReason::Cancelled } else { TerminationReason::TimeLimit };
            return Self::response(&request, DiscreteLoadingSolveResult { report: SolveReport::builder(ProblemStatus::Unknown, reason).build()?,
                solution: None, objective: None, registration: None }, "not_started", vec![]);
        }
        let request = Demo2LoadingContext::normalize(request)?;
        let registration = Demo2LoadingContext::register(&request, mode)?;
        let sparse_assignments = registration.context().aggregation().assignments.len();
        let mut notes = vec![format!("benders_problem_size_binary_variables={sparse_assignments}"),
            format!("source_milp_finishing_policy={:?}", request.solve_policy.source_milp_finishing)];
        if request.loading.partition_column_generation {
            let options = Demo2LoadingContext::partitions(&request, &registration)?;
            let config = request.loading.column_generation;
            let attempt = PartitionColumnGenerationApplicationService::solve(&registration, options,
                ospf_rust_framework_bpp::application::PartitionColumnGenerationConfig {
                    time_limit: config.time_limit.min(Self::acceleration_budget(&request, started)), ..config
                }, handle, &mut solve);
            let (result, reports) = match attempt {
                Ok(result) => {
                    if let Some(error) = &result.acceleration_error { notes.push(format!("cg_backend_error={error}")); }
                    notes.push(format!("cg_lp_converged={},root_columns={},completion_columns={}", result.lp_converged,
                        result.root_column_count, result.completion_column_count));
                    let reports = result.refinement_report.into_iter().chain(result.final_report).collect();
                    (DiscreteLoadingSolveResult { report: result.report, solution: result.solution,
                        objective: result.objective, registration: Some(registration) }, reports)
                }
                Err(error) if finishing && Self::recoverable(&error) => {
                    notes.push(format!("cg_error={error}"));
                    (Self::uncertified(registration)?, Vec::new())
                }
                Err(error) => return Err(error),
            };
            return Self::finish(&request, mode, result, "partition_column_generation", notes, reports, started, handle, &mut solve);
        }
        if let SolveMode::Benders(adaptive) = resolve_solve_mode_for_size(&request, sparse_assignments, &mut notes) {
            let mut first = Some(registration);
            let attempt = DiscreteLoadingBendersApplicationService::solve(|| first.take().map_or_else(
                || Demo2LoadingContext::register(&request, mode), Ok), DiscreteLoadingBendersOptions {
                config: FixedRecourseConfig { iteration_limit: adaptive.max_iterations,
                    time_limit: Self::acceleration_budget(&request, started), tolerance: adaptive.tolerance, ..Default::default() },
                fallback_to_milp: request.solve_policy.benders_fallback_to_milp, ..Default::default()
            }, handle, &mut solve);
            let result = match attempt {
                Ok(result) => result,
                Err(error) if finishing && Self::recoverable(&error) => {
                    notes.push(format!("benders_error={error}"));
                    let result = Self::uncertified(Demo2LoadingContext::register(&request, mode)?)?;
                    return Self::finish(&request, mode, result, "benders", notes, Vec::new(), started, handle, &mut solve);
                }
                Err(error) => return Err(error),
            };
            let path = if result.fallback_used { "milp_fallback" } else { "benders" };
            push_solver_path_note(&mut notes, path);
            if let Some(decomposition) = &result.decomposition {
                if let Some(error) = &decomposition.backend_error { notes.push(format!("benders_backend_error={error}")); }
                append_decomposition_quality_notes(&request, decomposition, started.elapsed().as_millis(), &mut notes);
                notes.push(format!("benders_stop={:?},iterations={},cuts={}", decomposition.stop,
                    decomposition.iterations, decomposition.feasibility_cuts + decomposition.optimality_cuts));
            }
            if let Some(error) = &result.decomposition_error { notes.push(format!("benders_error={error}")); }
            if let Some(report) = &result.fallback_report { notes.push(format!("fallback_status={:?},termination={:?}", report.problem_status, report.termination_reason)); }
            let mut reports = Vec::new();
            if let Some(decomposition) = result.decomposition {
                reports.extend(decomposition.master_report);
                reports.extend(decomposition.subproblem_report);
            }
            reports.extend(result.fallback_report);
            let mut source = result.result;
            if source.registration.is_none() {
                if handle.is_cancelled() || request.loading.time_limit.saturating_sub(started.elapsed()).is_zero()
                    || matches!(source.report.termination_reason, TerminationReason::Cancelled | TerminationReason::Interrupted) {
                    let mut response = Self::response(&request, source, path, notes)?;
                    response.algorithm_reports = reports;
                    return Ok(response);
                }
                let registration = first.take().map_or_else(|| Demo2LoadingContext::register(&request, mode), Ok)?;
                source = Self::uncertified(registration)?;
            }
            return Self::finish(&request, mode, source, path, notes, reports, started, handle, &mut solve);
        }
        if request.solve_policy.prefer_benders { notes.push(format!("Benders skipped: sparse assignments {sparse_assignments} below threshold {}", request.benders_adaptive.min_binary_variables)); }
        push_solver_path_note(&mut notes, "milp_direct");
        let result = DiscreteLoadingApplicationService::solve(|| Ok(registration), DiscreteLoadingSolveOptions {
            time_limit: request.loading.time_limit.saturating_sub(started.elapsed()), ..Default::default()
        }, handle, solve)?;
        Self::response(&request, result, "milp_direct", notes)
    }

    fn recoverable(error: &CoreError) -> bool {
        matches!(error.solver_error_class(), SolverErrorClass::Unsupported | SolverErrorClass::Environment
            | SolverErrorClass::License | SolverErrorClass::Backend | SolverErrorClass::Numerical)
    }

    fn acceleration_budget(request: &Demo2Request, started: Instant) -> Duration {
        let remaining = request.loading.time_limit.saturating_sub(started.elapsed());
        match request.solve_policy.source_milp_finishing {
            SourceMilpFinishingPolicy::Disabled => remaining,
            SourceMilpFinishingPolicy::IfUnproven => remaining.mul_f64(request.loading.acceleration_time_ratio),
        }
    }

    fn uncertified(registration: RegisteredDiscreteLoading<f64, Kilogram>) -> Result<DiscreteLoadingSolveResult<Kilogram>> {
        let compiled = registration.model().linear_model_snapshot()?.compile_flt64()?;
        let report = SolveReport::builder(ProblemStatus::Unknown, TerminationReason::Suboptimal)
            .model_mapping(Some(linear_model_mapping(&compiled.model)?))
            .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(&compiled.model)?), ..Default::default() }).build()?;
        Ok(DiscreteLoadingSolveResult { report, registration: Some(registration), solution: None, objective: None })
    }

    fn finish<S>(
        request: &Demo2Request,
        mode: Demo2PipelineMode,
        mut result: DiscreteLoadingSolveResult<Kilogram>,
        path: &str,
        mut notes: Vec<String>,
        mut reports: Vec<SolveReport<f64>>,
        started: Instant,
        handle: &SolveHandle,
        solve: &mut S,
    ) -> Result<Demo2Response>
    where
        S: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
    {
        if request.solve_policy.source_milp_finishing == SourceMilpFinishingPolicy::Disabled {
            DiscreteLoadingCertificationService::validate(&mut result, DiscreteLoadingSolveOptions::default().tolerance)?;
            notes.push("source_milp_finishing=disabled;source_validation=retained".into());
            push_solver_path_note(&mut notes, path);
            let mut response = Self::response(request, result, path, notes)?;
            response.algorithm_reports = reports;
            return Ok(response);
        }
        reports.push(result.report.clone());
        let certified = DiscreteLoadingCertificationService::solve(result, || Demo2LoadingContext::register(request, mode),
            DiscreteLoadingSolveOptions { time_limit: request.loading.time_limit.saturating_sub(started.elapsed()), ..Default::default() }, handle, solve)?;
        let certified_path = if let Some(report) = certified.certification_report {
            notes.push(format!("source_certification_status={:?},termination={:?}", report.problem_status, report.termination_reason));
            reports.push(report);
            format!("{path}_source_milp")
        } else { path.into() };
        if let Some(error) = certified.certification_error { notes.push(format!("source_certification_error={error}")); }
        push_solver_path_note(&mut notes, &certified_path);
        let mut response = Self::response(request, certified.result, &certified_path, notes)?;
        response.algorithm_reports = reports;
        Ok(response)
    }

    fn response(
        request: &Demo2Request,
        result: DiscreteLoadingSolveResult<Kilogram>,
        path: &str,
        mut notes: Vec<String>,
    ) -> Result<Demo2Response> {
        notes.push(format!("termination={:?}", result.report.termination_reason));
        let status = if result.report.is_optimal() { "Optimal".into() }
            else if result.solution.is_some() { "Feasible".into() }
            else if result.report.problem_status != ProblemStatus::Unknown { format!("{:?}", result.report.problem_status) }
            else { match result.report.termination_reason {
                TerminationReason::Cancelled => "Cancelled".into(), TerminationReason::TimeLimit => "TimeLimit".into(),
                TerminationReason::IterationLimit => "IterationLimit".into(), _ => "Unknown".into(),
            } };
        if result.solution.is_none() { append_core_feasibility_diagnostics(request, &mut notes); }
        let mut response = demo2_response(status, result.objective, vec![], notes);
        response.solve_report = Some(result.report.clone());
        response.framework_path = path.into();
        if let (Some(solution), Some(registration), Some(incumbent)) = (result.solution, result.registration, result.report.solution) {
            crate::framework::demo2::domain::shared::loading_solution::enrich(request, &registration, &solution, &incumbent.values, &mut response)?;
        }
        Ok(response)
    }
}
