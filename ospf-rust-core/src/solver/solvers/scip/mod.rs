//! SCIP 姹傝В鍣ㄦ帴鍙?//! SCIP Solver Interface

//! 姝ゆā鍧椾粎鍦ㄥ惎鐢?`scip` feature 鏃跺彲鐢ㄣ€?//! This module is only available when the `scip` feature is enabled.

use std::time::{Duration, Instant};

use crate::error::{CoreError, Result, SolverError};
use crate::model::ConstraintRelation;
use crate::model::ObjectiveCategory;
use crate::model::intermediate::{LinearTriadModel, QuadraticTetradModel};
use crate::solver::{
    LinearSolver, QuadraticSolver, SolverCapability, SolverInfo, SolverOutput, SolverStatus,
};
use crate::variable::VariableType;
#[cfg(feature = "scip")]
use russcip::{
    Constraint, Event, EventMask, Eventhdlr, Model, ParamSetting, ProblemCreated, ProblemOrSolving,
    SCIPEventhdlr, Solving, Status as SCIPStatus, Unsolved, VarType, WithSolutions,
    WithSolvingStats,
};

mod callbacks;
mod config;

pub use callbacks::{
    SCIPNativeCallback, SCIPNativeControl, SCIPNativeObserver, SCIPNativeSnapshot,
    SCIPNativeWhere,
    SCIPSnapshotControl, SCIPSnapshotObserver, SCIPStage, SCIPStageCallback, SCIPStageStatus,
    SCIPTelemetryCallback, SCIPTelemetryStatus,
};
pub use config::{PresolvingMode, SCIPConfig};

#[cfg(feature = "scip")]
#[derive(Clone)]
struct SCIPTelemetryRuntime {
    objective_category: ObjectiveCategory,
    no_improvement_time_limit: Option<f64>,
    improvement_tolerance: f64,
    telemetry_min_interval: Option<f64>,
    telemetry_callback: Option<SCIPTelemetryCallback>,
    snapshot_observers: Vec<SCIPSnapshotObserver>,
    native_callback: Option<SCIPNativeCallback>,
    native_observers: Vec<SCIPNativeObserver>,
}

#[cfg(feature = "scip")]
impl SCIPTelemetryRuntime {
    fn enabled(&self) -> bool {
        self.no_improvement_time_limit.is_some()
            || self.telemetry_callback.is_some()
            || !self.snapshot_observers.is_empty()
            || self.native_callback.is_some()
            || !self.native_observers.is_empty()
    }
}

#[cfg(feature = "scip")]
struct SCIPTelemetryEventHandler {
    runtime: SCIPTelemetryRuntime,
    solve_started_at: Instant,
    last_improvement: Instant,
    last_telemetry_emit: Option<Instant>,
    best_objective: Option<f64>,
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

