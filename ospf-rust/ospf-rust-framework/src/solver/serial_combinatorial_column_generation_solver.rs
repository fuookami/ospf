//! 串行组合列生成求解器
//! Serial Combinatorial Column Generation Solver
//!
//! 本模块提供串行执行的组合列生成求解器。
//! This module provides serial-executing combinatorial column generation solvers.

use super::column_generation_solver::{
    CombinatorialFallbackPolicy, CombinatorialSelection,
    aggregate_combinatorial_reports_with_metadata, emit_column_generation_progress,
    preserve_cancelled_attempts,
};
use super::column_generation_solver::{RegistrationStatusCallback, SolvingStatusCallback};
use super::framework_solve_options::child_cancellation_handle;
use super::{
    ColumnGenerationSolver, FeasibleSolution, FrameworkSolveOptions, LPResult,
    legacy_cancellation_error,
};
use ospf_rust_core::error::{CoreError, Result, SolverError, SolverNotFoundError};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{
    CombinatorialSolveReport, ProblemStatus, ProgressValue, SolveReport, SolverProvenance,
    cancelled_solve_report,
};
use std::sync::Arc;
use std::time::Instant;

/// 串行组合列生成求解器 / Serial Combinatorial Column Generation Solver
///
/// 按顺序尝试多个求解器，返回第一个成功的结果。
/// Tries multiple solvers in sequence, returning the first successful result.
pub struct SerialCombinatorialColumnGenerationSolver {
    /// 求解器列表 / Solver list
    solvers: Vec<Arc<dyn ColumnGenerationSolver>>,
    /// 名称缓存 / Cached name
    name: String,
}

impl SerialCombinatorialColumnGenerationSolver {
    /// 创建新的串行组合列生成求解器 / Create new serial combinatorial column generation solver
    pub fn new(solvers: Vec<Arc<dyn ColumnGenerationSolver>>) -> Self {
        let names: Vec<&str> = solvers.iter().map(|s| s.name()).collect();
        let name = format!("SerialCombinatorial({})", names.join(","));
        Self { solvers, name }
    }

    /// 使用默认配置创建 / Create with default configuration
    pub fn with_solvers(solvers: Vec<Arc<dyn ColumnGenerationSolver>>) -> Self {
        Self::new(solvers)
    }

    /// 获取求解器名称 / Get solver name
    pub fn name(&self) -> &str {
        &self.name
    }

    #[cfg(test)]
    fn should_stop_on_error(error: &CoreError) -> bool {
        Self::should_stop_on_error_with_policy(error, &CombinatorialFallbackPolicy::default())
    }

    fn should_stop_on_error_with_policy(
        error: &CoreError,
        policy: &CombinatorialFallbackPolicy,
    ) -> bool {
        policy.should_stop_on_error(error)
    }

    fn should_stop_on_report_with_policy(
        report: &SolveReport<f64>,
        policy: &CombinatorialFallbackPolicy,
    ) -> bool {
        policy.should_stop_on_report(report)
    }

    fn cancellation_requested(options: &FrameworkSolveOptions) -> bool {
        options
            .cancellation_handle
            .as_ref()
            .is_some_and(|handle| handle.is_cancelled())
    }

    fn should_stop_on_legacy_error(error: &CoreError, options: &FrameworkSolveOptions) -> bool {
        Self::should_stop_on_error_with_policy(error, &options.fallback_policy)
            || Self::cancellation_requested(options)
    }

    fn wrap_solving_status_callback(
        callback: Option<SolvingStatusCallback>,
        solver_name: String,
        solver_index: usize,
    ) -> Option<SolvingStatusCallback> {
        callback.map(|callback| {
            Arc::new(
                move |status: &super::column_generation_solver::SolvingStatus| {
                    let mut mapped = status.clone();
                    mapped.solver = solver_name.clone();
                    mapped.solver_index = solver_index;
                    callback(&mapped)
                },
            ) as SolvingStatusCallback
        })
    }

    fn wrap_registration_status_callback(
        callback: Option<RegistrationStatusCallback>,
        solver_name: String,
    ) -> Option<RegistrationStatusCallback> {
        callback.map(|callback| {
            Arc::new(
                move |status: &super::column_generation_solver::RegistrationStatus| {
                    let mut mapped = status.clone();
                    mapped.solver = solver_name.clone();
                    callback(&mapped)
                },
            ) as RegistrationStatusCallback
        })
    }

