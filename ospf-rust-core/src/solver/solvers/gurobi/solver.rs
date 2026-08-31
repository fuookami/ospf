//! Gurobi 求解器主体
//! Gurobi solver core

use std::time::{Duration, Instant};
use crate::error::Result;
use crate::model::intermediate::{LinearTriadModel, QuadraticTetradModel};
use crate::model::object::ObjectiveCategory;
use crate::solver::{

    LinearSolver, QuadraticSolver, SolverCapability, SolverInfo, SolverOutput, SolverStatus,
};

use super::config::{
    GurobiConfig, GurobiNativeControl, GurobiNativeSnapshot, GurobiNativeWhere,
    GurobiNumericDiagnostics, GurobiNumericProfile, GurobiStage, GurobiStageStatus,
    GurobiTelemetryStatus,
};
use super::{linear, quadratic};

#[derive(Debug, Clone, Copy)]
pub(super) struct GurobiNumericSettings {
    pub zero_tolerance: f64,
    pub numeric_focus: Option<i32>,
    pub scale_flag: Option<i32>,
}

/// Gurobi 求解器 / Gurobi Solver
///
/// 封装 Gurobi 优化器的求解器实现。
/// Solver implementation wrapping Gurobi optimizer.
///
/// # 特性 / Features
///
/// - 支持 LP、MIP、QP、MIQP
/// - 支持时间限制、Gap 容差等参数配置
/// - 支持对偶解获取
///
/// # 前提条件 / Prerequisites
///
/// - 需要启用 `gurobi10`、`gurobi11` 或 `gurobi12` feature
/// - 需要安装 Gurobi 软件和有效许可证
///
/// # 示例 / Examples
///
/// ```rust,ignore
/// use ospf_rust_core::solver::solvers::GurobiSolver;
///
/// let solver = GurobiSolver::new();
/// let result = solver.solve_linear(&model)?;
///
/// if let Some(solution) = result.solution {
///     println!("Optimal solution: {:?}", solution);
/// }
/// ```
#[derive(Debug)]
pub struct GurobiSolver {
    /// 配置 / Configuration
    config: GurobiConfig,
}

impl Default for GurobiSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl GurobiSolver {
    fn compute_relative_gap(objective_value: f64, best_bound: f64) -> f64 {
        let denominator = objective_value.abs().max(1.0);
        ((objective_value - best_bound).abs() / denominator).max(0.0)
    }

    fn recommend_numeric_profile(diagnostics: &GurobiNumericDiagnostics) -> GurobiNumericProfile {
        if diagnostics.non_finite_coefficient_count > 0 {
            return GurobiNumericProfile::Robust;
        }
        let dynamic_range = diagnostics.dynamic_range.unwrap_or(1.0);
        if dynamic_range >= 1e12
            || (diagnostics.tiny_coefficient_count > 0 && diagnostics.large_coefficient_count > 0)
        {
            return GurobiNumericProfile::Robust;
        }
        if dynamic_range >= 1e8
            || diagnostics.tiny_coefficient_count > 0
            || diagnostics.large_coefficient_count > 0
        {
            return GurobiNumericProfile::Balanced;
        }
        GurobiNumericProfile::Performance
    }

