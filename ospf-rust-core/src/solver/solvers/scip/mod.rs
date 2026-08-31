//! SCIP 求解器接口
//! SCIP Solver Interface

//! 此模块仅在启用 `scip` feature 时可用。
//! This module is only available when the `scip` feature is enabled.

use crate::error::{CoreError, Result, SolverError};
use crate::model::ConstraintRelation;
use crate::model::ObjectiveCategory;
use crate::model::intermediate::{LinearTriadModel, QuadraticTetradModel};
use crate::solver::solver::{emit_progress_start, emit_progress_terminal};
use crate::solver::{
    CapabilitySupport, LinearSolver, QuadraticSolver, SolveHandle, SolveOptions, SolveReport,
    SolveWarning, SolverCapabilities, SolverCapability, SolverDescriptor, SolverInfo, SolverOutput,
    SolverProvenance, SolverStatus, SolvingStatus, attach_linear_solution_audit,
    attach_quadratic_model_mapping, cancelled_solve_report, linear_dual_certificate_is_valid,
    solver_output_to_report_with_cancellation, solver_output_to_report_with_provenance,
    validate_per_solve_limits, verify_native_terminal_proof,
};
use crate::solver::{
    configuration_fingerprint, linear_model_fingerprint, quadratic_model_fingerprint,
    solver_provenance_fingerprint,
};
use crate::token::Token;
use crate::variable::VariableType;
#[cfg(feature = "scip")]
use russcip::{
    Constraint, Event, EventMask, Eventhdlr, Model, ParamSetting, ProblemCreated, ProblemOrSolving,
    Retcode, SCIPEventhdlr, Solving, Status as SCIPStatus, Unsolved, VarType, Variable,
    WithSolutions, WithSolvingStats,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod callbacks;
mod config;

pub use callbacks::{
    SCIPNativeCallback, SCIPNativeControl, SCIPNativeObserver, SCIPNativeSnapshot, SCIPNativeWhere,
    SCIPSnapshotControl, SCIPSnapshotObserver, SCIPStage, SCIPStageCallback, SCIPStageStatus,
    SCIPTelemetryCallback, SCIPTelemetryStatus, ScipNativeCallback, ScipNativeControl,
    ScipNativeObserver, ScipNativeSnapshot, ScipNativeWhere, ScipSnapshotControl,
    ScipSnapshotObserver, ScipStage, ScipStageCallback, ScipStageStatus, ScipTelemetryCallback,
    ScipTelemetryStatus,
};
pub use config::{PresolvingMode, SCIPConfig, ScipConfig};

#[cfg(feature = "scip")]
#[derive(Clone)]
struct SCIPTelemetryRuntime {
    objective_category: ObjectiveCategory,
    scip_vars: Vec<Variable>,
    no_improvement_time_limit: Option<f64>,
    interruptible_time: Option<f64>,
    interruptible_gap: Option<f64>,
    improvement_tolerance: f64,
    telemetry_min_interval: Option<f64>,
    native_event_mask: EventMask,
    telemetry_callback: Option<SCIPTelemetryCallback>,
    snapshot_observers: Vec<SCIPSnapshotObserver>,
    native_callback: Option<SCIPNativeCallback>,
    native_observers: Vec<SCIPNativeObserver>,
    cancellation_handle: Option<SolveHandle>,
    max_iterations: Option<i64>,
    iteration_limit_reached: Arc<AtomicBool>,
    callback_error: Arc<Mutex<Option<CoreError>>>,
}

#[cfg(feature = "scip")]
impl SCIPTelemetryRuntime {
    fn enabled(&self) -> bool {
        self.no_improvement_time_limit.is_some()
            || self.telemetry_callback.is_some()
            || !self.snapshot_observers.is_empty()
            || self.native_callback.is_some()
            || !self.native_observers.is_empty()
            || self.cancellation_handle.is_some()
            || self.max_iterations.is_some()
    }
}

#[cfg(feature = "scip")]
struct SCIPTelemetryEventHandler {
    runtime: SCIPTelemetryRuntime,
    solve_started_at: Instant,
    last_improvement: Instant,
    last_telemetry_emit: Option<Instant>,
    best_objective: Option<f64>,
    initial_objective_value: Option<f64>,
}

#[cfg(feature = "scip")]
impl SCIPTelemetryEventHandler {
    fn new(runtime: SCIPTelemetryRuntime) -> Self {
        let now = Instant::now();
        Self {
            runtime,
            solve_started_at: now,
            last_improvement: now,
            last_telemetry_emit: None,
            best_objective: None,
            initial_objective_value: None,
        }
    }

    fn classify_native_where(event_mask: EventMask) -> SCIPNativeWhere {
        if event_mask.matches(EventMask::NODE_EVENT) {
            return SCIPNativeWhere::Node;
        }
        if event_mask.matches(EventMask::LP_EVENT) {
            return SCIPNativeWhere::Lp;
        }
        if event_mask.matches(EventMask::SOL_EVENT) {
            return SCIPNativeWhere::Solution;
        }
        if event_mask.matches(EventMask::VAR_EVENT) {
            return SCIPNativeWhere::Variable;
        }
        if event_mask.matches(EventMask::ROW_EVENT) {
            return SCIPNativeWhere::Row;
        }
        if event_mask.matches(EventMask::PRESOLVE_ROUND) {
            return SCIPNativeWhere::PresolveRound;
        }
        if event_mask.matches(EventMask::SYNC) {
            return SCIPNativeWhere::Sync;
        }
        SCIPNativeWhere::Other
    }

    fn build_snapshot(&mut self, model: &Model<Solving>) -> SCIPTelemetryStatus {
        let best_solution = model.best_sol();
        let objective_value = best_solution
            .as_ref()
            .map(|sol| sol.obj_val())
            .filter(|v| v.is_finite());
        if self.initial_objective_value.is_none() {
            self.initial_objective_value = objective_value;
        }
        let incumbent_solution = best_solution.as_ref().map(|solution| {
            self.runtime
                .scip_vars
                .iter()
                .map(|var| solution.val(var))
                .collect()
        });
        let best_bound = {
            let bound = model.best_bound();
            if bound.is_finite() { Some(bound) } else { None }
        };
        let mip_gap = objective_value
            .zip(best_bound)
            .and_then(|(obj, bound)| SCIPSolver::relative_gap(obj, bound));
        SCIPTelemetryStatus {
            solve_time: self.solve_started_at.elapsed(),
            objective_category: self.runtime.objective_category,
            initial_objective_value: self.initial_objective_value,
            objective_value,
            incumbent_solution,
            best_bound,
            mip_gap,
            iterations: Some(model.n_lp_iterations()),
            node_count: Some(model.n_nodes()),
        }
    }

    fn should_emit_telemetry(&self) -> bool {
        match self.runtime.telemetry_min_interval {
            None => true,
            Some(min_interval_seconds) => self
                .last_telemetry_emit
                .map(|last| last.elapsed().as_secs_f64() >= min_interval_seconds)
                .unwrap_or(true),
        }
    }

    fn check_improvement(&mut self, objective_value: Option<f64>) {
        let Some(current) = objective_value else {
            return;
        };
        let improved = match self.best_objective {
            None => true,
            Some(previous) => match self.runtime.objective_category {
                ObjectiveCategory::Minimum => {
                    current < previous - self.runtime.improvement_tolerance
                }
                ObjectiveCategory::Maximum => {
                    current > previous + self.runtime.improvement_tolerance
                }
            },
        };
        if improved {
            self.best_objective = Some(current);
            self.last_improvement = Instant::now();
        }
    }
}

#[cfg(feature = "scip")]
impl Eventhdlr for SCIPTelemetryEventHandler {
    fn get_type(&self) -> EventMask {
        self.runtime.native_event_mask
    }

    fn execute(&mut self, model: Model<Solving>, eventhdlr: SCIPEventhdlr, event: Event) {
        let snapshot = self.build_snapshot(&model);
        let event_type = event.event_type();
        let event_mask_bits: u64 = event_type.into();
        let native_snapshot = SCIPNativeSnapshot {
            where_point: Self::classify_native_where(event_type),
            event_mask_bits,
            handler_name: eventhdlr.name(),
            solve_time: snapshot.solve_time,
            objective_category: snapshot.objective_category,
            initial_objective_value: snapshot.initial_objective_value,
            objective_value: snapshot.objective_value,
            incumbent_solution: snapshot.incumbent_solution.clone(),
            best_bound: snapshot.best_bound,
            mip_gap: snapshot.mip_gap,
            iterations: snapshot.iterations,
            node_count: snapshot.node_count,
        };
        self.check_improvement(snapshot.objective_value);

        let iteration_limit_reached = if let Some(limit) = self.runtime.max_iterations
            && let Some(iterations) = snapshot.iterations
            && u128::try_from(limit).ok().is_some_and(|limit| {
                u128::try_from(iterations)
                    .ok()
                    .is_some_and(|iterations| iterations >= limit)
            }) {
            self.runtime
                .iteration_limit_reached
                .store(true, Ordering::SeqCst);
            true
        } else {
            false
        };

        if let Some(callback) = self.runtime.telemetry_callback.as_ref()
            && self.should_emit_telemetry()
        {
            if let Err(error) = callback(&snapshot) {
                let mut callback_error = self
                    .runtime
                    .callback_error
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if callback_error.is_none() {
                    *callback_error = Some(CoreError::callback_error(format!(
                        "SCIP telemetry callback error: {}",
                        error
                    )));
                }
                unsafe {
                    russcip::ffi::SCIPinterruptSolve(model.inner());
                }
                return;
            }
            self.last_telemetry_emit = Some(Instant::now());
        }

        let mut request_interrupt = iteration_limit_reached
            || self
                .runtime
                .cancellation_handle
                .as_ref()
                .is_some_and(SolveHandle::is_cancelled);
        for observer in &self.runtime.snapshot_observers {
            match observer(&snapshot) {
                Ok(control) => {
                    if matches!(control, SCIPSnapshotControl::Interrupt) {
                        request_interrupt = true;
                    }
                }
                Err(error) => {
                    let mut callback_error = self
                        .runtime
                        .callback_error
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if callback_error.is_none() {
                        *callback_error = Some(CoreError::callback_error(format!(
                            "SCIP snapshot observer error: {}",
                            error
                        )));
                    }
                    request_interrupt = true;
                }
            }
        }
        for observer in &self.runtime.native_observers {
            match observer(&native_snapshot) {
                Ok(control) => {
                    if matches!(control, SCIPNativeControl::Interrupt) {
                        request_interrupt = true;
                    }
                }
                Err(error) => {
                    let mut callback_error = self
                        .runtime
                        .callback_error
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if callback_error.is_none() {
                        *callback_error = Some(CoreError::callback_error(format!(
                            "SCIP native observer error: {}",
                            error
                        )));
                    }
                    request_interrupt = true;
                }
            }
        }
        if let Some(callback) = self.runtime.native_callback.as_ref() {
            match callback(&native_snapshot) {
                Ok(control) => {
                    if matches!(control, SCIPNativeControl::Interrupt) {
                        request_interrupt = true;
                    }
                }
                Err(error) => {
                    let mut callback_error = self
                        .runtime
                        .callback_error
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if callback_error.is_none() {
                        *callback_error = Some(CoreError::callback_error(format!(
                            "SCIP native callback error: {}",
                            error
                        )));
                    }
                    request_interrupt = true;
                }
            }
        }

        if let Some(limit_seconds) = self.runtime.no_improvement_time_limit {
            let time_reached = self
                .runtime
                .interruptible_time
                .map(|minimum| snapshot.solve_time.as_secs_f64() >= minimum)
                .unwrap_or(true);
            let gap_reached = self
                .runtime
                .interruptible_gap
                .map(|threshold| {
                    snapshot
                        .objective_value
                        .zip(snapshot.best_bound)
                        .is_some_and(|(objective_value, best_bound)| {
                            (objective_value - best_bound).abs() < threshold
                        })
                })
                .unwrap_or(true);
            if self.last_improvement.elapsed().as_secs_f64() >= limit_seconds
                && time_reached
                && gap_reached
            {
                request_interrupt = true;
            }
        }

        if request_interrupt {
            unsafe {
                russcip::ffi::SCIPinterruptSolve(model.inner());
            }
        }
    }
}

/// SCIP 求解器 / SCIP Solver
///
/// 封装 SCIP 优化器的求解器实现。
/// Solver implementation wrapping SCIP optimizer.
///
/// # 特性 / Features
///
/// - 支持 LP、MIP、QP（QP 支持依赖 SCIP 插件与外部求解器）
/// - Supports LP, MIP, and QP (QP support depends on SCIP plugins and external solvers)
/// - 开源免费 / Open-source and free
/// - 支持约束整数规划 / Supports constraint integer programming
///
/// # 前提条件 / Prerequisites
///
/// - 需要启用 `scip` feature / Requires enabling the `scip` feature
///
/// # 示例 / Examples
///
/// ```rust,ignore
/// use ospf_rust_core::solver::solvers::SCIPSolver;
///
/// let solver = SCIPSolver::new();
/// let result = solver.solve_linear(&model)?;
///
/// if let Some(solution) = result.solution {
///     println!("Optimal solution: {:?}", solution);
/// }
/// ```
#[derive(Debug)]
#[cfg(feature = "scip")]
pub struct SCIPSolver {
    /// 配置 / Configuration
    config: SCIPConfig,
}

#[cfg(feature = "scip")]
impl Default for SCIPSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "scip")]
impl SCIPSolver {
    /// 创建新的 SCIP 求解器 / Create new SCIP solver
    pub fn new() -> Self {
        Self {
            config: SCIPConfig::default(),
        }
    }