    fn pre_cancelled_report(
        &self,
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
                solver_id: self.name.clone(),
                backend_name: "framework-combinatorial".to_owned(),
                ..SolverProvenance::default()
            },
            handle,
        )?;
        Ok(Some(CombinatorialSolveReport::single(
            report,
            format!("{}#0", self.name),
        )))
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl ColumnGenerationSolver for SerialCombinatorialColumnGenerationSolver {
    fn name(&self) -> &str {
        &self.name
    }

    async fn solve_milp_combinatorial_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = self.pre_cancelled_report(&options)? {
            return Ok(report);
        }
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        let begin = Instant::now();
        let mut attempts = Vec::new();
        let mut child_handles = Vec::new();
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            emit_column_generation_progress(
                &options,
                Some(solver_index),
                "dispatch",
                ProgressValue::known(0.0)?,
                ProgressValue::indeterminate(),
                begin.elapsed(),
                None,
                None,
                None,
                false,
            )?;
            let solver_name = solver.name().to_owned();
            let registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name.clone(),
                solver_index,
            );
            let child_handle = child_cancellation_handle(&options);
            let attempt_options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle.clone()));
            let result = solver
                .solve_milp_report_with_options(model, attempt_options)
                .await;
            child_handle.mark_completed();
            let should_stop = child_handle.cancellation_preceded_completion()
                || match &result {
                    Ok(report) => {
                        Self::should_stop_on_report_with_policy(report, &options.fallback_policy)
                    }
                    Err(error) => {
                        Self::should_stop_on_error_with_policy(error, &options.fallback_policy)
                            || Self::cancellation_requested(&options)
                    }
                };
            child_handles.push(child_handle);
            attempts.push((solver_index, solver_name, result));
            if should_stop {
                break;
            }
        }
        let aggregate = aggregate_combinatorial_reports_with_metadata(
            self.name(),
            CombinatorialSelection::First,
            preserve_cancelled_attempts(attempts, &child_handles),
        )?;
        emit_column_generation_progress(
            &options,
            None,
            "completed",
            ProgressValue::known(100.0)?,
            ProgressValue::known(100.0)?,
            begin.elapsed(),
            aggregate
                .report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective_value.or(solution.objective)),
            aggregate.report.statistics.best_bound_value,
            aggregate.report.statistics.relative_gap,
            true,
        )?;
        Ok(aggregate)
    }

    async fn solve_lp_combinatorial_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = self.pre_cancelled_report(&options)? {
            return Ok(report);
        }
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        let begin = Instant::now();
        let mut attempts = Vec::new();
        let mut child_handles = Vec::new();
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            emit_column_generation_progress(
                &options,
                Some(solver_index),
                "dispatch",
                ProgressValue::known(0.0)?,
                ProgressValue::indeterminate(),
                begin.elapsed(),
                None,
                None,
                None,
                false,
            )?;
            let solver_name = solver.name().to_owned();
            let registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name.clone(),
                solver_index,
            );
            let child_handle = child_cancellation_handle(&options);
            let attempt_options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle.clone()));
            let result = solver
                .solve_lp_report_with_options(model, attempt_options)
                .await;
            child_handle.mark_completed();
            let should_stop = child_handle.cancellation_preceded_completion()
                || match &result {
                    Ok(report) => {
                        Self::should_stop_on_report_with_policy(report, &options.fallback_policy)
                    }
                    Err(error) => {
                        Self::should_stop_on_error_with_policy(error, &options.fallback_policy)
                            || Self::cancellation_requested(&options)
                    }
                };
            child_handles.push(child_handle);
            attempts.push((solver_index, solver_name, result));
            if should_stop {
                break;
            }
        }
        let aggregate = aggregate_combinatorial_reports_with_metadata(
            self.name(),
            CombinatorialSelection::First,
            preserve_cancelled_attempts(attempts, &child_handles),
        )?;
        emit_column_generation_progress(
            &options,
            None,
            "completed",
            ProgressValue::known(100.0)?,
            ProgressValue::known(100.0)?,
            begin.elapsed(),
            aggregate
                .report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective_value.or(solution.objective)),
            aggregate.report.statistics.best_bound_value,
            aggregate.report.statistics.relative_gap,
            true,
        )?;
        Ok(aggregate)
    }

    async fn solve_milp_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        if Self::cancellation_requested(&options) {
            return Err(legacy_cancellation_error(&options));
        }
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver_name = solver.name().to_string();
            let wrapped_registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let wrapped_solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let attempt_options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(wrapped_registration_callback)
                .with_solving_callback(wrapped_solving_callback);
            match solver.solve_milp_with_options(model, attempt_options).await {
                Ok(solution) => {
                    log::info!("Solver {} found a solution.", solver.name());
                    return Ok(solution);
                }
                Err(e) => {
                    if Self::should_stop_on_legacy_error(&e, &options) {
                        return Err(e);
                    }
                    log::warn!("Solver {} failed with error: {}", solver.name(), e);
                }
            }
        }
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid.",
        )))
    }

    async fn solve_lp_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<LPResult> {
        if Self::cancellation_requested(&options) {
            return Err(legacy_cancellation_error(&options));
        }
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver_name = solver.name().to_string();
            let wrapped_registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let wrapped_solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let attempt_options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(wrapped_registration_callback)
                .with_solving_callback(wrapped_solving_callback);
            match solver.solve_lp_with_options(model, attempt_options).await {
                Ok(result) => {
                    log::info!("Solver {} found a solution.", solver.name());
                    return Ok(result);
                }
                Err(e) => {
                    if Self::should_stop_on_legacy_error(&e, &options) {
                        return Err(e);
                    }
                    log::warn!("Solver {} failed with error: {}", solver.name(), e);
                }
            }
        }
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid.",
        )))
    }
}