    fn build_snapshot(&self, model: &Model<Solving>) -> SCIPTelemetryStatus {
        let objective_value = model
            .best_sol()
            .map(|sol| sol.obj_val())
            .filter(|v| v.is_finite());
        let best_bound = {
            let bound = model.best_bound();
            if bound.is_finite() { Some(bound) } else { None }
        };
        let mip_gap = objective_value
            .zip(best_bound)
            .and_then(|(obj, bound)| SCIPSolver::relative_gap(obj, bound));
        SCIPTelemetryStatus {
            solve_time: self.solve_started_at.elapsed(),
            objective_value,
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
        EventMask::NODE_EVENT | EventMask::LP_EVENT | EventMask::SOL_EVENT
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
            objective_value: snapshot.objective_value,
            best_bound: snapshot.best_bound,
            mip_gap: snapshot.mip_gap,
            iterations: snapshot.iterations,
            node_count: snapshot.node_count,
        };
        self.check_improvement(snapshot.objective_value);

        if let Some(callback) = self.runtime.telemetry_callback.as_ref() {
            if self.should_emit_telemetry() {
                let _ = callback(&snapshot);
                self.last_telemetry_emit = Some(Instant::now());
            }
        }

        let mut request_interrupt = false;
        for observer in &self.runtime.snapshot_observers {
            if let Ok(control) = observer(&snapshot) {
                if matches!(control, SCIPSnapshotControl::Interrupt) {
                    request_interrupt = true;
                }
            }
        }
        for observer in &self.runtime.native_observers {
            if let Ok(control) = observer(&native_snapshot) {
                if matches!(control, SCIPNativeControl::Interrupt) {
                    request_interrupt = true;
                }
            }
        }
        if let Some(callback) = self.runtime.native_callback.as_ref() {
            if let Ok(control) = callback(&native_snapshot) {
                if matches!(control, SCIPNativeControl::Interrupt) {
                    request_interrupt = true;
                }
            }
        }

        if let Some(limit_seconds) = self.runtime.no_improvement_time_limit {
            if self.last_improvement.elapsed().as_secs_f64() >= limit_seconds {
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

/// SCIP 姹傝В鍣?/ SCIP Solver
///
/// 灏佽 SCIP 浼樺寲鍣ㄧ殑姹傝В鍣ㄥ疄鐜般€?/// Solver implementation wrapping SCIP optimizer.
///
/// # 鐗规€?/ Features
///
/// - 鏀寔 LP銆丮IP銆丵P锛堥€氳繃澶栭儴姹傝В鍣級
/// - 寮€婧愬厤璐?/// - 鏀寔绾︽潫鏁存暟瑙勫垝
///
/// # 鍓嶆彁鏉′欢 / Prerequisites
///
/// - 闇€瑕佸惎鐢?`scip` feature
///
/// # 绀轰緥 / Examples
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
    /// 閰嶇疆 / Configuration
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
    /// 鍒涘缓鏂扮殑 SCIP 姹傝В鍣?/ Create new SCIP solver
    pub fn new() -> Self {
        Self {
            config: SCIPConfig::default(),
        }
    }

    /// 鍒涘缓甯﹂厤缃殑姹傝В鍣?/ Create solver with configuration
    pub fn with_config(config: SCIPConfig) -> Self {
        Self { config }
    }

    /// 鑾峰彇閰嶇疆寮曠敤 / Get configuration reference
    pub fn config(&self) -> &SCIPConfig {
        &self.config
    }

    /// 鑾峰彇鍙彉閰嶇疆寮曠敤 / Get mutable configuration reference
    pub fn config_mut(&mut self) -> &mut SCIPConfig {
        &mut self.config
    }

    /// 灏?SCIP 鐘舵€佽浆鎹负姹傝В鍣ㄧ姸鎬?/ Convert SCIP status to solver status
    fn convert_status(status: SCIPStatus) -> SolverStatus {
        match status {
            SCIPStatus::Optimal => SolverStatus::Optimal,
            SCIPStatus::Infeasible => SolverStatus::Infeasible,
            SCIPStatus::Inforunbd => SolverStatus::InfeasibleOrUnbounded,
            SCIPStatus::Unbounded => SolverStatus::Unbounded,
            SCIPStatus::TimeLimit => SolverStatus::TimeLimit,
            SCIPStatus::NodeLimit
            | SCIPStatus::TotalNodeLimit
            | SCIPStatus::StallNodeLimit
            | SCIPStatus::GapLimit
            | SCIPStatus::SolutionLimit
            | SCIPStatus::BestSolutionLimit
            | SCIPStatus::RestartLimit => SolverStatus::IterationLimit,
            SCIPStatus::MemoryLimit => SolverStatus::NumericError,
            SCIPStatus::UserInterrupt => SolverStatus::UserInterrupt,
            _ => SolverStatus::Unknown,
        }
    }

    fn refine_status_with_solution(status: SolverStatus, has_solution: bool) -> SolverStatus {
        if has_solution && matches!(status, SolverStatus::Unknown) {
            return SolverStatus::Feasible;
        }
        status
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
    ) -> SCIPTelemetryRuntime {
        SCIPTelemetryRuntime {
            objective_category,
            no_improvement_time_limit: self.config.no_improvement_time_limit,
            improvement_tolerance: self.config.improvement_tolerance.unwrap_or(1e-9),
            telemetry_min_interval: self.config.telemetry_min_interval,
            telemetry_callback: self.config.telemetry_callback.clone(),
            snapshot_observers: self.config.snapshot_observers.clone(),
            native_callback: self.config.native_callback.clone(),
            native_observers: self.config.native_observers.clone(),
        }
    }

    fn install_telemetry_handler(
        &self,
        scip: &mut Model<ProblemCreated>,
        objective_category: ObjectiveCategory,
    ) {
        let runtime = self.build_telemetry_runtime(objective_category);
        if !runtime.enabled() {
            return;
        }
        scip.include_eventhdlr(
            "__ospf_scip_telemetry",
            "OSPF SCIP telemetry callback bridge",
            Box::new(SCIPTelemetryEventHandler::new(runtime)),
        );
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

    fn map_scip_error(context: &str, error: impl std::fmt::Debug) -> CoreError {
        CoreError::Solver(SolverError::SolveFailed(format!(
            "SCIP {} error: {:?}",
            context, error
        )))
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
        if let Some(iterations) = self.config.max_iterations {
            model = model
                .set_longint_param("lp/iterlim", iterations)
                .map_err(|e| Self::map_scip_error("set param", e))?;
        }
        if let Some(nodes) = self.config.node_limit {
            model = model
                .set_longint_param("limits/nodes", nodes)
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

    fn collect_dual_solution(constraints: &[Constraint]) -> Vec<f64> {
        constraints
            .iter()
            .map(|constraint: &Constraint| {
                constraint
                    .transformed()
                    .and_then(|transformed| transformed.dual_sol())
                    .or_else(|| constraint.dual_sol())
                    .unwrap_or(0.0)
            })
            .collect()
    }

    fn collect_farkas_solution(constraints: &[Constraint]) -> Vec<f64> {
        constraints
            .iter()
            .map(|constraint: &Constraint| {
                constraint
                    .transformed()
                    .and_then(|transformed| transformed.farkas_dual_sol())
                    .or_else(|| constraint.farkas_dual_sol())
                    .unwrap_or(0.0)
            })
            .collect()
    }

    fn relative_gap(objective_value: f64, best_bound: f64) -> Option<f64> {
        if !objective_value.is_finite() || !best_bound.is_finite() {
            return None;
        }
        let denominator = objective_value.abs().max(1.0);
        Some((objective_value - best_bound).abs() / denominator)
    }
}

#[cfg(feature = "scip")]
impl SCIPSolver {
    fn name(&self) -> &str {
        "SCIP"
    }

    fn capabilities(&self) -> Vec<SolverCapability> {
        vec![
            SolverCapability::Linear,
            SolverCapability::Mip,
            SolverCapability::NativeIndicator,
            SolverCapability::NativeSOS1,
            // SCIP 鐨?QP 鏀寔闇€瑕佸閮ㄦ眰瑙ｅ櫒锛堝 IPOPT锛?            #[cfg(feature = "scip-quadratic")]
            SolverCapability::Quadratic,
        ]
    }

    fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput> {
        let start_time = Instant::now();

        let mut scip = self.create_problem(model.objective_category)?;

        // 娣诲姞鍙橀噺
        let mut scip_vars = Vec::with_capacity(model.num_variables());
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
            let var = scip.add_var(lb, ub, obj, &token.variable.name(), vtype);

            scip_vars.push(var);
        }

        // 娣诲姞绾︽潫: Ax <= b
        let mut linear_constraints: Vec<Constraint> = Vec::with_capacity(model.num_constraints());
        for i in 0..model.num_constraints() {
            let mut vars = Vec::new();
            let mut values = Vec::new();

            // 浠庣█鐤忕煩闃垫彁鍙栫害鏉熻 / Build sparse row terms from triad matrix row
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
            linear_constraints.push(constraint);
        }

        self.emit_stage_status(SCIPStage::AfterModeling, None, start_time.elapsed(), None)?;
        self.install_telemetry_handler(&mut scip, model.objective_category);
        self.emit_stage_status(SCIPStage::Configuration, None, start_time.elapsed(), None)?;

        let solved = scip.solve();
        let solution = solved.best_sol();
        let solver_status = Self::refine_status_with_solution(
            Self::convert_status(solved.status()),
            solution.is_some(),
        );

        let mut output = SolverOutput::new(solver_status);
        output.node_count = Some(solved.n_nodes());
        output.iterations = Some(solved.n_lp_iterations());

        let best_bound = solved.best_bound();
        if best_bound.is_finite() {
            output.best_bound = Some(best_bound);
        }

        if let Some(solution) = solution {
            let objective_value = solution.obj_val();
            output.objective_value = Some(objective_value);
            output.solution = Some(scip_vars.iter().map(|var| solution.val(var)).collect());
            if let Some(best_bound) = output.best_bound {
                output.mip_gap = Self::relative_gap(objective_value, best_bound);
            }
        }

        // 绾挎€у鍋朵箻瀛愶紙鏈€浣冲姫鍔涳級/ Linear dual multipliers (best effort)
        if matches!(solver_status, SolverStatus::Optimal) {
            let dual_solution = Self::collect_dual_solution(&linear_constraints);
            if !dual_solution.is_empty() {
                output.dual_solution = Some(dual_solution);
            }
        }

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
        Ok(output)
    }

    fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput> {
        let start_time = Instant::now();

        let mut scip = self.create_problem(model.objective_category)?;

        // 娣诲姞鍙橀噺
        let mut scip_vars = Vec::with_capacity(model.num_variables());
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
            let var = scip.add_var(lb, ub, obj, &token.variable.name(), vtype);

            scip_vars.push(var);
        }

        // 閫氳繃寮曞叆杈呭姪鍙橀噺鎶婁簩娆＄洰鏍囪浆鎴愪簩娆＄害鏉?/ Transform quadratic objective via auxiliary-variable quadratic constraint
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
                    return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                        "quadratic objective row index {} out of bounds for {} variables",
                        i,
                        scip_vars.len()
                    ))));
                }
                for &(j, qval) in row.entries.iter() {
                    if qval.abs() <= f64::EPSILON {
                        continue;
                    }
                    if j >= scip_vars.len() {
                        return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                            "quadratic objective column index {} out of bounds for {} variables",
                            j,
                            scip_vars.len()
                        ))));
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
                    return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                        "quadratic constraint {} references invalid variable index {}",
                        i, var_index1
                    ))));
                }
                let coefficient = *monomial.coefficient();
                if coefficient.abs() <= f64::EPSILON {
                    continue;
                }
                if let Some(var_index2) = monomial.var_index2() {
                    if var_index2 >= scip_vars.len() {
                        return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                            "quadratic constraint {} references invalid variable index {}",
                            i, var_index2
                        ))));
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

        // 娣诲姞绾︽潫
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
            linear_constraints.push(constraint);
        }

        self.emit_stage_status(SCIPStage::AfterModeling, None, start_time.elapsed(), None)?;
        self.install_telemetry_handler(&mut scip, model.objective_category);
        self.emit_stage_status(SCIPStage::Configuration, None, start_time.elapsed(), None)?;

        let solved = scip.solve();
        let solution = solved.best_sol();
        let solver_status = Self::refine_status_with_solution(
            Self::convert_status(solved.status()),
            solution.is_some(),
        );

        let mut output = SolverOutput::new(solver_status);
        output.node_count = Some(solved.n_nodes());
        output.iterations = Some(solved.n_lp_iterations());

        let best_bound = solved.best_bound();
        if best_bound.is_finite() {
            output.best_bound = Some(best_bound);
        }

        if let Some(solution) = solution {
            let objective_value = solution.obj_val();
            output.objective_value = Some(objective_value);
            output.solution = Some(scip_vars.iter().map(|var| solution.val(var)).collect());
            if let Some(best_bound) = output.best_bound {
                output.mip_gap = Self::relative_gap(objective_value, best_bound);
            }
        }

        // 绾挎€х害鏉熷鍋跺€硷紙鐢ㄤ簬浜屾瀛愰棶棰樻渶浼樻€?cut锛? Linear-row duals for quadratic-subproblem optimality cuts
        if matches!(solver_status, SolverStatus::Optimal) {
            let dual_solution = Self::collect_dual_solution(&linear_constraints);
            if !dual_solution.is_empty() {
                output.dual_solution = Some(dual_solution);
            }
        }

        if solver_status.is_infeasible() {
            // 绾挎€х害鏉?Farkas 涔樺瓙锛堢敤浜庝簩娆″瓙闂鍙鎬?cut锛? Linear-row Farkas duals for quadratic-subproblem feasibility cuts
            let farkas_solution = Self::collect_farkas_solution(&linear_constraints);
            if !farkas_solution.is_empty() {
                output.dual_solution = Some(farkas_solution);
            }
        }

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
        Ok(output)
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
}