    fn build_numeric_diagnostics(&self, coefficients: &[f64]) -> GurobiNumericDiagnostics {
        let mut nonzero_coefficient_count = 0usize;
        let mut non_finite_coefficient_count = 0usize;
        let mut min_abs_nonzero_coefficient: Option<f64> = None;
        let mut max_abs_coefficient: Option<f64> = None;
        let mut tiny_coefficient_count = 0usize;
        let mut large_coefficient_count = 0usize;

        for coefficient in coefficients {
            if !coefficient.is_finite() {
                non_finite_coefficient_count += 1;
                continue;
            }
            let abs_value = coefficient.abs();
            if abs_value == 0.0 {
                continue;
            }
            nonzero_coefficient_count += 1;
            min_abs_nonzero_coefficient = Some(
                min_abs_nonzero_coefficient
                    .map(|value| value.min(abs_value))
                    .unwrap_or(abs_value),
            );
            max_abs_coefficient = Some(
                max_abs_coefficient
                    .map(|value| value.max(abs_value))
                    .unwrap_or(abs_value),
            );
            if abs_value < 1e-10 {
                tiny_coefficient_count += 1;
            }
            if abs_value > 1e6 {
                large_coefficient_count += 1;
            }
        }

        let dynamic_range = min_abs_nonzero_coefficient
            .zip(max_abs_coefficient)
            .and_then(|(minimum, maximum)| (minimum > 0.0).then_some(maximum / minimum));
        let mut diagnostics = GurobiNumericDiagnostics {
            nonzero_coefficient_count,
            non_finite_coefficient_count,
            min_abs_nonzero_coefficient,
            max_abs_coefficient,
            dynamic_range,
            tiny_coefficient_count,
            large_coefficient_count,
            recommended_profile: GurobiNumericProfile::Performance,
        };
        diagnostics.recommended_profile = Self::recommend_numeric_profile(&diagnostics);
        diagnostics
    }

    fn apply_numeric_profile(settings: &mut GurobiNumericSettings, profile: GurobiNumericProfile) {
        match profile {
            GurobiNumericProfile::Robust => {
                settings.zero_tolerance = settings.zero_tolerance.max(1e-10);
                if settings.numeric_focus.is_none() {
                    settings.numeric_focus = Some(3);
                }
                if settings.scale_flag.is_none() {
                    settings.scale_flag = Some(2);
                }
            }
            GurobiNumericProfile::Balanced => {
                settings.zero_tolerance = settings.zero_tolerance.max(1e-12);
                if settings.numeric_focus.is_none() {
                    settings.numeric_focus = Some(1);
                }
                if settings.scale_flag.is_none() {
                    settings.scale_flag = Some(1);
                }
            }
            GurobiNumericProfile::Performance => {
                if settings.zero_tolerance <= 0.0 || !settings.zero_tolerance.is_finite() {
                    settings.zero_tolerance = 1e-13;
                }
                if settings.numeric_focus.is_none() {
                    settings.numeric_focus = Some(0);
                }
                if settings.scale_flag.is_none() {
                    settings.scale_flag = Some(-1);
                }
            }
        }
    }

    pub(super) fn resolve_numeric_settings(
        &self,
        coefficients: &[f64],
    ) -> Result<GurobiNumericSettings> {
        let mut settings = GurobiNumericSettings {
            zero_tolerance: if self.config.coefficient_zero_tolerance.is_finite()
                && self.config.coefficient_zero_tolerance > 0.0
            {
                self.config.coefficient_zero_tolerance
            } else {
                1e-13
            },
            numeric_focus: self.config.numeric_focus,
            scale_flag: self.config.scale_flag,
        };

        if !self.config.enable_numeric_diagnostics {
            return Ok(settings);
        }

        let diagnostics = self.build_numeric_diagnostics(coefficients);
        if diagnostics.non_finite_coefficient_count > 0 {
            return Err(crate::error::CoreError::Solver(
                crate::error::SolverError::NonFinite(format!(
                    "numeric diagnostics found {} non-finite coefficients",
                    diagnostics.non_finite_coefficient_count
                )),
            ));
        }

        if let Some(callback) = self.config.numeric_diagnostics_callback.as_ref() {
            callback(&diagnostics)?;
        }
        if self.config.auto_apply_numeric_profile {
            Self::apply_numeric_profile(&mut settings, diagnostics.recommended_profile);
        }
        Ok(settings)
    }

    /// 创建新的 Gurobi 求解器 / Create new Gurobi solver
    pub fn new() -> Self {
        Self {
            config: GurobiConfig::default(),
        }
    }

    /// 创建带配置的求解器 / Create solver with configuration
    pub fn with_config(config: GurobiConfig) -> Self {
        Self { config }
    }

    /// 获取配置引用 / Get configuration reference
    pub fn config(&self) -> &GurobiConfig {
        &self.config
    }