    /// 使用配置创建求解器 / Create solver with configuration
    pub fn with_config(config: SCIPConfig) -> Self {
        Self { config }
    }

    /// 获取配置引用 / Get configuration reference
    pub fn config(&self) -> &SCIPConfig {
        &self.config
    }

    /// 获取可变配置引用 / Get mutable configuration reference
    pub fn config_mut(&mut self) -> &mut SCIPConfig {
        &mut self.config
    }

    /// 将 SCIP 状态转换为求解器状态 / Convert SCIP status to solver status
    fn convert_status(status: SCIPStatus) -> SolverStatus {
        match status {
            SCIPStatus::Optimal => SolverStatus::Optimal,
            SCIPStatus::Infeasible => SolverStatus::Infeasible,
            SCIPStatus::Inforunbd => SolverStatus::InfeasibleOrUnbounded,
            SCIPStatus::Unbounded => SolverStatus::Unbounded,
            SCIPStatus::TimeLimit => SolverStatus::TimeLimit,
            SCIPStatus::NodeLimit => SolverStatus::NodeLimit,
            SCIPStatus::TotalNodeLimit => SolverStatus::TotalNodeLimit,
            SCIPStatus::StallNodeLimit => SolverStatus::StallNodeLimit,
            SCIPStatus::GapLimit => SolverStatus::GapLimit,
            SCIPStatus::SolutionLimit => SolverStatus::SolutionLimit,
            SCIPStatus::BestSolutionLimit => SolverStatus::BestSolutionLimit,
            SCIPStatus::RestartLimit => SolverStatus::RestartLimit,
            SCIPStatus::MemoryLimit => SolverStatus::MemoryLimit,
            SCIPStatus::UserInterrupt => SolverStatus::UserInterrupt,
            SCIPStatus::Terminate => SolverStatus::UserInterrupt,
            _ => SolverStatus::Unknown,
        }
    }

    fn refine_status_with_solution(status: SolverStatus, has_solution: bool) -> SolverStatus {
        if matches!(status, SolverStatus::Optimal) && !has_solution {
            SolverStatus::Unknown
        } else if has_solution && matches!(status, SolverStatus::Unknown) {
            SolverStatus::Feasible
        } else {
            status
        }
    }

    fn map_presolving_mode(mode: PresolvingMode) -> ParamSetting {
        match mode {
            PresolvingMode::Off => ParamSetting::Off,
            PresolvingMode::Fast => ParamSetting::Fast,
            PresolvingMode::Medium => ParamSetting::Default,
            PresolvingMode::Aggressive => ParamSetting::Aggressive,
        }
    }

    fn map_heuristics_priority(priority: i32) -> ParamSetting {
        if priority <= 0 {
            ParamSetting::Off
        } else if priority == 1 {
            ParamSetting::Fast
        } else if priority >= 3 {
            ParamSetting::Aggressive
        } else {
            ParamSetting::Default
        }
    }

    fn build_telemetry_runtime(
        &self,
        objective_category: ObjectiveCategory,
        scip_vars: &[Variable],
        cancellation_handle: Option<&SolveHandle>,
        callback_error: Arc<Mutex<Option<CoreError>>>,
    ) -> SCIPTelemetryRuntime {
        let native_event_mask =
            if cancellation_handle.is_some() || self.config.max_iterations.is_some() {
                self.config.native_event_mask | EventMask::NODE_EVENT | EventMask::LP_EVENT
            } else {
                self.config.native_event_mask
            };
        SCIPTelemetryRuntime {
            objective_category,
            scip_vars: scip_vars.to_vec(),
            no_improvement_time_limit: self.config.no_improvement_time_limit,
            interruptible_time: self.config.interruptible_time,
            interruptible_gap: self.config.interruptible_gap,
            improvement_tolerance: self.config.improvement_tolerance.unwrap_or(1e-9),
            telemetry_min_interval: self.config.telemetry_min_interval,
            native_event_mask,
            telemetry_callback: self.config.telemetry_callback.clone(),
            snapshot_observers: self.config.snapshot_observers.clone(),
            native_callback: self.config.native_callback.clone(),
            native_observers: self.config.native_observers.clone(),
            cancellation_handle: cancellation_handle.cloned(),
            max_iterations: self.config.max_iterations,
            iteration_limit_reached: Arc::new(AtomicBool::new(false)),
            callback_error,
        }
    }

    fn install_telemetry_handler(
        &self,
        scip: &mut Model<ProblemCreated>,
        objective_category: ObjectiveCategory,
        scip_vars: &[Variable],
        cancellation_handle: Option<&SolveHandle>,
        callback_error: Arc<Mutex<Option<CoreError>>>,
    ) -> Arc<AtomicBool> {
        let runtime = self.build_telemetry_runtime(
            objective_category,
            scip_vars,
            cancellation_handle,
            callback_error,
        );
        let iteration_limit_reached = Arc::clone(&runtime.iteration_limit_reached);
        if !runtime.enabled() {
            return iteration_limit_reached;
        }
        scip.include_eventhdlr(
            "__ospf_scip_telemetry",
            "OSPF SCIP telemetry callback bridge",
            Box::new(SCIPTelemetryEventHandler::new(runtime)),
        );
        iteration_limit_reached
    }

    fn emit_stage_status(
        &self,
        stage: SCIPStage,
        solver_status: Option<SolverStatus>,
        solve_time: Duration,
        output: Option<&SolverOutput>,
    ) -> Result<()> {
        if let Some(callback) = self.config.stage_callback.as_ref() {
            callback(&SCIPStageStatus {
                stage,
                solver_status,
                solve_time,
                objective_value: output.and_then(|value| value.objective_value),
                best_bound: output.and_then(|value| value.best_bound),
                mip_gap: output.and_then(|value| value.mip_gap),
                iterations: output.and_then(|value| value.iterations),
                node_count: output.and_then(|value| value.node_count),
            })?;
        }
        Ok(())
    }