#[cfg(feature = "scip")]
impl LinearSolver for SCIPSolver {
    fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput> {
        SCIPSolver::solve_linear(self, model)
    }
}

#[cfg(feature = "scip")]
impl QuadraticSolver for SCIPSolver {
    fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput> {
        SCIPSolver::solve_quadratic(self, model)
    }
}

#[cfg(all(test, feature = "scip"))]
mod tests {
    use super::*;
    use crate::flatten::{Quadratic, QuadraticMonomial};
    use crate::model::intermediate::{BasicQuadraticTetradModel, SparseMatrix, SparseVector};
    use crate::model::{ConstraintRelation, ObjectiveCategory};
    use crate::token::Token;
    use crate::variable::UContinuousVariableItem;
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
            .with_improvement_tolerance(1e-6)
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
        assert_eq!(config.improvement_tolerance, Some(1e-6));
        assert_eq!(config.telemetry_min_interval, Some(0.2));
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
            objective_value: Some(1.0),
            best_bound: Some(0.8),
            mip_gap: Some(0.2),
            iterations: Some(10),
            node_count: Some(5),
        })
        .expect("telemetry callback should execute");
        assert_eq!(telemetry_counter.load(Ordering::SeqCst), 1);

        let control = (config.snapshot_observers[0])(&SCIPTelemetryStatus {
            solve_time: Duration::ZERO,
            objective_value: Some(1.0),
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
        let native_callback: SCIPNativeCallback =
            Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let native_observer: SCIPNativeObserver =
            Arc::new(|_| Ok(SCIPNativeControl::Continue));

        let config = SCIPConfig::new()
            .with_native_callback(Some(native_callback))
            .add_native_observer(native_observer);

        assert!(config.native_callback.is_some());
        assert_eq!(config.native_observers.len(), 1);
    }

    #[test]
    fn test_native_callback_override_and_observer_append_semantics() {
        let first_callback: SCIPNativeCallback =
            Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let second_callback: SCIPNativeCallback =
            Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let first_observer: SCIPNativeObserver =
            Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let second_observer: SCIPNativeObserver =
            Arc::new(|_| Ok(SCIPNativeControl::Continue));

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
            objective_value: None,
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