    /// 获取可变配置引用 / Get mutable configuration reference
    pub fn config_mut(&mut self) -> &mut GurobiConfig {
        &mut self.config
    }

    pub(super) fn create_env(&self) -> Result<grb::Env> {
        use grb::prelude::*;

        let has_compute_server = self.config.compute_server.is_some()
            || self.config.server_password.is_some()
            || self.config.server_timeout.is_some()
            || self.config.cs_queue_timeout.is_some();

        if has_compute_server {
            let mut env = Env::empty().map_err(|e| {
                crate::error::CoreError::SolverEnvironmentLost(crate::error::SolverEnvironmentLostError::new(format!(
                    "Gurobi empty env error: {}",
                    e
                )))
            })?;
            self.apply_env_params_empty(&mut env)?;
            let mut started_env = env.start().map_err(|e| {
                crate::error::CoreError::SolverEnvironmentLost(crate::error::SolverEnvironmentLostError::new(format!(
                    "Gurobi env start error: {}",
                    e
                )))
            })?;
            self.apply_env_callback(&mut started_env)?;
            return Ok(started_env);
        }

        let mut env = Env::new("").map_err(|e| {
            crate::error::CoreError::SolverEnvironmentLost(crate::error::SolverEnvironmentLostError::new(format!(
                "Gurobi env error: {}",
                e
            )))
        })?;
        self.apply_env_params_started(&mut env)?;
        self.apply_env_callback(&mut env)?;
        Ok(env)
    }