#[cfg(not(feature = "async"))]
impl ColumnGenerationSolver for SerialCombinatorialColumnGenerationSolver {
    fn name(&self) -> &str {
        &self.name
    }

    fn solve_milp_combinatorial_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = self.pre_cancelled_report(&options)? {
            return Ok(report);
        }
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        let begin = Instant::now();
        let mut attempts = Vec::new();
        let mut child_handles = Vec::new();
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            emit_column_generation_progress(
                &options,
                Some(solver_index),
                "dispatch",
                ProgressValue::known(0.0)?,
                ProgressValue::indeterminate(),
                begin.elapsed(),
                None,
                None,
                None,
                false,
            )?;
            let solver_name = solver.name().to_owned();
            let registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name.clone(),
                solver_index,
            );
            let child_handle = child_cancellation_handle(&options);
            let attempt_options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle.clone()));
            let result = solver.solve_milp_report_with_options(model, attempt_options);
            child_handle.mark_completed();
            let should_stop = child_handle.cancellation_preceded_completion()
                || match &result {
                    Ok(report) => {
                        Self::should_stop_on_report_with_policy(report, &options.fallback_policy)
                    }
                    Err(error) => {
                        Self::should_stop_on_error_with_policy(error, &options.fallback_policy)
                            || Self::cancellation_requested(&options)
                    }
                };
            child_handles.push(child_handle);
            attempts.push((solver_index, solver_name, result));
            if should_stop {
                break;
            }
        }
        let aggregate = aggregate_combinatorial_reports_with_metadata(
            self.name(),
            CombinatorialSelection::First,
            preserve_cancelled_attempts(attempts, &child_handles),
        )?;
        emit_column_generation_progress(
            &options,
            None,
            "completed",
            ProgressValue::known(100.0)?,
            ProgressValue::known(100.0)?,
            begin.elapsed(),
            aggregate
                .report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective_value.or(solution.objective)),
            aggregate.report.statistics.best_bound_value,
            aggregate.report.statistics.relative_gap,
            true,
        )?;
        Ok(aggregate)
    }

    fn solve_lp_combinatorial_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = self.pre_cancelled_report(&options)? {
            return Ok(report);
        }
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        let begin = Instant::now();
        let mut attempts = Vec::new();
        let mut child_handles = Vec::new();
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            emit_column_generation_progress(
                &options,
                Some(solver_index),
                "dispatch",
                ProgressValue::known(0.0)?,
                ProgressValue::indeterminate(),
                begin.elapsed(),
                None,
                None,
                None,
                false,
            )?;
            let solver_name = solver.name().to_owned();
            let registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name.clone(),
                solver_index,
            );
            let child_handle = child_cancellation_handle(&options);
            let attempt_options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle.clone()));
            let result = solver.solve_lp_report_with_options(model, attempt_options);
            child_handle.mark_completed();
            let should_stop = child_handle.cancellation_preceded_completion()
                || match &result {
                    Ok(report) => {
                        Self::should_stop_on_report_with_policy(report, &options.fallback_policy)
                    }
                    Err(error) => {
                        Self::should_stop_on_error_with_policy(error, &options.fallback_policy)
                            || Self::cancellation_requested(&options)
                    }
                };
            child_handles.push(child_handle);
            attempts.push((solver_index, solver_name, result));
            if should_stop {
                break;
            }
        }
        let aggregate = aggregate_combinatorial_reports_with_metadata(
            self.name(),
            CombinatorialSelection::First,
            preserve_cancelled_attempts(attempts, &child_handles),
        )?;
        emit_column_generation_progress(
            &options,
            None,
            "completed",
            ProgressValue::known(100.0)?,
            ProgressValue::known(100.0)?,
            begin.elapsed(),
            aggregate
                .report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective_value.or(solution.objective)),
            aggregate.report.statistics.best_bound_value,
            aggregate.report.statistics.relative_gap,
            true,
        )?;
        Ok(aggregate)
    }

    fn solve_milp_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        if Self::cancellation_requested(&options) {
            return Err(legacy_cancellation_error(&options));
        }
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver_name = solver.name().to_string();
            let wrapped_registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let wrapped_solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let attempt_options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(wrapped_registration_callback)
                .with_solving_callback(wrapped_solving_callback);
            match solver.solve_milp_with_options(model, attempt_options) {
                Ok(solution) => {
                    log::info!("Solver {} found a solution.", solver.name());
                    return Ok(solution);
                }
                Err(e) => {
                    if Self::should_stop_on_legacy_error(&e, &options) {
                        return Err(e);
                    }
                    log::warn!("Solver {} failed with error: {}", solver.name(), e);
                }
            }
        }
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid.",
        )))
    }

    fn solve_lp_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<LPResult> {
        if Self::cancellation_requested(&options) {
            return Err(legacy_cancellation_error(&options));
        }
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver_name = solver.name().to_string();
            let wrapped_registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let wrapped_solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let attempt_options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(wrapped_registration_callback)
                .with_solving_callback(wrapped_solving_callback);
            match solver.solve_lp_with_options(model, attempt_options) {
                Ok(result) => {
                    log::info!("Solver {} found a solution.", solver.name());
                    return Ok(result);
                }
                Err(e) => {
                    if Self::should_stop_on_legacy_error(&e, &options) {
                        return Err(e);
                    }
                    log::warn!("Solver {} failed with error: {}", solver.name(), e);
                }
            }
        }
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid.",
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::LinearDualSolution;
    #[cfg(not(feature = "async"))]
    use crate::solver::column_generation_solver::{RegistrationStatus, SolvingStatus};
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::{ConstraintRelation, MetaModel};
    use ospf_rust_core::solver::{
        CancellationOrigin, SolveAttemptOutcome, SolveHandle, SolveProgressReporter,
        SolveProgressSnapshot, SolveStage, TerminationReason,
    };
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::variable::ContinuousVariableItem;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockSolver {
        name: String,
        should_fail: bool,
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl ColumnGenerationSolver for MockSolver {
        fn name(&self) -> &str {
            &self.name
        }

        #[cfg(feature = "async")]
        async fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            if self.should_fail {
                Err(CoreError::Solver(SolverError::SolveFailed(
                    "Mock error".into(),
                )))
            } else {
                Ok(FeasibleSolution::new(1.0, vec![1.0]))
            }
        }

        #[cfg(not(feature = "async"))]
        fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            if self.should_fail {
                Err(CoreError::Solver(SolverError::SolveFailed(
                    "Mock error".into(),
                )))
            } else {
                if let Some(callback) = options.registration_status_callback {
                    callback(&RegistrationStatus::new(self.name.clone(), 1, 1, 1))?;
                }
                if let Some(callback) = options.solving_status_callback {
                    callback(&SolvingStatus::new(self.name.clone(), 0, 1.0))?;
                }
                Ok(FeasibleSolution::new(1.0, vec![1.0]))
            }
        }

        #[cfg(feature = "async")]
        async fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            if self.should_fail {
                Err(CoreError::Solver(SolverError::SolveFailed(
                    "Mock error".into(),
                )))
            } else {
                Ok(LPResult::new(
                    FeasibleSolution::new(1.0, vec![1.0]),
                    LinearDualSolution::default(),
                ))
            }
        }

        #[cfg(not(feature = "async"))]
        fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            if self.should_fail {
                Err(CoreError::Solver(SolverError::SolveFailed(
                    "Mock error".into(),
                )))
            } else {
                if let Some(callback) = options.registration_status_callback {
                    callback(&RegistrationStatus::new(self.name.clone(), 1, 1, 1))?;
                }
                if let Some(callback) = options.solving_status_callback {
                    callback(&SolvingStatus::new(self.name.clone(), 0, 1.0))?;
                }
                Ok(LPResult::new(
                    FeasibleSolution::new(1.0, vec![1.0]),
                    LinearDualSolution::default(),
                ))
            }
        }
    }

    struct CountingMockSolver {
        calls: Arc<AtomicUsize>,
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl ColumnGenerationSolver for CountingMockSolver {
        fn name(&self) -> &str {
            "counting-mock"
        }

        #[cfg(feature = "async")]
        async fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(FeasibleSolution::new(0.0, Vec::new()))
        }

        #[cfg(not(feature = "async"))]
        fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(FeasibleSolution::new(0.0, Vec::new()))
        }

        #[cfg(feature = "async")]
        async fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(LPResult::new(
                FeasibleSolution::new(0.0, Vec::new()),
                LinearDualSolution::default(),
            ))
        }

        #[cfg(not(feature = "async"))]
        fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(LPResult::new(
                FeasibleSolution::new(0.0, Vec::new()),
                LinearDualSolution::default(),
            ))
        }
    }

    struct CancelledReportMockSolver {
        name: String,
        calls: Arc<AtomicUsize>,
        cancelled: bool,
        cancel_on_success: bool,
    }

    fn cancelled_report() -> SolveReport<f64> {
        SolveReport::builder(ProblemStatus::Unknown, TerminationReason::Cancelled)
            .build()
            .expect("cancelled report should be valid")
    }

    #[test]
    fn terminal_errors_stop_the_column_generation_fallback_chain() {
        for error in [
            CoreError::Solver(SolverError::Cancelled("backend".to_owned())),
            CoreError::Solver(SolverError::Interrupted("external".to_owned())),
            CoreError::Solver(SolverError::Timeout(std::time::Duration::from_secs(1))),
        ] {
            assert!(SerialCombinatorialColumnGenerationSolver::should_stop_on_error(&error));
        }
        assert!(
            !SerialCombinatorialColumnGenerationSolver::should_stop_on_error(&CoreError::Solver(
                SolverError::SolveFailed("fallback".to_owned())
            ))
        );
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl ColumnGenerationSolver for CancelledReportMockSolver {
        fn name(&self) -> &str {
            &self.name
        }

        #[cfg(feature = "async")]
        async fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(1.0, vec![1.0]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(1.0, vec![1.0]))
        }

        #[cfg(feature = "async")]
        async fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            Ok(LPResult::new(
                FeasibleSolution::new(1.0, vec![1.0]),
                LinearDualSolution::default(),
            ))
        }

        #[cfg(not(feature = "async"))]
        fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            Ok(LPResult::new(
                FeasibleSolution::new(1.0, vec![1.0]),
                LinearDualSolution::default(),
            ))
        }

        #[cfg(feature = "async")]
        async fn solve_milp_report_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<SolveReport<f64>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.cancelled {
                Ok(cancelled_report())
            } else {
                if self.cancel_on_success {
                    let handle = options
                        .cancellation_handle
                        .as_ref()
                        .expect("serial attempt should receive a child cancellation handle");
                    assert!(handle.cancel(ospf_rust_core::solver::CancellationOrigin::Callback));
                }
                FeasibleSolution::new(1.0, vec![1.0]).to_solve_report(&self.name)
            }
        }

        #[cfg(not(feature = "async"))]
        fn solve_milp_report_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<SolveReport<f64>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.cancelled {
                Ok(cancelled_report())
            } else {
                if self.cancel_on_success {
                    let handle = options
                        .cancellation_handle
                        .as_ref()
                        .expect("serial attempt should receive a child cancellation handle");
                    assert!(handle.cancel(ospf_rust_core::solver::CancellationOrigin::Callback));
                }
                FeasibleSolution::new(1.0, vec![1.0]).to_solve_report(&self.name)
            }
        }
    }

    fn assert_pre_cancelled_report(aggregate: &CombinatorialSolveReport<f64>, solver_name: &str) {
        aggregate
            .validate()
            .expect("pre-cancelled aggregate should validate");
        assert_eq!(aggregate.report.problem_status, ProblemStatus::Unknown);
        assert_eq!(
            aggregate.report.termination_reason,
            TerminationReason::Cancelled
        );
        assert!(!aggregate.report.has_incumbent());
        assert_eq!(aggregate.attempts.len(), 1);
        assert_eq!(
            aggregate.attempts[0].outcome,
            SolveAttemptOutcome::Cancelled
        );
        assert_eq!(
            aggregate.attempts[0].cancellation_reason.as_deref(),
            Some("USER")
        );
        let expected_attempt_id = format!("{}#0", solver_name);
        assert_eq!(
            aggregate.selected_attempt_id.as_deref(),
            Some(expected_attempt_id.as_str())
        );
    }

    fn assert_column_generation_progress(snapshots: &Arc<Mutex<Vec<SolveProgressSnapshot>>>) {
        let snapshots = snapshots
            .lock()
            .expect("column-generation progress snapshots should not be poisoned");
        assert!(snapshots.iter().any(|snapshot| {
            snapshot.stage == SolveStage::Combinatorial
                && snapshot.stage_path
                    == vec![
                        "column-generation".to_owned(),
                        "attempt/0".to_owned(),
                        "dispatch".to_owned(),
                    ]
        }));
        assert!(snapshots.iter().any(|snapshot| {
            snapshot.stage == SolveStage::Completed
                && snapshot.stage_path
                    == vec!["column-generation".to_owned(), "completed".to_owned()]
                && snapshot.terminal
        }));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn report_entries_publish_hierarchical_column_generation_progress() {
        let snapshots = Arc::new(Mutex::new(Vec::<SolveProgressSnapshot>::new()));
        let snapshots_for_reporter = Arc::clone(&snapshots);
        let reporter: SolveProgressReporter = Arc::new(move |snapshot| {
            snapshots_for_reporter
                .lock()
                .expect("column-generation progress mutex should not be poisoned")
                .push(snapshot.clone());
            Ok(())
        });
        let solver =
            SerialCombinatorialColumnGenerationSolver::with_solvers(vec![Arc::new(MockSolver {
                name: "progress-mock".to_owned(),
                should_fail: false,
            })]);
        solver
            .solve_milp_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_progress"),
                FrameworkSolveOptions::default().with_progress_reporter(Some(reporter)),
            )
            .expect("column-generation report should succeed");
        assert_column_generation_progress(&snapshots);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn report_entries_publish_hierarchical_column_generation_progress() {
        let snapshots = Arc::new(Mutex::new(Vec::<SolveProgressSnapshot>::new()));
        let snapshots_for_reporter = Arc::clone(&snapshots);
        let reporter: SolveProgressReporter = Arc::new(move |snapshot| {
            snapshots_for_reporter
                .lock()
                .expect("column-generation progress mutex should not be poisoned")
                .push(snapshot.clone());
            Ok(())
        });
        let solver =
            SerialCombinatorialColumnGenerationSolver::with_solvers(vec![Arc::new(MockSolver {
                name: "progress-mock".to_owned(),
                should_fail: false,
            })]);
        solver
            .solve_milp_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_progress"),
                FrameworkSolveOptions::default().with_progress_reporter(Some(reporter)),
            )
            .await
            .expect("column-generation report should succeed");
        assert_column_generation_progress(&snapshots);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn report_entries_honor_pre_cancelled_handle_without_starting_solver() {
        let calls = Arc::new(AtomicUsize::new(0));
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(vec![Arc::new(
            CountingMockSolver {
                calls: Arc::clone(&calls),
            },
        )]);
        let model = LinearTriadModel::new("serial_pre_cancelled");
        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::User));
        let options = FrameworkSolveOptions::new().with_cancellation_handle(Some(handle));

        let milp = solver
            .solve_milp_combinatorial_report_with_options(&model, options.clone())
            .expect("pre-cancelled MILP should produce a report");
        let lp = solver
            .solve_lp_combinatorial_report_with_options(&model, options)
            .expect("pre-cancelled LP should produce a report");

        assert_pre_cancelled_report(&milp, solver.name());
        assert_pre_cancelled_report(&lp, solver.name());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn report_entries_honor_pre_cancelled_handle_without_starting_solver() {
        let calls = Arc::new(AtomicUsize::new(0));
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(vec![Arc::new(
            CountingMockSolver {
                calls: Arc::clone(&calls),
            },
        )]);
        let model = LinearTriadModel::new("serial_pre_cancelled");
        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::User));
        let options = FrameworkSolveOptions::new().with_cancellation_handle(Some(handle));

        let milp = solver
            .solve_milp_combinatorial_report_with_options(&model, options.clone())
            .await
            .expect("pre-cancelled MILP should produce a report");
        let lp = solver
            .solve_lp_combinatorial_report_with_options(&model, options)
            .await
            .expect("pre-cancelled LP should produce a report");

        assert_pre_cancelled_report(&milp, solver.name());
        assert_pre_cancelled_report(&lp, solver.name());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn report_cancellation_stops_the_fallback_chain() {
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(vec![
            Arc::new(CancelledReportMockSolver {
                name: "cancelled".to_owned(),
                calls: Arc::clone(&first_calls),
                cancelled: true,
                cancel_on_success: false,
            }),
            Arc::new(CancelledReportMockSolver {
                name: "fallback".to_owned(),
                calls: Arc::clone(&second_calls),
                cancelled: false,
                cancel_on_success: false,
            }),
        ]);

        let aggregate = solver
            .solve_milp_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_cancelled_report"),
                FrameworkSolveOptions::default(),
            )
            .expect("cancelled report should be aggregated");
        assert_eq!(
            aggregate.report.termination_reason,
            TerminationReason::Cancelled
        );
        assert_eq!(aggregate.attempts.len(), 1);
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn report_cancellation_stops_the_fallback_chain() {
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(vec![
            Arc::new(CancelledReportMockSolver {
                name: "cancelled".to_owned(),
                calls: Arc::clone(&first_calls),
                cancelled: true,
                cancel_on_success: false,
            }),
            Arc::new(CancelledReportMockSolver {
                name: "fallback".to_owned(),
                calls: Arc::clone(&second_calls),
                cancelled: false,
                cancel_on_success: false,
            }),
        ]);

        let aggregate = solver
            .solve_milp_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_cancelled_report"),
                FrameworkSolveOptions::default(),
            )
            .await
            .expect("cancelled report should be aggregated");
        assert_eq!(
            aggregate.report.termination_reason,
            TerminationReason::Cancelled
        );
        assert_eq!(aggregate.attempts.len(), 1);
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn report_success_after_attempt_cancellation_is_not_selected() {
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(vec![
            Arc::new(CancelledReportMockSolver {
                name: "cancel-after-start".to_owned(),
                calls: Arc::clone(&first_calls),
                cancelled: false,
                cancel_on_success: true,
            }),
            Arc::new(CancelledReportMockSolver {
                name: "must-not-run".to_owned(),
                calls: Arc::clone(&second_calls),
                cancelled: false,
                cancel_on_success: false,
            }),
        ]);

        let aggregate = solver
            .solve_milp_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_cancel_after_start"),
                FrameworkSolveOptions::default(),
            )
            .expect("cancelled attempt should produce a structured report");

        assert_eq!(
            aggregate.report.termination_reason,
            TerminationReason::Cancelled
        );
        assert!(!aggregate.report.has_incumbent());
        assert_eq!(aggregate.attempts.len(), 1);
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            aggregate.attempts[0].cancellation_reason.as_deref(),
            Some("CALLBACK")
        );
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn report_success_after_attempt_cancellation_is_not_selected() {
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(vec![
            Arc::new(CancelledReportMockSolver {
                name: "cancel-after-start".to_owned(),
                calls: Arc::clone(&first_calls),
                cancelled: false,
                cancel_on_success: true,
            }),
            Arc::new(CancelledReportMockSolver {
                name: "must-not-run".to_owned(),
                calls: Arc::clone(&second_calls),
                cancelled: false,
                cancel_on_success: false,
            }),
        ]);

        let aggregate = solver
            .solve_milp_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_cancel_after_start"),
                FrameworkSolveOptions::default(),
            )
            .await
            .expect("cancelled attempt should produce a structured report");

        assert_eq!(
            aggregate.report.termination_reason,
            TerminationReason::Cancelled
        );
        assert!(!aggregate.report.has_incumbent());
        assert_eq!(aggregate.attempts.len(), 1);
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            aggregate.attempts[0].cancellation_reason.as_deref(),
            Some("CALLBACK")
        );
    }

    #[test]
    fn test_serial_combinatorial_column_generation_solver() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                should_fail: true,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                should_fail: false,
            }),
        ];

        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(solvers);
        assert!(solver.name().contains("solver1"));
        assert!(solver.name().contains("solver2"));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_serial_stops_on_infeasible_error() {
        enum StopErrorKind {
            Infeasible,
        }

        struct StopMockSolver {
            name: String,
            error: Option<StopErrorKind>,
            call_counter: Arc<AtomicUsize>,
        }

        impl ColumnGenerationSolver for StopMockSolver {
            fn name(&self) -> &str {
                &self.name
            }

            fn solve_milp_with_options(
                &self,
                _model: &LinearTriadModel,
                _options: FrameworkSolveOptions,
            ) -> Result<FeasibleSolution> {
                self.call_counter.fetch_add(1, Ordering::SeqCst);
                match &self.error {
                    Some(StopErrorKind::Infeasible) => {
                        Err(CoreError::Solver(SolverError::Infeasible))
                    }
                    None => Ok(FeasibleSolution::new(1.0, vec![1.0])),
                }
            }

            fn solve_lp_with_options(
                &self,
                _model: &LinearTriadModel,
                _options: FrameworkSolveOptions,
            ) -> Result<LPResult> {
                self.call_counter.fetch_add(1, Ordering::SeqCst);
                match &self.error {
                    Some(StopErrorKind::Infeasible) => {
                        Err(CoreError::Solver(SolverError::Infeasible))
                    }
                    None => Ok(LPResult::new(
                        FeasibleSolution::new(1.0, vec![1.0]),
                        LinearDualSolution::default(),
                    )),
                }
            }
        }

        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![
            Arc::new(StopMockSolver {
                name: "solver1".to_string(),
                error: Some(StopErrorKind::Infeasible),
                call_counter: first_calls.clone(),
            }),
            Arc::new(StopMockSolver {
                name: "solver2".to_string(),
                error: None,
                call_counter: second_calls.clone(),
            }),
        ];
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(solvers);
        let model = LinearTriadModel::new("serial_stop_cg");

        let result = solver.solve_milp_with_options(
            &model,
            FrameworkSolveOptions::new().with_name("serial_stop_case"),
        );
        assert!(matches!(
            result,
            Err(CoreError::Solver(SolverError::Infeasible))
        ));
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_serial_solver_forwards_callbacks_with_solver_index() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                should_fail: true,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                should_fail: false,
            }),
        ];
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(solvers);
        let model = LinearTriadModel::new("serial_callback_model");
        let solving_statuses: Arc<Mutex<Vec<SolvingStatus>>> = Arc::new(Mutex::new(Vec::new()));
        let registration_statuses: Arc<Mutex<Vec<RegistrationStatus>>> =
            Arc::new(Mutex::new(Vec::new()));

        let solving_statuses_for_callback = solving_statuses.clone();
        let solving_callback: SolvingStatusCallback = Arc::new(move |status| {
            solving_statuses_for_callback
                .lock()
                .unwrap()
                .push(status.clone());
            Ok(())
        });
        let registration_statuses_for_callback = registration_statuses.clone();
        let registration_callback: RegistrationStatusCallback = Arc::new(move |status| {
            registration_statuses_for_callback
                .lock()
                .unwrap()
                .push(status.clone());
            Ok(())
        });

        let options = FrameworkSolveOptions::new()
            .with_name("serial_callback_case")
            .with_log_model(true)
            .with_registration_callback(Some(registration_callback))
            .with_solving_callback(Some(solving_callback));
        let _ = solver
            .solve_milp_with_options(&model, options)
            .expect("serial solver should return feasible solution");

        let solving_statuses = solving_statuses.lock().unwrap();
        let registration_statuses = registration_statuses.lock().unwrap();
        assert_eq!(solving_statuses.len(), 1);
        assert_eq!(registration_statuses.len(), 1);
        assert_eq!(solving_statuses[0].solver_index, 1);
        assert_eq!(solving_statuses[0].solver, "solver2");
        assert_eq!(registration_statuses[0].solver, "solver2");
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_serial_solver_supports_meta_model_shortcut() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![Arc::new(MockSolver {
            name: "solver_meta".to_string(),
            should_fail: false,
        })];
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(solvers);
        let mut meta_model = MetaModel::<f64>::new("serial_cg_meta_shortcut");
        let x = ContinuousVariableItem::auto("serial_cg_meta_shortcut_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "serial_cg_meta_shortcut_c",
            )
            .unwrap();

        let result = solver
            .solve(&meta_model)
            .expect("serial column-generation meta shortcut should succeed");
        assert!((result.obj - 1.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_serial_solver_meta_shortcut_rejects_non_finite_value() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![Arc::new(MockSolver {
            name: "solver_meta_non_finite".to_string(),
            should_fail: false,
        })];
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(solvers);
        let mut meta_model = MetaModel::<f64>::new("serial_cg_meta_non_finite");
        let x = ContinuousVariableItem::auto("serial_cg_meta_non_finite_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, f64::NAN)],
                ConstraintRelation::LessEqual,
                1.0,
                "serial_cg_meta_non_finite_c",
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
}