    fn map_scip_error(context: &str, error: Retcode) -> CoreError {
        let message = format!("SCIP {} error: {:?}", context, error);
        match error {
            Retcode::ParameterUnknown
            | Retcode::ParameterWrongType
            | Retcode::ParameterWrongVal => {
                CoreError::Solver(crate::error::SolverError::InvalidInput(message))
            }
            Retcode::InvalidData
            | Retcode::InvalidCall
            | Retcode::NoProblem
            | Retcode::KeyAlreadyExisting => CoreError::solver_modeling(message),
            Retcode::PluginNotFound
            | Retcode::NoMemory
            | Retcode::ReadError
            | Retcode::WriteError
            | Retcode::NoFile
            | Retcode::FileCreateError => CoreError::solver_environment(message),
            Retcode::LpError => CoreError::solver_backend(message),
            Retcode::InvalidResult => CoreError::parsing_error(message),
            Retcode::NotImplemented => CoreError::NotImplemented(message),
            Retcode::MaxDepthLevel
            | Retcode::BranchError
            | Retcode::Error
            | Retcode::Unknown(_) => CoreError::solver_backend(message),
            Retcode::Okay => CoreError::contract_error(format!(
                "SCIP returned success where an error was expected in {}",
                context
            )),
        }
    }

    fn apply_unsolved_config(&self, mut model: Model<Unsolved>) -> Result<Model<Unsolved>> {
        if !self.config.output_flag {
            model = model.hide_output();
        }
        if let Some(tl) = self.config.time_limit {
            model = model
                .set_real_param("limits/time", tl)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(gap) = self.config.mip_gap {
            model = model
                .set_real_param("limits/gap", gap)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(nodes) = self.config.node_limit {
            model = model
                .set_longint_param("limits/nodes", nodes)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(solution_limit) = self.config.solution_limit {
            let solution_limit = i32::try_from(solution_limit).map_err(|_| {
                CoreError::Solver(crate::error::SolverError::InvalidInput(format!(
                    "SCIP solution limit {} exceeds the native int parameter range",
                    solution_limit
                )))
            })?;
            model = model
                .set_int_param("limits/solutions", solution_limit)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(mem_limit) = self.config.mem_limit {
            model = model
                .set_real_param("limits/memory", mem_limit)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(display_freq) = self.config.display_freq {
            model = model
                .set_int_param("display/freq", display_freq)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(threads) = self.config.threads {
            model = model
                .set_int_param("parallel/maxnthreads", threads)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(seed) = self.config.seed {
            model = model
                .set_int_param("randomization/randomseedshift", seed)
                .map_err(|e| Self::map_scip_error("set param", e))?;
            model = model
                .set_int_param("randomization/permutationseed", seed)
                .map_err(|e| Self::map_scip_error("set param", e))?;
            model = model
                .set_int_param("randomization/lpseed", seed)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(tolerance) = self.config.optimality_tolerance {
            model = model
                .set_real_param("numerics/dualfeastol", tolerance)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(tolerance) = self.config.feasibility_tolerance {
            model = model
                .set_real_param("numerics/feastol", tolerance)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(ref log_file) = self.config.log_file {
            model = model
                .set_str_param("output/file", log_file)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(presolving) = self.config.presolving {
            model = model.set_presolving(Self::map_presolving_mode(presolving));
        }
        if let Some(heuristics_priority) = self.config.heuristics_priority {
            model = model.set_heuristics(Self::map_heuristics_priority(heuristics_priority));
        }
        Ok(model)
    }

    fn create_problem(
        &self,
        objective_category: ObjectiveCategory,
    ) -> Result<Model<ProblemCreated>> {
        let model = self.apply_unsolved_config(Model::new())?;
        let model = model.include_default_plugins();
        let model = model.create_prob("model");
        Ok(match objective_category {
            ObjectiveCategory::Maximum => model.maximize(),
            ObjectiveCategory::Minimum => model.minimize(),
        })
    }

    fn apply_per_solve_limits(
        mut model: Model<ProblemCreated>,
        options: &SolveOptions<'_>,
    ) -> Result<Model<ProblemCreated>> {
        if let Some(time_limit) = options.time_limit {
            model = model
                .set_real_param("limits/time", time_limit.as_secs_f64())
                .map_err(|error| Self::map_scip_error("set per-solve time limit", error))?;
        }
        if let Some(node_limit) = options.node_limit {
            let node_limit = i64::try_from(node_limit).map_err(|_| {
                CoreError::Solver(SolverError::InvalidInput(
                    "SCIP per-solve node limit exceeds the native int range".to_owned(),
                ))
            })?;
            model = model
                .set_longint_param("limits/nodes", node_limit)
                .map_err(|error| Self::map_scip_error("set per-solve node limit", error))?;
        }
        if let Some(solution_limit) = options.solution_limit {
            let solution_limit = i32::try_from(solution_limit).map_err(|_| {
                CoreError::Solver(SolverError::InvalidInput(
                    "SCIP per-solve solution limit exceeds the native int range".to_owned(),
                ))
            })?;
            model = model
                .set_int_param("limits/solutions", solution_limit)
                .map_err(|error| Self::map_scip_error("set per-solve solution limit", error))?;
        }
        Ok(model)
    }

    fn collect_dual_solution(
        constraints: &[Constraint],
        objective_category: ObjectiveCategory,
    ) -> Option<Vec<f64>> {
        let mut duals = Vec::with_capacity(constraints.len());
        for constraint in constraints {
            let transformed = constraint.transformed();
            let row_dual = transformed
                .as_ref()
                .and_then(|transformed| transformed.row().map(|row| row.dual()))
                .or_else(|| constraint.row().map(|row| row.dual()));
            // SCIP 约束层可能返回表示“当前不可用”的超大哨兵值；优先读取 LP row 对偶，避免把哨兵当作证书。
            let dual = row_dual
                .filter(|value| value.is_finite() && value.abs() < 1e90)
                .or_else(|| {
                    transformed
                        .as_ref()
                        .and_then(|transformed| transformed.dual_sol())
                        .filter(|value| value.is_finite() && value.abs() < 1e90)
                })
                .or_else(|| {
                    constraint
                        .dual_sol()
                        .filter(|value| value.is_finite() && value.abs() < 1e90)
                })?;
            // SCIP 对最大化模型返回相反方向的 LP 行对偶；统一报告采用原模型方向。
            // SCIP returns the opposite LP-row dual orientation for maximization; unified reports use the original model orientation.
            duals.push(match objective_category {
                ObjectiveCategory::Maximum => -dual,
                ObjectiveCategory::Minimum => dual,
            });
        }
        Some(duals)
    }

    fn collect_farkas_solution(constraints: &[Constraint]) -> Option<Vec<f64>> {
        let mut duals = Vec::with_capacity(constraints.len());
        for constraint in constraints {
            let dual = constraint
                .transformed()
                .and_then(|transformed| transformed.farkas_dual_sol())
                .or_else(|| constraint.farkas_dual_sol())?;
            duals.push(dual);
        }
        Some(duals)
    }

    fn resolve_verified_linear_dual(
        &self,
        model: &LinearTriadModel,
        primal_values: &[f64],
        native_dual: Option<Vec<f64>>,
    ) -> Option<Vec<f64>> {
        if let Some(dual) = native_dual
            && linear_dual_certificate_is_valid(model, primal_values, &dual)
        {
            return Some(dual);
        }
        if model.has_integer_variables() || model.num_constraints() == 0 {
            return None;
        }

        // 原生 dual 不可用时求解显式对偶，并用原模型证书重新校验。
        // Solve the explicit dual when native multipliers are unavailable, then revalidate them against the primal model.
        let dual_model = model.to_dual();
        let Ok((dual_output, _)) =
            self.solve_linear_internal(&dual_model, 1, false, None, false, None)
        else {
            return None;
        };
        if !matches!(dual_output.status, SolverStatus::Optimal) {
            return None;
        }
        let dual_values = dual_output.solution?;
        let row_count = model.num_constraints();
        if dual_values.len() < row_count {
            return None;
        }
        let candidate = dual_values[..row_count].to_vec();
        linear_dual_certificate_is_valid(model, primal_values, &candidate).then_some(candidate)
    }

    fn relative_gap(objective_value: f64, best_bound: f64) -> Option<f64> {
        if !objective_value.is_finite() || !best_bound.is_finite() {
            return None;
        }
        let denominator = objective_value.abs().max(1.0);
        Some((objective_value - best_bound).abs() / denominator)
    }

    fn solutions_equal(left: &[f64], right: &[f64]) -> bool {
        left.len() == right.len()
            && left
                .iter()
                .zip(right.iter())
                .all(|(left, right)| (*left - *right).abs() <= f64::EPSILON)
    }

    fn push_unique_solution(solutions: &mut Vec<Vec<f64>>, candidate: Vec<f64>) {
        if !candidate.is_empty()
            && !solutions
                .iter()
                .any(|existing| Self::solutions_equal(existing, &candidate))
        {
            solutions.push(candidate);
        }
    }

    fn collect_solution_pool(
        primary_solution: Option<Vec<f64>>,
        scip_solutions: Option<Vec<russcip::solution::Solution>>,
        scip_vars: &[Variable],
        solution_amount: usize,
    ) -> Vec<Vec<f64>> {
        let mut solutions = Vec::new();
        if let Some(primary) = primary_solution {
            Self::push_unique_solution(&mut solutions, primary);
        }

        if solution_amount <= solutions.len() {
            solutions.truncate(solution_amount);
            return solutions;
        }

        if let Some(scip_solutions) = scip_solutions {
            for solution in scip_solutions {
                let candidate = scip_vars.iter().map(|var| solution.val(var)).collect();
                Self::push_unique_solution(&mut solutions, candidate);
                if solutions.len() >= solution_amount {
                    break;
                }
            }
        }
        solutions
    }
}

#[cfg(feature = "scip")]
impl SCIPSolver {
    /// 收集按变量索引排列的初始解 / Collect indexed initial solution values
    fn collect_initial_solution(tokens: &[Token<f64>]) -> Vec<(usize, f64)> {
        tokens
            .iter()
            .enumerate()
            .filter_map(|(index, token)| token.get_result().map(|value| (index, value)))
            .collect()
    }

    /// 写入 SCIP 初始解 / Inject SCIP initial solution
    fn inject_initial_solution(
        scip: &Model<ProblemCreated>,
        scip_vars: &[Variable],
        initial_solution: &[(usize, f64)],
    ) {
        if initial_solution.is_empty() {
            return;
        }
        let solution = scip.create_orig_sol();
        for &(index, value) in initial_solution {
            if let Some(var) = scip_vars.get(index) {
                solution.set_val(var, value);
            }
        }
        let _ = scip.add_sol(solution);
    }

    fn name(&self) -> &str {
        "SCIP"
    }

    fn native_version() -> String {
        let major = unsafe { russcip::ffi::SCIPmajorVersion() };
        let minor = unsafe { russcip::ffi::SCIPminorVersion() };
        let technical = unsafe { russcip::ffi::SCIPtechVersion() };
        format!("{}.{}.{}", major, minor, technical)
    }

    fn capabilities(&self) -> Vec<SolverCapability> {
        vec![
            SolverCapability::Linear,
            SolverCapability::Mip,
            SolverCapability::NativeIndicator,
            SolverCapability::NativeSOS1,
            // SCIP 的 QP 支持通常需要外部求解器（如 IPOPT）。
            // SCIP QP support usually requires an external solver such as IPOPT.
            #[cfg(feature = "scip-quadratic")]
            SolverCapability::Quadratic,
        ]
    }

    pub(super) fn provenance(&self) -> SolverProvenance {
        let mut configuration = BTreeMap::new();
        configuration.insert(
            "time_limit".to_owned(),
            format!("{:?}", self.config.time_limit),
        );
        configuration.insert("mip_gap".to_owned(), format!("{:?}", self.config.mip_gap));
        configuration.insert(
            "no_improvement_time_limit".to_owned(),
            format!("{:?}", self.config.no_improvement_time_limit),
        );
        configuration.insert(
            "interruptible_time".to_owned(),
            format!("{:?}", self.config.interruptible_time),
        );
        configuration.insert(
            "interruptible_gap".to_owned(),
            format!("{:?}", self.config.interruptible_gap),
        );
        configuration.insert(
            "improve_threshold".to_owned(),
            format!("{:?}", self.config.improvement_tolerance),
        );
        configuration.insert(
            "max_iterations".to_owned(),
            format!("{:?}", self.config.max_iterations),
        );
        configuration.insert("threads".to_owned(), format!("{:?}", self.config.threads));
        configuration.insert("seed".to_owned(), format!("{:?}", self.config.seed));
        configuration.insert(
            "node_limit".to_owned(),
            format!("{:?}", self.config.node_limit),
        );
        configuration.insert(
            "solution_limit".to_owned(),
            format!("{:?}", self.config.solution_limit),
        );
        configuration.insert(
            "mem_limit".to_owned(),
            format!("{:?}", self.config.mem_limit),
        );
        configuration.insert(
            "presolving".to_owned(),
            format!("{:?}", self.config.presolving),
        );
        configuration.insert(
            "heuristics_priority".to_owned(),
            format!("{:?}", self.config.heuristics_priority),
        );
        configuration.insert(
            "output_flag".to_owned(),
            self.config.output_flag.to_string(),
        );
        configuration.insert(
            "native_callback_registered".to_owned(),
            self.config.native_callback.is_some().to_string(),
        );
        configuration.insert(
            "telemetry_callback_registered".to_owned(),
            self.config.telemetry_callback.is_some().to_string(),
        );
        let native_version = Self::native_version();
        let mut environment_summary = BTreeMap::new();
        environment_summary.insert("binding".to_owned(), "russcip-0.9.1".to_owned());
        environment_summary.insert("native_version".to_owned(), native_version.clone());
        SolverProvenance {
            solver_id: format!("scip/{}", native_version),
            backend_name: "SCIP".to_owned(),
            backend_version: Some(native_version),
            plugin_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            requested_configuration: configuration.clone(),
            effective_configuration: configuration,
            thread_count: self.config.threads.map(|value| value.max(0) as usize),
            random_seed: self.config.seed.map(|value| value.max(0) as u64),
            deterministic: self.config.seed.map(|_| self.config.threads == Some(1)),
            environment_summary,
        }
    }

    pub(super) fn report_from_output(
        &self,
        output: SolverOutput,
        cancellation_handle: Option<&SolveHandle>,
    ) -> Result<SolveReport<f64>> {
        let native_status = output.status;
        if let Some(handle) = cancellation_handle {
            handle.mark_completed();
        }
        let mut report = match cancellation_handle {
            Some(handle) => {
                solver_output_to_report_with_cancellation(output, self.provenance(), handle)
            }
            None => solver_output_to_report_with_provenance(output, self.provenance()),
        }?;
        verify_native_terminal_proof(&mut report, native_status)?;
        let provenance = self.provenance();
        report.fingerprints.configuration = Some(configuration_fingerprint(
            &provenance.effective_configuration,
        ));
        report.fingerprints.solver = Some(solver_provenance_fingerprint(&provenance));
        if self.has_non_replayable_callback() {
            report.warnings.push(SolveWarning::new(
                "NonReplayableCallback",
                "backend callbacks are execution-only and are not included in replay identity",
            ));
        }
        Ok(report)
    }

    fn has_non_replayable_callback(&self) -> bool {
        self.config.stage_callback.is_some()
            || self.config.telemetry_callback.is_some()
            || self.config.native_callback.is_some()
            || !self.config.native_observers.is_empty()
    }

    fn attach_model_fingerprint(
        &self,
        mut report: SolveReport<f64>,
        model: &LinearTriadModel,
    ) -> Result<SolveReport<f64>> {
        report.fingerprints.model = Some(linear_model_fingerprint(model)?);
        attach_linear_solution_audit(report, model)
    }

    fn attach_quadratic_model_fingerprint(
        &self,
        mut report: SolveReport<f64>,
        model: &QuadraticTetradModel,
    ) -> Result<SolveReport<f64>> {
        report.fingerprints.model = Some(quadratic_model_fingerprint(model)?);
        attach_quadratic_model_mapping(report, model)
    }

    fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput> {
        let (output, _) = self.solve_linear_internal(model, 1, false, None, true, None)?;
        Ok(output)
    }

    fn solve_linear_with_solution_pool_internal(
        &self,
        model: &LinearTriadModel,
        solution_amount: usize,
    ) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
        self.solve_linear_internal(model, solution_amount.max(1), true, None, true, None)
    }

    fn solve_linear_with_options(
        &self,
        model: &LinearTriadModel,
        options: &SolveOptions<'_>,
    ) -> Result<SolverOutput> {
        validate_per_solve_limits(options)?;
        let (output, _) = self.solve_linear_internal(
            model,
            1,
            false,
            options.cancellation_handle,
            true,
            Some(options),
        )?;
        Ok(output)
    }

    fn solve_linear_internal(
        &self,
        model: &LinearTriadModel,
        solution_amount: usize,
        collect_solution_pool: bool,
        cancellation_handle: Option<&SolveHandle>,
        allow_explicit_dual_fallback: bool,
        per_solve_options: Option<&SolveOptions<'_>>,
    ) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
        crate::solver::audit::validate_linear_model_for_backend(model)?;
        let start_time = Instant::now();

        let mut scip = self.create_problem(model.objective_category)?;
        if collect_solution_pool && solution_amount > 1 {
            scip = scip
                .set_int_param("limits/solutions", solution_amount as i32)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(options) = per_solve_options {
            scip = Self::apply_per_solve_limits(scip, options)?;
        }

        // 添加变量
        // Add variables.
        let mut scip_vars = Vec::with_capacity(model.num_variables());
        let initial_solution = Self::collect_initial_solution(&model.variables);
        for (i, token) in model.variables.iter().enumerate() {
            let lb = model.lb[i];
            let ub = model.ub[i];
            let model_var_type = model
                .var_types
                .get(i)
                .copied()
                .unwrap_or_else(|| token.variable.var_type());
            let vtype = match model_var_type {
                VariableType::Binary => VarType::Binary,
                VariableType::Integer
                | VariableType::Ternary
                | VariableType::BalancedTernary
                | VariableType::UInteger => VarType::Integer,
                _ => VarType::Continuous,
            };

            let obj = model.c.get(i).copied().unwrap_or(0.0);
            let var = scip.add_var(lb, ub, obj, token.variable.name(), vtype);

            scip_vars.push(var);
        }
        Self::inject_initial_solution(&scip, &scip_vars, &initial_solution);

        // 添加约束：Ax <= b
        // Add constraints: Ax <= b.
        let mut linear_constraints: Vec<Constraint> = Vec::with_capacity(model.num_constraints());
        for i in 0..model.num_constraints() {
            let mut vars = Vec::new();
            let mut values = Vec::new();

            // 从稀疏矩阵提取约束行
            // Build sparse row terms from triad matrix row.
            if let Some(row) = model.A.get_row(i) {
                for &(j, val) in row.entries.iter() {
                    if val != 0.0 {
                        vars.push(&scip_vars[j]);
                        values.push(val);
                    }
                }
            }

            let constraint = scip.add_cons(
                vars,
                &values,
                -f64::INFINITY,
                model.b[i],
                &format!("c{}", i),
            );
            scip.set_cons_removable(&constraint, false);
            linear_constraints.push(constraint);
        }

        self.emit_stage_status(SCIPStage::AfterModeling, None, start_time.elapsed(), None)?;
        let callback_error = Arc::new(Mutex::new(None::<CoreError>));
        let iteration_limit_reached = self.install_telemetry_handler(
            &mut scip,
            model.objective_category,
            &scip_vars,
            cancellation_handle,
            Arc::clone(&callback_error),
        );
        self.emit_stage_status(SCIPStage::Configuration, None, start_time.elapsed(), None)?;

        let solved = scip.solve();
        if let Some(error) = callback_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            return Err(error);
        }
        let solution = solved.best_sol();
        let mut solver_status = Self::refine_status_with_solution(
            Self::convert_status(solved.status()),
            solution.is_some(),
        );
        if iteration_limit_reached.load(Ordering::SeqCst)
            && !cancellation_handle.is_some_and(SolveHandle::is_cancelled)
        {
            solver_status = SolverStatus::IterationLimit;
        }

        let mut output = SolverOutput::new(solver_status);
        output.node_count = Some(solved.n_nodes());
        output.iterations = Some(solved.n_lp_iterations());
        output.solution_count = Some(solved.n_sols());

        let best_bound = solved.best_bound();
        if best_bound.is_finite() {
            output.best_bound = Some(best_bound);
        }

        if solver_status.is_feasible()
            && let Some(solution) = solution
        {
            let objective_value = solution.obj_val();
            output.objective_value = Some(objective_value);
            output.solution = Some(scip_vars.iter().map(|var| solution.val(var)).collect());
            if let Some(best_bound) = output.best_bound {
                output.mip_gap = Self::relative_gap(objective_value, best_bound);
            }
        }

        // 线性对偶乘子（尽力获取）
        // Linear dual multipliers (best effort).
        if allow_explicit_dual_fallback
            && matches!(solver_status, SolverStatus::Optimal)
            && let Some(primal_values) = output.solution.as_deref()
        {
            let dual_solution = self.resolve_verified_linear_dual(
                model,
                primal_values,
                Self::collect_dual_solution(&linear_constraints, model.objective_category),
            );
            if let Some(dual_solution) = dual_solution
                && !dual_solution.is_empty()
            {
                output.dual_solution = Some(dual_solution);
            }
        }

        let solutions =
            if collect_solution_pool && solution_amount > 1 && output.status.is_feasible() {
                Self::collect_solution_pool(
                    output.solution.clone(),
                    solved.get_sols(),
                    &scip_vars,
                    solution_amount,
                )
            } else {
                Vec::new()
            };

        output.solve_time = start_time.elapsed();
        self.emit_stage_status(
            if output.status.is_feasible() {
                SCIPStage::AnalyzingSolution
            } else {
                SCIPStage::AfterFailure
            },
            Some(output.status),
            output.solve_time,
            Some(&output),
        )?;
        Ok((output, solutions))
    }

    fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput> {
        let (output, _) = self.solve_quadratic_internal(model, 1, false, None, None)?;
        Ok(output)
    }

    /// 求解线性模型并返回统一报告 / Solve a linear model as a unified report
    pub fn solve_linear_report(&self, model: &LinearTriadModel) -> Result<SolveReport<f64>> {
        self.attach_model_fingerprint(
            self.report_from_output(self.solve_linear(model)?, None)?,
            model,
        )
    }

    /// 求解二次模型并返回统一报告 / Solve a quadratic model as a unified report
    pub fn solve_quadratic_report(&self, model: &QuadraticTetradModel) -> Result<SolveReport<f64>> {
        self.attach_quadratic_model_fingerprint(
            self.report_from_output(self.solve_quadratic(model)?, None)?,
            model,
        )
    }

    fn solve_quadratic_with_solution_pool_internal(
        &self,
        model: &QuadraticTetradModel,
        solution_amount: usize,
    ) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
        self.solve_quadratic_internal(model, solution_amount.max(1), true, None, None)
    }

    fn solve_quadratic_with_options(
        &self,
        model: &QuadraticTetradModel,
        options: &SolveOptions<'_>,
    ) -> Result<SolverOutput> {
        validate_per_solve_limits(options)?;
        let (output, _) = self.solve_quadratic_internal(
            model,
            1,
            false,
            options.cancellation_handle,
            Some(options),
        )?;
        Ok(output)
    }

    fn solve_quadratic_internal(
        &self,
        model: &QuadraticTetradModel,
        solution_amount: usize,
        collect_solution_pool: bool,
        cancellation_handle: Option<&SolveHandle>,
        per_solve_options: Option<&SolveOptions<'_>>,
    ) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
        crate::solver::audit::validate_quadratic_model_for_backend(model)?;
        let start_time = Instant::now();

        let mut scip = self.create_problem(model.objective_category)?;
        if collect_solution_pool && solution_amount > 1 {
            scip = scip
                .set_int_param("limits/solutions", solution_amount as i32)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(options) = per_solve_options {
            scip = Self::apply_per_solve_limits(scip, options)?;
        }

        // 添加变量
        // Add variables.
        let mut scip_vars = Vec::with_capacity(model.num_variables());
        let initial_solution = Self::collect_initial_solution(&model.linear.variables);
        for (i, token) in model.linear.variables.iter().enumerate() {
            let lb = model.linear.lb[i];
            let ub = model.linear.ub[i];
            let model_var_type = model
                .linear
                .var_types
                .get(i)
                .copied()
                .unwrap_or_else(|| token.variable.var_type());
            let vtype = match model_var_type {
                VariableType::Binary => VarType::Binary,
                VariableType::Integer
                | VariableType::Ternary
                | VariableType::BalancedTernary
                | VariableType::UInteger => VarType::Integer,
                _ => VarType::Continuous,
            };

            let obj = model.c.get(i).copied().unwrap_or(0.0);
            let var = scip.add_var(lb, ub, obj, token.variable.name(), vtype);

            scip_vars.push(var);
        }
        Self::inject_initial_solution(&scip, &scip_vars, &initial_solution);

        // 通过引入辅助变量，将二次目标转换为二次约束。
        // Transform quadratic objective through auxiliary-variable quadratic constraints.
        let has_quadratic_objective = model.Q.rows.iter().any(|row| {
            row.entries
                .iter()
                .any(|(_, value)| value.abs() > f64::EPSILON)
        });
        if has_quadratic_objective {
            let objective_var = scip.add_var(
                -f64::INFINITY,
                f64::INFINITY,
                1.0,
                "__scip_quadratic_objective",
                VarType::Continuous,
            );
            let mut linear_coefs = vec![-1.0];
            let linear_vars = vec![&objective_var];
            let mut quad_vars_1 = Vec::new();
            let mut quad_vars_2 = Vec::new();
            let mut quad_coefs = Vec::new();

            for (i, row) in model.Q.rows.iter().enumerate() {
                if i >= scip_vars.len() {
                    return Err(CoreError::solver_modeling(format!(
                        "quadratic objective row index {} out of bounds for {} variables",
                        i,
                        scip_vars.len()
                    )));
                }
                for &(j, qval) in row.entries.iter() {
                    if qval.abs() <= f64::EPSILON {
                        continue;
                    }
                    if j >= scip_vars.len() {
                        return Err(CoreError::solver_modeling(format!(
                            "quadratic objective column index {} out of bounds for {} variables",
                            j,
                            scip_vars.len()
                        )));
                    }
                    quad_vars_1.push(&scip_vars[i]);
                    quad_vars_2.push(&scip_vars[j]);
                    quad_coefs.push(qval);
                }
            }

            let (lhs, rhs) = match model.objective_category {
                ObjectiveCategory::Minimum => (-f64::INFINITY, 0.0),
                ObjectiveCategory::Maximum => (0.0, f64::INFINITY),
            };
            scip.add_cons_quadratic(
                linear_vars,
                linear_coefs.as_mut_slice(),
                quad_vars_1,
                quad_vars_2,
                quad_coefs.as_mut_slice(),
                lhs,
                rhs,
                "__scip_quadratic_objective_cons",
            );
        }

        for (i, constraint) in model.quadratic_constraints.iter().enumerate() {
            let mut lin_vars = Vec::new();
            let mut lin_coefs = Vec::new();
            let mut quad_vars_1 = Vec::new();
            let mut quad_vars_2 = Vec::new();
            let mut quad_coefs = Vec::new();

            for monomial in constraint.polynomial.monomials() {
                let var_index1 = monomial.var_index1();
                if var_index1 >= scip_vars.len() {
                    return Err(CoreError::solver_modeling(format!(
                        "quadratic constraint {} references invalid variable index {}",
                        i, var_index1
                    )));
                }
                let coefficient = *monomial.coefficient();
                if coefficient.abs() <= f64::EPSILON {
                    continue;
                }
                if let Some(var_index2) = monomial.var_index2() {
                    if var_index2 >= scip_vars.len() {
                        return Err(CoreError::solver_modeling(format!(
                            "quadratic constraint {} references invalid variable index {}",
                            i, var_index2
                        )));
                    }
                    quad_vars_1.push(&scip_vars[var_index1]);
                    quad_vars_2.push(&scip_vars[var_index2]);
                    quad_coefs.push(coefficient);
                } else {
                    lin_vars.push(&scip_vars[var_index1]);
                    lin_coefs.push(coefficient);
                }
            }

            let shifted_rhs = constraint.rhs - *constraint.polynomial.constant();
            let (lhs, rhs) = match constraint.relation {
                ConstraintRelation::LessEqual => (-f64::INFINITY, shifted_rhs),
                ConstraintRelation::Equal => (shifted_rhs, shifted_rhs),
                ConstraintRelation::GreaterEqual => (shifted_rhs, f64::INFINITY),
            };
            let constraint_name = model
                .quadratic_constraint_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("qc{}", i));
            scip.add_cons_quadratic(
                lin_vars,
                lin_coefs.as_mut_slice(),
                quad_vars_1,
                quad_vars_2,
                quad_coefs.as_mut_slice(),
                lhs,
                rhs,
                &constraint_name,
            );
        }

        // 添加线性约束
        // Add linear constraints.
        let mut linear_constraints: Vec<Constraint> =
            Vec::with_capacity(model.linear.num_constraints());
        for i in 0..model.linear.num_constraints() {
            let mut vars = Vec::new();
            let mut values = Vec::new();

            if let Some(row) = model.linear.A.get_row(i) {
                for &(j, val) in row.entries.iter() {
                    if val != 0.0 {
                        vars.push(&scip_vars[j]);
                        values.push(val);
                    }
                }
            }

            let constraint = scip.add_cons(
                vars,
                &values,
                -f64::INFINITY,
                model.linear.b[i],
                &format!("c{}", i),
            );
            scip.set_cons_removable(&constraint, false);
            linear_constraints.push(constraint);
        }

        self.emit_stage_status(SCIPStage::AfterModeling, None, start_time.elapsed(), None)?;
        let callback_error = Arc::new(Mutex::new(None::<CoreError>));
        let iteration_limit_reached = self.install_telemetry_handler(
            &mut scip,
            model.objective_category,
            &scip_vars,
            cancellation_handle,
            Arc::clone(&callback_error),
        );
        self.emit_stage_status(SCIPStage::Configuration, None, start_time.elapsed(), None)?;

        let solved = scip.solve();
        if let Some(error) = callback_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            return Err(error);
        }
        let solution = solved.best_sol();
        let mut solver_status = Self::refine_status_with_solution(
            Self::convert_status(solved.status()),
            solution.is_some(),
        );
        if iteration_limit_reached.load(Ordering::SeqCst)
            && !cancellation_handle.is_some_and(SolveHandle::is_cancelled)
        {
            solver_status = SolverStatus::IterationLimit;
        }

        let mut output = SolverOutput::new(solver_status);
        output.node_count = Some(solved.n_nodes());
        output.iterations = Some(solved.n_lp_iterations());
        output.solution_count = Some(solved.n_sols());

        let best_bound = solved.best_bound();
        if best_bound.is_finite() {
            output.best_bound = Some(best_bound);
        }

        if solver_status.is_feasible()
            && let Some(solution) = solution
        {
            let objective_value = solution.obj_val();
            output.objective_value = Some(objective_value);
            output.solution = Some(scip_vars.iter().map(|var| solution.val(var)).collect());
            if let Some(best_bound) = output.best_bound {
                output.mip_gap = Self::relative_gap(objective_value, best_bound);
            }
        }

        // 线性约束对偶值（用于二次子问题最优性 cut）
        // Linear-row duals for quadratic-subproblem optimality cuts.
        if matches!(solver_status, SolverStatus::Optimal)
            && let Some(dual_solution) =
                Self::collect_dual_solution(&linear_constraints, model.objective_category)
            && !dual_solution.is_empty()
        {
            output.dual_solution = Some(dual_solution);
        }

        if solver_status.is_infeasible() {
            // 线性约束 Farkas 乘子（用于二次子问题可行性 cut）
            // Linear-row Farkas duals for quadratic-subproblem feasibility cuts.
            if let Some(farkas_solution) = Self::collect_farkas_solution(&linear_constraints)
                && !farkas_solution.is_empty()
            {
                output.dual_solution = Some(farkas_solution);
            }
        }

        let solutions =
            if collect_solution_pool && solution_amount > 1 && output.status.is_feasible() {
                Self::collect_solution_pool(
                    output.solution.clone(),
                    solved.get_sols(),
                    &scip_vars,
                    solution_amount,
                )
            } else {
                Vec::new()
            };

        output.solve_time = start_time.elapsed();
        self.emit_stage_status(
            if output.status.is_feasible() {
                SCIPStage::AnalyzingSolution
            } else {
                SCIPStage::AfterFailure
            },
            Some(output.status),
            output.solve_time,
            Some(&output),
        )?;
        Ok((output, solutions))
    }
}

#[cfg(feature = "scip")]
impl SolverInfo for SCIPSolver {
    fn name(&self) -> &str {
        SCIPSolver::name(self)
    }

    fn capabilities(&self) -> Vec<SolverCapability> {
        SCIPSolver::capabilities(self)
    }

    fn descriptor(&self) -> SolverDescriptor {
        let provenance = self.provenance();
        let capabilities = SolverCapabilities::from_legacy(&SCIPSolver::capabilities(self))
            .with("solution_pool", CapabilitySupport::Supported)
            .with("primal", CapabilitySupport::Supported)
            .with("dual", CapabilitySupport::Conditional)
            .with("farkas", CapabilitySupport::Conditional)
            .with("external_interrupt", CapabilitySupport::Supported);
        SolverDescriptor {
            solver_id: provenance.solver_id,
            display_name: "SCIP".to_owned(),
            backend_name: provenance.backend_name,
            backend_version: provenance.backend_version,
            runtime_available: None,
            capabilities,
            warnings: vec![
                "native runtime availability is confirmed when a solve starts".to_owned(),
                "dual and Farkas support depends on the active SCIP problem type".to_owned(),
            ],
        }
    }
}

#[cfg(feature = "scip")]
impl LinearSolver for SCIPSolver {
    fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput> {
        SCIPSolver::solve_linear(self, model)
    }

    fn solve_linear_report(&self, model: &LinearTriadModel) -> Result<SolveReport<f64>> {
        SCIPSolver::solve_linear_report(self, model)
    }

    fn solve_linear_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: &crate::solver::SolveOptions<'_>,
    ) -> Result<SolveReport<f64>> {
        validate_per_solve_limits(options)?;
        emit_progress_start(options.progress_reporter, self.name())?;
        if let Some(handle) = options.cancellation_handle
            && handle.is_cancelled()
        {
            let report = cancelled_solve_report(self.provenance(), handle)?;
            emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
            return Ok(report);
        }
        if let Some(callback) = options.solving_status_callback {
            callback(&SolvingStatus::solving(self.name()))?;
        }
        if options.solution_amount > 1
            && let Some((output, pool)) = Some(self.solve_linear_internal(
                model,
                options.solution_amount,
                true,
                options.cancellation_handle,
                true,
                Some(options),
            )?)
        {
            let final_status = SolvingStatus::from_output(self.name(), &output);
            let mut report = self.attach_model_fingerprint(
                self.report_from_output(output, options.cancellation_handle)?,
                model,
            )?;
            if let Some(solution) = report.solution.as_mut() {
                solution.pool = pool;
            }
            if let Some(callback) = options.solving_status_callback {
                callback(&final_status)?;
            }
            emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
            return Ok(report);
        }
        let output = self.solve_linear_with_options(model, options)?;
        let final_status = SolvingStatus::from_output(self.name(), &output);
        let report = self.attach_model_fingerprint(
            self.report_from_output(output, options.cancellation_handle)?,
            model,
        )?;
        if let Some(callback) = options.solving_status_callback {
            callback(&final_status)?;
        }
        emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
        Ok(report)
    }

    fn solve_linear_with_solution_pool(
        &self,
        model: &LinearTriadModel,
        solution_amount: usize,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        if solution_amount <= 1 {
            return Ok(None);
        }
        Ok(Some(self.solve_linear_with_solution_pool_internal(
            model,
            solution_amount,
        )?))
    }

    fn solve_linear_with_solution_pool_with_options(
        &self,
        model: &LinearTriadModel,
        solution_amount: usize,
        options: &crate::solver::SolveOptions<'_>,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        validate_per_solve_limits(options)?;
        if solution_amount <= 1 {
            return Ok(None);
        }
        Ok(Some(self.solve_linear_internal(
            model,
            solution_amount,
            true,
            options.cancellation_handle,
            true,
            Some(options),
        )?))
    }
}

#[cfg(feature = "scip")]
impl QuadraticSolver for SCIPSolver {
    fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput> {
        SCIPSolver::solve_quadratic(self, model)
    }

    fn solve_quadratic_report(&self, model: &QuadraticTetradModel) -> Result<SolveReport<f64>> {
        SCIPSolver::solve_quadratic_report(self, model)
    }

    fn solve_quadratic_report_with_options(
        &self,
        model: &QuadraticTetradModel,
        options: &crate::solver::SolveOptions<'_>,
    ) -> Result<SolveReport<f64>> {
        validate_per_solve_limits(options)?;
        emit_progress_start(options.progress_reporter, self.name())?;
        if let Some(handle) = options.cancellation_handle
            && handle.is_cancelled()
        {
            let report = cancelled_solve_report(self.provenance(), handle)?;
            emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
            return Ok(report);
        }
        if let Some(callback) = options.solving_status_callback {
            callback(&SolvingStatus::solving(self.name()))?;
        }
        if options.solution_amount > 1
            && let Some((output, pool)) = Some(self.solve_quadratic_internal(
                model,
                options.solution_amount,
                true,
                options.cancellation_handle,
                Some(options),
            )?)
        {
            let final_status = SolvingStatus::from_output(self.name(), &output);
            let mut report = self.attach_quadratic_model_fingerprint(
                self.report_from_output(output, options.cancellation_handle)?,
                model,
            )?;
            if let Some(solution) = report.solution.as_mut() {
                solution.pool = pool;
            }
            if let Some(callback) = options.solving_status_callback {
                callback(&final_status)?;
            }
            emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
            return Ok(report);
        }
        let output = self.solve_quadratic_with_options(model, options)?;
        let final_status = SolvingStatus::from_output(self.name(), &output);
        let report = self.attach_quadratic_model_fingerprint(
            self.report_from_output(output, options.cancellation_handle)?,
            model,
        )?;
        if let Some(callback) = options.solving_status_callback {
            callback(&final_status)?;
        }
        emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
        Ok(report)
    }

    fn solve_quadratic_with_solution_pool(
        &self,
        model: &QuadraticTetradModel,
        solution_amount: usize,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        if solution_amount <= 1 {
            return Ok(None);
        }
        Ok(Some(self.solve_quadratic_with_solution_pool_internal(
            model,
            solution_amount,
        )?))
    }

    fn solve_quadratic_with_solution_pool_with_options(
        &self,
        model: &QuadraticTetradModel,
        solution_amount: usize,
        options: &crate::solver::SolveOptions<'_>,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        validate_per_solve_limits(options)?;
        if solution_amount <= 1 {
            return Ok(None);
        }
        Ok(Some(self.solve_quadratic_internal(
            model,
            solution_amount,
            true,
            options.cancellation_handle,
            Some(options),
        )?))
    }
}

/// Rust-style alias for [`SCIPSolver`].
/// [`SCIPSolver`] 的 Rust 风格别名。
#[cfg(feature = "scip")]
pub type ScipSolver = SCIPSolver;

#[cfg(all(test, feature = "scip"))]
mod tests {
    use super::*;
    use crate::error::SolverErrorClass;
    use crate::model::intermediate::{BasicQuadraticTetradModel, SparseMatrix, SparseVector};
    use crate::model::{ConstraintRelation, ObjectiveCategory};
    use crate::symbol::flatten::{Quadratic, QuadraticMonomial};
    use crate::token::Token;
    use crate::variable::{BinaryVariableItem, UContinuousVariableItem};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn build_single_var_quadratic_model(name: &str) -> QuadraticTetradModel {
        let mut basic = BasicQuadraticTetradModel::new(name);
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            10.0,
            crate::variable::VariableType::UContinuous,
        );
        let mut model = QuadraticTetradModel::from_basic(basic);
        let mut q = SparseMatrix::new();
        q.add_row(SparseVector::new());
        model.set_objective(vec![1.0], q, ObjectiveCategory::Maximum);
        model
    }

    #[test]
    fn test_collect_initial_solution_keeps_solver_indices() {
        let x = Token::from_generic(BinaryVariableItem::auto("x"), 0);
        let y = Token::from_generic(UContinuousVariableItem::auto("y"), 1);
        let z = Token::from_generic(UContinuousVariableItem::auto("z"), 2);
        x.set_result(1.0);
        z.set_result(3.5);

        let initial_solution = SCIPSolver::collect_initial_solution(&[x, y, z]);

        assert_eq!(initial_solution, vec![(0, 1.0), (2, 3.5)]);
    }

    #[test]
    fn test_scip_config() {
        let config = SCIPConfig::new()
            .with_time_limit(120.0)
            .with_mip_gap(0.005)
            .with_output(false)
            .with_presolving(PresolvingMode::Aggressive)
            .with_node_limit(1000)
            .with_mem_limit(2048.0)
            .with_display_freq(10)
            .with_heuristics_priority(3)
            .with_no_improvement_time_limit(30.0)
            .with_interruptible_time(60.0)
            .with_interruptible_gap(0.05)
            .with_improvement_tolerance(1e-6)
            .with_seed(42)
            .with_optimality_tolerance(1e-8)
            .with_feasibility_tolerance(1e-7)
            .with_telemetry_min_interval(0.2);

        assert_eq!(config.time_limit, Some(120.0));
        assert_eq!(config.mip_gap, Some(0.005));
        assert!(!config.output_flag);
        assert_eq!(config.presolving, Some(PresolvingMode::Aggressive));
        assert_eq!(config.node_limit, Some(1000));
        assert_eq!(config.mem_limit, Some(2048.0));
        assert_eq!(config.display_freq, Some(10));
        assert_eq!(config.heuristics_priority, Some(3));
        assert_eq!(config.no_improvement_time_limit, Some(30.0));
        assert_eq!(config.interruptible_time, Some(60.0));
        assert_eq!(config.interruptible_gap, Some(0.05));
        assert_eq!(config.improvement_tolerance, Some(1e-6));
        assert_eq!(config.seed, Some(42));
        assert_eq!(config.optimality_tolerance, Some(1e-8));
        assert_eq!(config.feasibility_tolerance, Some(1e-7));
        assert_eq!(config.telemetry_min_interval, Some(0.2));
    }

    #[test]
    fn test_scip_retcode_mapping_preserves_error_boundaries() {
        let cases = [
            (Retcode::ParameterUnknown, SolverErrorClass::Input),
            (Retcode::ParameterWrongType, SolverErrorClass::Input),
            (Retcode::ParameterWrongVal, SolverErrorClass::Input),
            (Retcode::InvalidData, SolverErrorClass::Modeling),
            (Retcode::InvalidCall, SolverErrorClass::Modeling),
            (Retcode::NoProblem, SolverErrorClass::Modeling),
            (Retcode::KeyAlreadyExisting, SolverErrorClass::Modeling),
            (Retcode::PluginNotFound, SolverErrorClass::Environment),
            (Retcode::NoMemory, SolverErrorClass::Environment),
            (Retcode::ReadError, SolverErrorClass::Environment),
            (Retcode::WriteError, SolverErrorClass::Environment),
            (Retcode::NoFile, SolverErrorClass::Environment),
            (Retcode::FileCreateError, SolverErrorClass::Environment),
            (Retcode::LpError, SolverErrorClass::Backend),
            (Retcode::InvalidResult, SolverErrorClass::Parsing),
            (Retcode::NotImplemented, SolverErrorClass::Unsupported),
            (Retcode::MaxDepthLevel, SolverErrorClass::Backend),
            (Retcode::BranchError, SolverErrorClass::Backend),
            (Retcode::Error, SolverErrorClass::Backend),
            (Retcode::Okay, SolverErrorClass::InternalContract),
        ];

        for (retcode, expected) in cases {
            let error = SCIPSolver::map_scip_error("test", retcode);
            assert_eq!(error.solver_error_class(), expected, "{retcode:?}");
            assert!(!error.is_terminal_projection(), "{retcode:?}");
        }
    }

    #[test]
    fn test_scip_provenance_separates_native_product_from_rust_binding() {
        let provenance = SCIPSolver::new().provenance();
        let native_version = provenance
            .environment_summary
            .get("native_version")
            .expect("native SCIP version");
        assert_eq!(
            provenance.backend_version.as_deref(),
            Some(native_version.as_str())
        );
        assert_eq!(provenance.solver_id, format!("scip/{}", native_version));
        assert_eq!(
            provenance
                .environment_summary
                .get("binding")
                .map(String::as_str),
            Some("russcip-0.9.1")
        );
        assert_ne!(native_version, "russcip-0.9.1");
    }

    #[test]
    fn test_scip_callback_report_is_marked_non_replayable() {
        let config = SCIPConfig::new().add_configuration_callback(Arc::new(|_| Ok(())));
        let report = SCIPSolver::with_config(config)
            .report_from_output(SolverOutput::optimal(1.0, vec![1.0]), None)
            .expect("callback report should be valid");
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.code == "NonReplayableCallback")
        );
    }

    #[test]
    fn test_common_config_preserves_deterministic_parameters() {
        let common = crate::solver::SolverConfig::new("scip")
            .with_seed(42)
            .with_optimality_tolerance(1e-8)
            .with_feasibility_tolerance(1e-7)
            .with_interruptible_time(std::time::Duration::from_secs(60))
            .with_interruptible_gap(0.05);

        let config = SCIPConfig::from(&common);

        assert_eq!(config.seed, Some(42));
        assert_eq!(config.optimality_tolerance, Some(1e-8));
        assert_eq!(config.feasibility_tolerance, Some(1e-7));
        assert_eq!(config.interruptible_time, Some(60.0));
        assert_eq!(config.interruptible_gap, Some(0.05));
    }

    #[test]
    fn test_scip_config_ergonomic_aliases() {
        let config = SCIPConfig::new()
            .with_gap(0.02)
            .with_memory_limit_gb(1.5)
            .with_improve_threshold(1e-5);

        assert_eq!(config.mip_gap, Some(0.02));
        assert_eq!(config.mem_limit, Some(1536.0));
        assert_eq!(config.improvement_tolerance, Some(1e-5));

        let config = config.with_memory_limit_mb(512.0);
        assert_eq!(config.mem_limit, Some(512.0));
    }

    #[test]
    fn test_scip_config_native_event_mask_defaults_and_setters() {
        let config = SCIPConfig::new();
        assert!(
            config.native_event_mask == SCIPConfig::default_native_event_mask(),
            "default native event mask should match Kotlin wrapper default"
        );

        let custom_mask = EventMask::LP_EVENT | EventMask::NODE_EVENT;
        let config = config.with_native_event_mask(custom_mask);
        assert!(config.native_event_mask == custom_mask);

        let callback: SCIPNativeCallback = Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let config = SCIPConfig::new().with_native_callback_with_event_mask(custom_mask, callback);
        assert!(config.native_event_mask == custom_mask);
        assert!(config.native_callback.is_some());
    }

    #[test]
    fn test_scip_lp_subproblem_defaults() {
        let config = SCIPConfig::new()
            .with_time_limit(10.0)
            .with_mip_gap(0.01)
            .with_lp_subproblem_defaults();

        assert_eq!(config.threads, Some(1));
        assert_eq!(config.presolving, Some(PresolvingMode::Off));
        assert_eq!(config.heuristics_priority, Some(0));
        assert_eq!(config.time_limit, Some(10.0));
        assert_eq!(config.mip_gap, Some(0.01));

        let defaults = SCIPConfig::recommended_lp_subproblem_defaults();
        assert_eq!(defaults.threads, Some(1));
        assert_eq!(defaults.presolving, Some(PresolvingMode::Off));
        assert_eq!(defaults.heuristics_priority, Some(0));
    }

    #[test]
    fn test_scip_solver() {
        let solver = SCIPSolver::new();
        assert_eq!(solver.name(), "SCIP");
        assert!(solver.supports(SolverCapability::Linear));
        assert!(solver.supports(SolverCapability::Mip));
        assert!(solver.supports(SolverCapability::NativeIndicator));
        assert!(solver.supports(SolverCapability::NativeSOS1));
    }

    #[test]
    fn test_presolving_mode() {
        assert_eq!(PresolvingMode::Off, PresolvingMode::Off);
        assert_ne!(PresolvingMode::Fast, PresolvingMode::Aggressive);
    }

    #[test]
    fn test_status_mapping_for_inforunbd() {
        assert_eq!(
            SCIPSolver::convert_status(SCIPStatus::Inforunbd),
            SolverStatus::InfeasibleOrUnbounded
        );
    }

    #[test]
    fn optimal_status_without_solution_is_unknown() {
        assert_eq!(
            SCIPSolver::refine_status_with_solution(SolverStatus::Optimal, false),
            SolverStatus::Unknown
        );
        assert_eq!(
            SCIPSolver::refine_status_with_solution(SolverStatus::Optimal, true),
            SolverStatus::Optimal
        );
        assert_eq!(
            SCIPSolver::refine_status_with_solution(SolverStatus::Unknown, true),
            SolverStatus::Feasible
        );
    }

    #[test]
    fn test_stage_callback_helpers_can_chain_and_filter() {
        let modeling_counter = Arc::new(AtomicUsize::new(0));
        let failure_counter = Arc::new(AtomicUsize::new(0));

        let modeling_counter_ref = modeling_counter.clone();
        let on_modeling: SCIPStageCallback = Arc::new(move |_| {
            modeling_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let failure_counter_ref = failure_counter.clone();
        let on_failure: SCIPStageCallback = Arc::new(move |_| {
            failure_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        let config = SCIPConfig::new()
            .add_after_modeling_callback(on_modeling)
            .add_after_failure_callback(on_failure);
        let callback = config
            .stage_callback
            .as_ref()
            .expect("stage callback should be registered");

        callback(&SCIPStageStatus {
            stage: SCIPStage::Configuration,
            solver_status: None,
            solve_time: Duration::ZERO,
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        })
        .expect("configuration stage should be ignored");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 0);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 0);

        callback(&SCIPStageStatus {
            stage: SCIPStage::AfterModeling,
            solver_status: None,
            solve_time: Duration::ZERO,
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        })
        .expect("after modeling stage should be accepted");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 0);

        callback(&SCIPStageStatus {
            stage: SCIPStage::AfterFailure,
            solver_status: Some(SolverStatus::Infeasible),
            solve_time: Duration::ZERO,
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        })
        .expect("after failure stage should be accepted");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_telemetry_and_snapshot_observer_helpers_chain() {
        let telemetry_counter = Arc::new(AtomicUsize::new(0));
        let observer_counter = Arc::new(AtomicUsize::new(0));

        let telemetry_counter_ref = telemetry_counter.clone();
        let telemetry: SCIPTelemetryCallback = Arc::new(move |_| {
            telemetry_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let observer_counter_ref = observer_counter.clone();
        let observer: SCIPSnapshotObserver = Arc::new(move |_| {
            observer_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(SCIPSnapshotControl::Continue)
        });

        let config = SCIPConfig::new()
            .add_telemetry_callback(telemetry)
            .add_snapshot_observer(observer);
        assert!(config.telemetry_callback.is_some());
        assert_eq!(config.snapshot_observers.len(), 1);

        let callback = config
            .telemetry_callback
            .as_ref()
            .expect("telemetry callback should be registered");
        callback(&SCIPTelemetryStatus {
            solve_time: Duration::ZERO,
            objective_category: ObjectiveCategory::Minimum,
            initial_objective_value: Some(1.0),
            objective_value: Some(1.0),
            incumbent_solution: Some(vec![1.0]),
            best_bound: Some(0.8),
            mip_gap: Some(0.2),
            iterations: Some(10),
            node_count: Some(5),
        })
        .expect("telemetry callback should execute");
        assert_eq!(telemetry_counter.load(Ordering::SeqCst), 1);

        let control = (config.snapshot_observers[0])(&SCIPTelemetryStatus {
            solve_time: Duration::ZERO,
            objective_category: ObjectiveCategory::Minimum,
            initial_objective_value: Some(1.0),
            objective_value: Some(1.0),
            incumbent_solution: Some(vec![1.0]),
            best_bound: Some(0.8),
            mip_gap: Some(0.2),
            iterations: Some(10),
            node_count: Some(5),
        })
        .expect("snapshot observer should execute");
        assert!(matches!(control, SCIPSnapshotControl::Continue));
        assert_eq!(observer_counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_native_callback_and_observer_helpers_register() {
        let native_callback: SCIPNativeCallback = Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let native_observer: SCIPNativeObserver = Arc::new(|_| Ok(SCIPNativeControl::Continue));

        let config = SCIPConfig::new()
            .with_native_callback(Some(native_callback))
            .add_native_observer(native_observer);

        assert!(config.native_callback.is_some());
        assert_eq!(config.native_observers.len(), 1);
    }

    #[test]
    fn test_native_callback_override_and_observer_append_semantics() {
        let first_callback: SCIPNativeCallback = Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let second_callback: SCIPNativeCallback = Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let first_observer: SCIPNativeObserver = Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let second_observer: SCIPNativeObserver = Arc::new(|_| Ok(SCIPNativeControl::Continue));

        let config = SCIPConfig::new()
            .add_native_callback(first_callback.clone())
            .add_native_callback(second_callback.clone())
            .add_native_observer(first_observer.clone())
            .add_native_observer(second_observer.clone());

        let callback = config
            .native_callback
            .as_ref()
            .expect("native callback should be registered");
        assert!(
            Arc::ptr_eq(callback, &second_callback),
            "add_native_callback should override previous callback"
        );
        assert_eq!(
            config.native_observers.len(),
            2,
            "add_native_observer should append observers"
        );
        assert!(Arc::ptr_eq(&config.native_observers[0], &first_observer));
        assert!(Arc::ptr_eq(&config.native_observers[1], &second_observer));
    }

    #[test]
    fn test_native_where_classification_and_mask_helper() {
        let node_where = SCIPTelemetryEventHandler::classify_native_where(EventMask::NODE_EVENT);
        assert!(matches!(node_where, SCIPNativeWhere::Node));
        let lp_where = SCIPTelemetryEventHandler::classify_native_where(EventMask::LP_EVENT);
        assert!(matches!(lp_where, SCIPNativeWhere::Lp));

        let snapshot = SCIPNativeSnapshot {
            where_point: node_where,
            event_mask_bits: u64::from(EventMask::NODE_EVENT),
            handler_name: "__test__".to_string(),
            solve_time: Duration::ZERO,
            objective_category: ObjectiveCategory::Minimum,
            initial_objective_value: None,
            objective_value: None,
            incumbent_solution: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        assert!(snapshot.matches_mask_bits(u64::from(EventMask::NODE_EVENT)));
        assert!(!snapshot.matches_mask_bits(u64::from(EventMask::LP_EVENT)));
    }

    #[test]
    fn test_scip_quadratic_constraint_feasible() {
        let solver = SCIPSolver::with_config(SCIPConfig::new().with_output(false));
        let mut model = build_single_var_quadratic_model("scip_qc_feasible");
        model.add_quadratic_constraint_with_metadata(
            crate::model::mechanism::QuadraticInequality::new(
                Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0),
                ConstraintRelation::LessEqual,
                1.0,
            ),
            "qc_feasible".to_string(),
            None,
            false,
            0,
            None,
            None,
        );

        let output = solver
            .solve_quadratic(&model)
            .expect("quadratic model with feasible quadratic constraint should solve");
        assert!(output.status.is_feasible());
        let solution = output
            .solution
            .as_ref()
            .expect("feasible quadratic solve should provide solution vector");
        assert!(solution[0] <= 1.0 + 1e-6);
        assert!(solution[0] >= -1e-6);
    }

    #[test]
    fn test_scip_quadratic_constraint_infeasible() {
        let solver = SCIPSolver::with_config(SCIPConfig::new().with_output(false));
        let mut model = build_single_var_quadratic_model("scip_qc_infeasible");
        model.add_quadratic_constraint_with_metadata(
            crate::model::mechanism::QuadraticInequality::new(
                Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0),
                ConstraintRelation::LessEqual,
                -1.0,
            ),
            "qc_infeasible".to_string(),
            None,
            false,
            0,
            None,
            None,
        );

        let output = solver
            .solve_quadratic(&model)
            .expect("quadratic model solve should finish with infeasible status");
        assert!(output.status.is_infeasible());
    }

    #[test]
    fn test_scip_quadratic_objective_affects_solution() {
        let solver = SCIPSolver::with_config(SCIPConfig::new().with_output(false));
        let mut basic = BasicQuadraticTetradModel::new("scip_q_obj");
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            1.0,
            3.0,
            crate::variable::VariableType::UContinuous,
        );
        let mut model = QuadraticTetradModel::from_basic(basic);
        let mut q = SparseMatrix::new();
        let mut row = SparseVector::new();
        row.add(0, 1.0);
        q.add_row(row);
        model.set_objective(vec![0.0], q, ObjectiveCategory::Minimum);

        let output = solver
            .solve_quadratic(&model)
            .expect("quadratic objective model should solve");
        assert!(output.status.is_feasible());
        let solution = output
            .solution
            .as_ref()
            .expect("feasible quadratic objective solve should provide solution");
        assert!((solution[0] - 1.0).abs() <= 1e-6);
        let objective = output
            .objective_value
            .expect("feasible quadratic objective solve should provide objective");
        assert!((objective - 1.0).abs() <= 1e-5);
    }
}