    fn apply_env_params_started(&self, env: &mut grb::Env) -> Result<()> {
        use grb::prelude::*;

        if !self.config.output_flag {
            env.set(param::OutputFlag, 0).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(tl) = self.config.time_limit {
            env.set(param::TimeLimit, tl).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(gap) = self.config.mip_gap {
            env.set(param::MIPGap, gap).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(iter) = self.config.max_iterations {
            env.set(param::IterationLimit, iter as f64).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(threads) = self.config.threads {
            env.set(param::Threads, threads).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(seed) = self.config.seed {
            env.set(param::Seed, seed).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(tolerance) = self.config.optimality_tolerance {
            env.set(param::OptimalityTol, tolerance).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(tolerance) = self.config.feasibility_tolerance {
            env.set(param::FeasibilityTol, tolerance).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(ref log_file) = self.config.log_file {
            env.set(param::LogFile, log_file.clone()).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        Ok(())
    }

    fn apply_env_params_empty(&self, env: &mut grb::EmptyEnv) -> Result<()> {
        use grb::prelude::*;

        if !self.config.output_flag {
            env.set(param::OutputFlag, 0).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(tl) = self.config.time_limit {
            env.set(param::TimeLimit, tl).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(gap) = self.config.mip_gap {
            env.set(param::MIPGap, gap).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(iter) = self.config.max_iterations {
            env.set(param::IterationLimit, iter as f64).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(threads) = self.config.threads {
            env.set(param::Threads, threads).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(seed) = self.config.seed {
            env.set(param::Seed, seed).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(tolerance) = self.config.optimality_tolerance {
            env.set(param::OptimalityTol, tolerance).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(tolerance) = self.config.feasibility_tolerance {
            env.set(param::FeasibilityTol, tolerance).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(ref log_file) = self.config.log_file {
            env.set(param::LogFile, log_file.clone()).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(server_timeout) = self.config.server_timeout {
            env.set(param::ServerTimeout, server_timeout).map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi param error: {}",
                    e
                )))
            })?;
        }
        if let Some(cs_queue_timeout) = self.config.cs_queue_timeout {
            env.set(param::CSQueueTimeout, cs_queue_timeout)
                .map_err(|e| {
                    crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(
                        format!("Gurobi param error: {}", e),
                    ))
                })?;
        }
        if let Some(ref compute_server) = self.config.compute_server {
            env.set(param::ComputeServer, compute_server.clone())
                .map_err(|e| {
                    crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(
                        format!("Gurobi param error: {}", e),
                    ))
                })?;
        }
        if let Some(ref server_password) = self.config.server_password {
            env.set(param::ServerPassword, server_password.clone())
                .map_err(|e| {
                    crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(
                        format!("Gurobi param error: {}", e),
                    ))
                })?;
        }
        Ok(())
    }

    fn apply_env_callback(&self, env: &mut grb::Env) -> Result<()> {
        if let Some(callback) = self.config.env_callback.as_ref() {
            callback(env)?;
        }
        Ok(())
    }

    pub(super) fn optimize_model(
        &self,
        grb_model: &mut grb::Model,
        objective_category: ObjectiveCategory,
        grb_vars: Option<&[grb::Var]>,
    ) -> Result<()> {
        let no_improvement_time_limit = self
            .config
            .no_improvement_time_limit
            .filter(|seconds| *seconds > 0.0);
        let telemetry_min_interval = self
            .config
            .telemetry_min_interval
            .filter(|seconds| *seconds > 0.0);
        let telemetry_callback = self.config.telemetry_callback.clone();
        let native_observers = self.config.native_observers.clone();
        let native_callback = self.config.native_callback.clone();
        if no_improvement_time_limit.is_none()
            && telemetry_callback.is_none()
            && native_observers.is_empty()
            && native_callback.is_none()
        {
            return grb_model.optimize().map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi optimize error: {}",
                    e
                )))
            });
        }

        let mut best_objective: Option<f64> = None;
        let mut initial_objective_value: Option<f64> = None;
        let mut last_improvement_time = Instant::now();
        let optimize_start_time = Instant::now();
        let mut last_telemetry_emit_time: Option<Instant> = None;
        let mut pending_observer_terminate = false;
        let callback_vars = grb_vars.map(|vars| vars.to_vec());
        let improvement_tolerance = self
            .config
            .improve_threshold
            .filter(|value| *value > 0.0)
            .unwrap_or(1e-9f64);
        let mut callback = move |where_ctx: grb::callback::Where<'_>| -> grb::callback::CbResult {
            if pending_observer_terminate {
                if let grb::callback::Where::MIP(mip_ctx) = &where_ctx {
                    mip_ctx.terminate();
                }
            }

            let snapshot = if let grb::callback::Where::MIPSol(mip_ctx) = &where_ctx {
                let objective_value = mip_ctx.obj().ok().filter(|value| value.is_finite());
                if initial_objective_value.is_none() {
                    initial_objective_value = objective_value;
                }
                let best_bound = mip_ctx.obj_bnd().ok().filter(|value| value.is_finite());
                let node_count = mip_ctx
                    .node_cnt()
                    .ok()
                    .filter(|value| value.is_finite() && *value >= 0.0)
                    .map(|value| value as usize);
                let mip_gap =
                    objective_value
                        .zip(best_bound)
                        .map(|(objective_value, best_bound)| {
                            Self::compute_relative_gap(objective_value, best_bound)
                        });
                let incumbent_solution = callback_vars
                    .as_ref()
                    .and_then(|vars| mip_ctx.get_solution(vars).ok());
                GurobiNativeSnapshot {
                    where_point: GurobiNativeWhere::Mip,
                    solve_time: optimize_start_time.elapsed(),
                    objective_category,
                    initial_objective_value,
                    objective_value,
                    incumbent_solution,
                    best_bound,
                    mip_gap,
                    iterations: None,
                    node_count,
                }
            } else if let grb::callback::Where::MIP(mip_ctx) = &where_ctx {
                let objective_value = mip_ctx.obj_best().ok().filter(|value| value.is_finite());
                if initial_objective_value.is_none() {
                    initial_objective_value = objective_value;
                }
                let best_bound = mip_ctx.obj_bnd().ok().filter(|value| value.is_finite());
                let node_count = mip_ctx
                    .node_cnt()
                    .ok()
                    .filter(|value| value.is_finite() && *value >= 0.0)
                    .map(|value| value as usize);
                let iterations = mip_ctx
                    .iter_cnt()
                    .ok()
                    .filter(|value| value.is_finite() && *value >= 0.0)
                    .map(|value| value as usize);
                let mip_gap =
                    objective_value
                        .zip(best_bound)
                        .map(|(objective_value, best_bound)| {
                            Self::compute_relative_gap(objective_value, best_bound)
                        });
                GurobiNativeSnapshot {
                    where_point: GurobiNativeWhere::Mip,
                    solve_time: optimize_start_time.elapsed(),
                    objective_category,
                    initial_objective_value,
                    objective_value,
                    incumbent_solution: None,
                    best_bound,
                    mip_gap,
                    iterations,
                    node_count,
                }
            } else {
                GurobiNativeSnapshot {
                    where_point: GurobiNativeWhere::Other,
                    solve_time: optimize_start_time.elapsed(),
                    objective_category,
                    initial_objective_value,
                    objective_value: None,
                    incumbent_solution: None,
                    best_bound: None,
                    mip_gap: None,
                    iterations: None,
                    node_count: None,
                }
            };

            if let Some(callback) = telemetry_callback.as_ref() {
                if matches!(
                    &where_ctx,
                    grb::callback::Where::MIP(_) | grb::callback::Where::MIPSol(_)
                ) {
                    let should_emit = match telemetry_min_interval {
                        None => true,
                        Some(min_interval_seconds) => last_telemetry_emit_time
                            .map(|last_emit_time| {
                                last_emit_time.elapsed().as_secs_f64() >= min_interval_seconds
                            })
                            .unwrap_or(true),
                    };
                    if should_emit {
                        callback(&GurobiTelemetryStatus {
                            solve_time: snapshot.solve_time,
                            objective_category: snapshot.objective_category,
                            initial_objective_value: snapshot.initial_objective_value,
                            objective_value: snapshot.objective_value,
                            incumbent_solution: snapshot.incumbent_solution.clone(),
                            best_bound: snapshot.best_bound,
                            mip_gap: snapshot.mip_gap,
                            iterations: snapshot.iterations,
                            node_count: snapshot.node_count,
                        })
                        .map_err(|error| {
                            grb::Error::FromAPI(
                                format!("Gurobi telemetry callback error: {}", error),
                                40000,
                            )
                        })?;
                        last_telemetry_emit_time = Some(Instant::now());
                    }
                }
            }

            for observer in &native_observers {
                match observer(&snapshot).map_err(|error| {
                    grb::Error::FromAPI(
                        format!("Gurobi native observer callback error: {}", error),
                        40000,
                    )
                })? {
                    GurobiNativeControl::Continue => {}
                    GurobiNativeControl::Terminate => {
                        pending_observer_terminate = true;
                        if let grb::callback::Where::MIP(mip_ctx) = &where_ctx {
                            mip_ctx.terminate();
                        }
                    }
                }
            }

            if let Some(limit_seconds) = no_improvement_time_limit {
                if let grb::callback::Where::MIP(mip_ctx) = &where_ctx {
                    if let Ok(current_best) = mip_ctx.obj_best() {
                        if current_best.is_finite() {
                            let improved = match best_objective {
                                None => true,
                                Some(previous_best) => match objective_category {
                                    ObjectiveCategory::Minimum => {
                                        current_best < previous_best - improvement_tolerance
                                    }
                                    ObjectiveCategory::Maximum => {
                                        current_best > previous_best + improvement_tolerance
                                    }
                                },
                            };
                            if improved {
                                best_objective = Some(current_best);
                                last_improvement_time = Instant::now();
                            } else if last_improvement_time.elapsed().as_secs_f64() >= limit_seconds
                            {
                                mip_ctx.terminate();
                            }
                        }
                    }
                }
            }

            if let Some(callback) = native_callback.as_ref() {
                callback(where_ctx)?;
            }
            Ok(())
        };
        grb_model
            .optimize_with_callback(&mut callback)
            .map_err(|e| {
                crate::error::CoreError::Solver(crate::error::SolverError::SolveFailed(format!(
                    "Gurobi optimize_with_callback error: {}",
                    e
                )))
            })
    }

    /// 将 Gurobi 状态转换为求解器状态 / Convert Gurobi status to solver status
    pub(super) fn convert_status(status: grb::Status) -> SolverStatus {
        use grb::Status;
        match status {
            Status::Optimal => SolverStatus::Optimal,
            Status::Infeasible => SolverStatus::Infeasible,
            Status::InfOrUnbd => SolverStatus::InfeasibleOrUnbounded,
            Status::Unbounded => SolverStatus::Unbounded,
            Status::IterationLimit => SolverStatus::IterationLimit,
            Status::NodeLimit => SolverStatus::IterationLimit,
            Status::TimeLimit => SolverStatus::TimeLimit,
            Status::Numeric => SolverStatus::NumericError,
            Status::SubOptimal => SolverStatus::Feasible,
            _ => SolverStatus::Unknown,
        }
    }

    /// 结合解数量细化状态 / Refine status with solution availability.
    pub(super) fn refine_status_with_solution(
        status: SolverStatus,
        has_solution: bool,
    ) -> SolverStatus {
        if has_solution && matches!(status, SolverStatus::Unknown) {
            SolverStatus::Feasible
        } else {
            status
        }
    }

    pub(super) fn emit_stage_status(
        &self,
        stage: GurobiStage,
        solver_status: Option<SolverStatus>,
        solve_time: Duration,
        output: Option<&SolverOutput>,
    ) -> Result<()> {
        if let Some(callback) = self.config.stage_callback.as_ref() {
            callback(&GurobiStageStatus {
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

    /// 求解线性模型 / Solve linear model
    pub fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput> {
        linear::solve_linear(self, model)
    }

    /// 求解二次模型 / Solve quadratic model
    pub fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput> {
        quadratic::solve_quadratic(self, model)
    }

    /// 求解线性模型并返回解池 / Solve linear model and return solution pool
    pub fn solve_linear_multi(
        &self,
        model: &LinearTriadModel,
        solution_amount: usize,
    ) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
        linear::solve_linear_with_solution_pool(self, model, solution_amount)
    }

    /// 求解二次模型并返回解池 / Solve quadratic model and return solution pool
    pub fn solve_quadratic_multi(
        &self,
        model: &QuadraticTetradModel,
        solution_amount: usize,
    ) -> Result<(SolverOutput, Vec<Vec<f64>>)> {
        quadratic::solve_quadratic_with_solution_pool(self, model, solution_amount)
    }

    fn name_internal(&self) -> &str {
        "Gurobi"
    }

    fn capabilities_internal(&self) -> Vec<SolverCapability> {
        vec![
            SolverCapability::Linear,
            SolverCapability::Mip,
            SolverCapability::Quadratic,
            SolverCapability::MIQP,
            SolverCapability::SOCP,
            SolverCapability::NativeIndicator,
            SolverCapability::NativeSOS1,
        ]
    }
}

impl SolverInfo for GurobiSolver {
    fn name(&self) -> &str {
        self.name_internal()
    }

    fn capabilities(&self) -> Vec<SolverCapability> {
        self.capabilities_internal()
    }
}

impl LinearSolver for GurobiSolver {
    fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput> {
        GurobiSolver::solve_linear(self, model)
    }

    fn solve_linear_with_solution_pool(
        &self,
        model: &LinearTriadModel,
        solution_amount: usize,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        if solution_amount <= 1 {
            return Ok(None);
        }
        Ok(Some(GurobiSolver::solve_linear_multi(
            self,
            model,
            solution_amount,
        )?))
    }
}

impl QuadraticSolver for GurobiSolver {
    fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput> {
        GurobiSolver::solve_quadratic(self, model)
    }

    fn solve_quadratic_with_solution_pool(
        &self,
        model: &QuadraticTetradModel,
        solution_amount: usize,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        if solution_amount <= 1 {
            return Ok(None);
        }
        Ok(Some(GurobiSolver::solve_quadratic_multi(
            self,
            model,
            solution_amount,
        )?))
    }
}
