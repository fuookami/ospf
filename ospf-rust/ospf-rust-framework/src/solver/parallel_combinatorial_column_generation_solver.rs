//! 并行组合列生成求解器
//! Parallel Combinatorial Column Generation Solver
//!
//! 本模块提供并行执行的组合列生成求解器。
//! This module provides parallel-executing combinatorial column generation solvers.

use super::column_generation_solver::{
    RegistrationStatusCallback, SolvingStatusCallback, emit_column_generation_progress,
    report_is_selectable,
};
use super::{
    ColumnGenerationSolver, CombinatorialSelection, FeasibleSolution, FrameworkSolveOptions,
    LPResult, ObjectiveCategory, ParallelCombinatorialMode,
    aggregate_combinatorial_reports_with_metadata,
    column_generation_solver::preserve_cancelled_attempts,
    framework_solve_options::{child_cancellation_handle, legacy_cancellation_error},
};
use ospf_rust_core::error::{CoreError, Result, SolverError, SolverNotFoundError};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{
    CombinatorialSolveReport, ProgressValue, SolveReport, SolverProvenance, cancelled_solve_report,
};
use std::sync::Arc;
use std::time::Instant;

/// 并行组合列生成求解器 / Parallel Combinatorial Column Generation Solver
///
/// 组合多个列生成求解器并行执行，根据模式选择结果。
/// Combines multiple column generation solvers to execute in parallel, selecting result based on mode.
pub struct ParallelCombinatorialColumnGenerationSolver {
    /// 求解器列表 / Solver list
    solvers: Vec<Arc<dyn ColumnGenerationSolver>>,
    /// 组合模式 / Combinatorial mode
    mode: ParallelCombinatorialMode,
    /// 名称缓存 / Cached name
    name: String,
}

impl ParallelCombinatorialColumnGenerationSolver {
    /// 创建新的并行组合列生成求解器 / Create new parallel combinatorial column generation solver
    pub fn new(
        solvers: Vec<Arc<dyn ColumnGenerationSolver>>,
        mode: ParallelCombinatorialMode,
    ) -> Self {
        let names: Vec<&str> = solvers.iter().map(|s| s.name()).collect();
        let name = format!("ParallelCombinatorial({})", names.join(","));
        Self {
            solvers,
            mode,
            name,
        }
    }

    /// 使用默认模式创建 / Create with default mode
    pub fn with_solvers(solvers: Vec<Arc<dyn ColumnGenerationSolver>>) -> Self {
        Self::new(solvers, ParallelCombinatorialMode::default())
    }

    /// 获取求解器名称 / Get solver name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取模式 / Get mode
    pub fn mode(&self) -> ParallelCombinatorialMode {
        self.mode
    }

    /// 选择最优解 / Select best solution
    fn select_best(
        solutions: Vec<FeasibleSolution>,
        objective_category: ObjectiveCategory,
    ) -> Option<FeasibleSolution> {
        if solutions.is_empty() {
            return None;
        }

        match objective_category {
            ObjectiveCategory::Minimum => solutions.into_iter().min_by(|a, b| {
                a.obj
                    .partial_cmp(&b.obj)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            ObjectiveCategory::Maximum => solutions.into_iter().max_by(|a, b| {
                a.obj
                    .partial_cmp(&b.obj)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
    }

    /// 选择最优 LP 结果 / Select best LP result
    fn select_best_lp(
        results: Vec<LPResult>,
        objective_category: ObjectiveCategory,
    ) -> Option<LPResult> {
        if results.is_empty() {
            return None;
        }

        match objective_category {
            ObjectiveCategory::Minimum => results.into_iter().min_by(|a, b| {
                a.result
                    .obj
                    .partial_cmp(&b.result.obj)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            ObjectiveCategory::Maximum => results.into_iter().max_by(|a, b| {
                a.result
                    .obj
                    .partial_cmp(&b.result.obj)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
    }

    fn objective_category(model: &LinearTriadModel) -> ObjectiveCategory {
        match model.objective_category {
            ospf_rust_core::model::ObjectiveCategory::Minimum => ObjectiveCategory::Minimum,
            ospf_rust_core::model::ObjectiveCategory::Maximum => ObjectiveCategory::Maximum,
        }
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
impl ColumnGenerationSolver for ParallelCombinatorialColumnGenerationSolver {
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
        use tokio::task::JoinSet;

        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        let begin = Instant::now();
        for solver_index in 0..self.solvers.len() {
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
        }
        let mut tasks: JoinSet<(usize, String, Result<SolveReport<f64>>)> = JoinSet::new();
        let mut task_workers = std::collections::HashMap::new();
        let mut child_handles = Vec::with_capacity(self.solvers.len());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver = Arc::clone(solver);
            let solver_name = solver.name().to_owned();
            let model = model.clone();
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
            child_handles.push(child_handle.clone());
            let worker_handle = child_handle.clone();
            let options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle));
            let worker_name = solver_name.clone();
            let task_id = tasks
                .spawn(async move {
                    let result = solver.solve_milp_report_with_options(&model, options).await;
                    worker_handle.mark_completed();
                    (solver_index, worker_name, result)
                })
                .id();
            task_workers.insert(task_id, (solver_index, solver_name));
        }

        let mut attempts = Vec::with_capacity(self.solvers.len());
        let mut loser_cancelled = false;
        let mut first_selected_index = None;
        while let Some(result) = tasks.join_next_with_id().await {
            match result {
                Ok((_, attempt)) => {
                    task_workers.retain(|_, (index, _)| *index != attempt.0);
                    if matches!(self.mode, ParallelCombinatorialMode::First)
                        && !loser_cancelled
                        && attempt.2.as_ref().is_ok_and(report_is_selectable)
                    {
                        first_selected_index = Some(attempt.0);
                        for (index, handle) in child_handles.iter().enumerate() {
                            if index != attempt.0 {
                                handle.cancel(
                                    ospf_rust_core::solver::CancellationOrigin::FrameworkLoser,
                                );
                            }
                        }
                        loser_cancelled = true;
                    }
                    attempts.push(attempt)
                }
                Err(error) => {
                    let (solver_index, solver_name) = task_workers
                        .remove(&error.id())
                        .unwrap_or_else(|| (usize::MAX, format!("task-{:?}", error.id())));
                    attempts.push((
                        solver_index,
                        solver_name,
                        Err(CoreError::Internal(
                            "column-generation worker panicked".to_owned(),
                        )),
                    ));
                }
            }
        }
        let aggregate = aggregate_combinatorial_reports_with_metadata(
            self.name(),
            match self.mode {
                ParallelCombinatorialMode::First => first_selected_index.map_or(
                    CombinatorialSelection::First,
                    CombinatorialSelection::FirstCompleted,
                ),
                ParallelCombinatorialMode::Best => {
                    CombinatorialSelection::Best(Self::objective_category(model))
                }
            },
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
        use tokio::task::JoinSet;

        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        let begin = Instant::now();
        for solver_index in 0..self.solvers.len() {
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
        }
        let mut tasks: JoinSet<(usize, String, Result<SolveReport<f64>>)> = JoinSet::new();
        let mut task_workers = std::collections::HashMap::new();
        let mut child_handles = Vec::with_capacity(self.solvers.len());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver = Arc::clone(solver);
            let solver_name = solver.name().to_owned();
            let model = model.clone();
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
            child_handles.push(child_handle.clone());
            let worker_handle = child_handle.clone();
            let options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle));
            let worker_name = solver_name.clone();
            let task_id = tasks
                .spawn(async move {
                    let result = solver.solve_lp_report_with_options(&model, options).await;
                    worker_handle.mark_completed();
                    (solver_index, worker_name, result)
                })
                .id();
            task_workers.insert(task_id, (solver_index, solver_name));
        }

        let mut attempts = Vec::with_capacity(self.solvers.len());
        let mut loser_cancelled = false;
        let mut first_selected_index = None;
        while let Some(result) = tasks.join_next_with_id().await {
            match result {
                Ok((_, attempt)) => {
                    task_workers.retain(|_, (index, _)| *index != attempt.0);
                    if matches!(self.mode, ParallelCombinatorialMode::First)
                        && !loser_cancelled
                        && attempt.2.as_ref().is_ok_and(report_is_selectable)
                    {
                        first_selected_index = Some(attempt.0);
                        for (index, handle) in child_handles.iter().enumerate() {
                            if index != attempt.0 {
                                handle.cancel(
                                    ospf_rust_core::solver::CancellationOrigin::FrameworkLoser,
                                );
                            }
                        }
                        loser_cancelled = true;
                    }
                    attempts.push(attempt)
                }
                Err(error) => {
                    let (solver_index, solver_name) = task_workers
                        .remove(&error.id())
                        .unwrap_or_else(|| (usize::MAX, format!("task-{:?}", error.id())));
                    attempts.push((
                        solver_index,
                        solver_name,
                        Err(CoreError::Internal(
                            "column-generation LP worker panicked".to_owned(),
                        )),
                    ));
                }
            }
        }
        let aggregate = aggregate_combinatorial_reports_with_metadata(
            self.name(),
            match self.mode {
                ParallelCombinatorialMode::First => first_selected_index.map_or(
                    CombinatorialSelection::First,
                    CombinatorialSelection::FirstCompleted,
                ),
                ParallelCombinatorialMode::Best => {
                    CombinatorialSelection::Best(Self::objective_category(model))
                }
            },
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
        if options
            .cancellation_handle
            .as_ref()
            .is_some_and(ospf_rust_core::solver::SolveHandle::is_cancelled)
        {
            return Err(legacy_cancellation_error(&options));
        }
        use tokio::task::JoinSet;
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());

        let mut tasks: JoinSet<(usize, Result<FeasibleSolution>)> = JoinSet::new();
        let mut child_handles = Vec::with_capacity(self.solvers.len());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver = Arc::clone(solver);
            let solver_name = solver.name().to_string();
            let model = model.clone();
            let solve_name = solve_name.clone();
            let registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let child_handle = child_cancellation_handle(&options);
            child_handles.push(child_handle.clone());
            let child_options = options
                .clone()
                .with_name(solve_name)
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle));
            tasks.spawn(async move {
                let result = solver.solve_milp_with_options(&model, child_options).await;
                (solver_index, result)
            });
        }

        let mut first_solution = None;
        let mut solutions = Vec::new();
        let mut loser_cancelled = false;
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok((solver_index, Ok(solution))) => {
                    if matches!(self.mode, ParallelCombinatorialMode::First) {
                        if first_solution.is_none() {
                            first_solution = Some(solution);
                        }
                        if !loser_cancelled {
                            for (index, handle) in child_handles.iter().enumerate() {
                                if index != solver_index {
                                    handle.cancel(
                                        ospf_rust_core::solver::CancellationOrigin::FrameworkLoser,
                                    );
                                }
                            }
                            loser_cancelled = true;
                        }
                    } else {
                        solutions.push(solution);
                    }
                }
                Ok((_, Err(error))) => log::warn!("Solver failed: {}", error),
                Err(error) => log::warn!("Solver task failed: {}", error),
            }
        }

        if matches!(self.mode, ParallelCombinatorialMode::First) {
            if let Some(solution) = first_solution {
                return Ok(solution);
            }
        } else if let Some(solution) = Self::select_best(solutions, Self::objective_category(model))
        {
            return Ok(solution);
        }
        if options
            .cancellation_handle
            .as_ref()
            .is_some_and(ospf_rust_core::solver::SolveHandle::is_cancelled)
        {
            return Err(legacy_cancellation_error(&options));
        }
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid",
        )))
    }

    async fn solve_lp_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<LPResult> {
        if options
            .cancellation_handle
            .as_ref()
            .is_some_and(ospf_rust_core::solver::SolveHandle::is_cancelled)
        {
            return Err(legacy_cancellation_error(&options));
        }
        use tokio::task::JoinSet;
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());

        let mut tasks: JoinSet<(usize, Result<LPResult>)> = JoinSet::new();
        let mut child_handles = Vec::with_capacity(self.solvers.len());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver = Arc::clone(solver);
            let solver_name = solver.name().to_string();
            let model = model.clone();
            let solve_name = solve_name.clone();
            let registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let child_handle = child_cancellation_handle(&options);
            child_handles.push(child_handle.clone());
            let child_options = options
                .clone()
                .with_name(solve_name)
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle));
            tasks.spawn(async move {
                let result = solver.solve_lp_with_options(&model, child_options).await;
                (solver_index, result)
            });
        }

        let mut first_result = None;
        let mut results = Vec::new();
        let mut loser_cancelled = false;
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok((solver_index, Ok(lp_result))) => {
                    if matches!(self.mode, ParallelCombinatorialMode::First) {
                        if first_result.is_none() {
                            first_result = Some(lp_result);
                        }
                        if !loser_cancelled {
                            for (index, handle) in child_handles.iter().enumerate() {
                                if index != solver_index {
                                    handle.cancel(
                                        ospf_rust_core::solver::CancellationOrigin::FrameworkLoser,
                                    );
                                }
                            }
                            loser_cancelled = true;
                        }
                    } else {
                        results.push(lp_result);
                    }
                }
                Ok((_, Err(error))) => log::warn!("Solver failed: {}", error),
                Err(error) => log::warn!("Solver task failed: {}", error),
            }
        }

        if matches!(self.mode, ParallelCombinatorialMode::First) {
            if let Some(result) = first_result {
                return Ok(result);
            }
        } else if let Some(result) = Self::select_best_lp(results, Self::objective_category(model))
        {
            return Ok(result);
        }
        if options
            .cancellation_handle
            .as_ref()
            .is_some_and(ospf_rust_core::solver::SolveHandle::is_cancelled)
        {
            return Err(legacy_cancellation_error(&options));
        }
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid",
        )))
    }
}

#[cfg(not(feature = "async"))]
impl ColumnGenerationSolver for ParallelCombinatorialColumnGenerationSolver {
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
        use std::thread;

        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        let begin = Instant::now();
        for solver_index in 0..self.solvers.len() {
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
        }
        let mut child_handles = Vec::with_capacity(self.solvers.len());
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut handles = Vec::with_capacity(self.solvers.len());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver = Arc::clone(solver);
            let solver_name = solver.name().to_owned();
            let model = model.clone();
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
            child_handles.push(child_handle.clone());
            let worker_handle = child_handle.clone();
            let options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle));
            let sender = sender.clone();
            handles.push((
                solver_index,
                solver_name.clone(),
                thread::spawn(move || {
                    let result = solver.solve_milp_report_with_options(&model, options);
                    worker_handle.mark_completed();
                    let _ = sender.send((solver_index, solver_name, result));
                }),
            ));
        }
        drop(sender);
        let mut attempts = Vec::with_capacity(handles.len());
        let mut loser_cancelled = false;
        let mut first_selected_index = None;
        while let Ok(attempt) = receiver.recv() {
            if matches!(self.mode, ParallelCombinatorialMode::First)
                && !loser_cancelled
                && attempt.2.as_ref().is_ok_and(report_is_selectable)
            {
                first_selected_index = Some(attempt.0);
                for (index, child_handle) in child_handles.iter().enumerate() {
                    if index != attempt.0 {
                        child_handle
                            .cancel(ospf_rust_core::solver::CancellationOrigin::FrameworkLoser);
                    }
                }
                loser_cancelled = true;
            }
            attempts.push(attempt);
        }
        let completed = attempts
            .iter()
            .map(|(index, _, _)| *index)
            .collect::<std::collections::BTreeSet<_>>();
        for (index, solver_name, handle) in handles {
            if handle.join().is_err() && !completed.contains(&index) {
                attempts.push((
                    index,
                    solver_name,
                    Err(CoreError::Internal(
                        "column-generation worker panicked".to_owned(),
                    )),
                ));
            }
        }
        let aggregate = aggregate_combinatorial_reports_with_metadata(
            self.name(),
            match self.mode {
                ParallelCombinatorialMode::First => first_selected_index.map_or(
                    CombinatorialSelection::First,
                    CombinatorialSelection::FirstCompleted,
                ),
                ParallelCombinatorialMode::Best => {
                    CombinatorialSelection::Best(Self::objective_category(model))
                }
            },
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
        use std::thread;

        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        let begin = Instant::now();
        for solver_index in 0..self.solvers.len() {
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
        }
        let mut child_handles = Vec::with_capacity(self.solvers.len());
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut handles = Vec::with_capacity(self.solvers.len());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver = Arc::clone(solver);
            let solver_name = solver.name().to_owned();
            let model = model.clone();
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
            child_handles.push(child_handle.clone());
            let worker_handle = child_handle.clone();
            let options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle));
            let sender = sender.clone();
            handles.push((
                solver_index,
                solver_name.clone(),
                thread::spawn(move || {
                    let result = solver.solve_lp_report_with_options(&model, options);
                    worker_handle.mark_completed();
                    let _ = sender.send((solver_index, solver_name, result));
                }),
            ));
        }
        drop(sender);
        let mut attempts = Vec::with_capacity(handles.len());
        let mut loser_cancelled = false;
        let mut first_selected_index = None;
        while let Ok(attempt) = receiver.recv() {
            if matches!(self.mode, ParallelCombinatorialMode::First)
                && !loser_cancelled
                && attempt.2.as_ref().is_ok_and(report_is_selectable)
            {
                first_selected_index = Some(attempt.0);
                for (index, child_handle) in child_handles.iter().enumerate() {
                    if index != attempt.0 {
                        child_handle
                            .cancel(ospf_rust_core::solver::CancellationOrigin::FrameworkLoser);
                    }
                }
                loser_cancelled = true;
            }
            attempts.push(attempt);
        }
        let completed = attempts
            .iter()
            .map(|(index, _, _)| *index)
            .collect::<std::collections::BTreeSet<_>>();
        for (index, solver_name, handle) in handles {
            if handle.join().is_err() && !completed.contains(&index) {
                attempts.push((
                    index,
                    solver_name,
                    Err(CoreError::Internal(
                        "column-generation LP worker panicked".to_owned(),
                    )),
                ));
            }
        }
        let aggregate = aggregate_combinatorial_reports_with_metadata(
            self.name(),
            match self.mode {
                ParallelCombinatorialMode::First => first_selected_index.map_or(
                    CombinatorialSelection::First,
                    CombinatorialSelection::FirstCompleted,
                ),
                ParallelCombinatorialMode::Best => {
                    CombinatorialSelection::Best(Self::objective_category(model))
                }
            },
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
        if options
            .cancellation_handle
            .as_ref()
            .is_some_and(ospf_rust_core::solver::SolveHandle::is_cancelled)
        {
            return Err(legacy_cancellation_error(&options));
        }
        use std::thread;
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());

        let mut child_handles = Vec::with_capacity(self.solvers.len());
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut handles = Vec::with_capacity(self.solvers.len());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver = Arc::clone(solver);
            let solver_name = solver.name().to_string();
            let model = model.clone();
            let solve_name = solve_name.clone();
            let registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let child_handle = child_cancellation_handle(&options);
            child_handles.push(child_handle.clone());
            let child_options = options
                .clone()
                .with_name(solve_name)
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle));
            let sender = sender.clone();
            handles.push(thread::spawn(move || {
                let result = solver.solve_milp_with_options(&model, child_options);
                let _ = sender.send((solver_index, result));
            }));
        }
        drop(sender);

        let mut first_solution = None;
        let mut solutions = Vec::new();
        let mut loser_cancelled = false;
        while let Ok((solver_index, result)) = receiver.recv() {
            match result {
                Ok(solution) => {
                    if matches!(self.mode, ParallelCombinatorialMode::First) {
                        if first_solution.is_none() {
                            first_solution = Some(solution);
                        }
                        if !loser_cancelled {
                            for (index, handle) in child_handles.iter().enumerate() {
                                if index != solver_index {
                                    handle.cancel(
                                        ospf_rust_core::solver::CancellationOrigin::FrameworkLoser,
                                    );
                                }
                            }
                            loser_cancelled = true;
                        }
                    } else {
                        solutions.push(solution);
                    }
                }
                Err(error) => log::warn!("Solver failed: {}", error),
            }
        }
        for handle in handles {
            if handle.join().is_err() {
                log::warn!("Solver thread panicked");
            }
        }

        if matches!(self.mode, ParallelCombinatorialMode::First) {
            if let Some(solution) = first_solution {
                return Ok(solution);
            }
        } else if let Some(solution) = Self::select_best(solutions, Self::objective_category(model))
        {
            return Ok(solution);
        }
        if options
            .cancellation_handle
            .as_ref()
            .is_some_and(ospf_rust_core::solver::SolveHandle::is_cancelled)
        {
            return Err(legacy_cancellation_error(&options));
        }
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid",
        )))
    }

    fn solve_lp_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<LPResult> {
        if options
            .cancellation_handle
            .as_ref()
            .is_some_and(ospf_rust_core::solver::SolveHandle::is_cancelled)
        {
            return Err(legacy_cancellation_error(&options));
        }
        use std::thread;
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());

        let mut child_handles = Vec::with_capacity(self.solvers.len());
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut handles = Vec::with_capacity(self.solvers.len());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver = Arc::clone(solver);
            let solver_name = solver.name().to_string();
            let model = model.clone();
            let solve_name = solve_name.clone();
            let registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let child_handle = child_cancellation_handle(&options);
            child_handles.push(child_handle.clone());
            let child_options = options
                .clone()
                .with_name(solve_name)
                .with_registration_callback(registration_callback)
                .with_solving_callback(solving_callback)
                .with_cancellation_handle(Some(child_handle));
            let sender = sender.clone();
            handles.push(thread::spawn(move || {
                let result = solver.solve_lp_with_options(&model, child_options);
                let _ = sender.send((solver_index, result));
            }));
        }
        drop(sender);

        let mut first_result = None;
        let mut results = Vec::new();
        let mut loser_cancelled = false;
        while let Ok((solver_index, result)) = receiver.recv() {
            match result {
                Ok(lp_result) => {
                    if matches!(self.mode, ParallelCombinatorialMode::First) {
                        if first_result.is_none() {
                            first_result = Some(lp_result);
                        }
                        if !loser_cancelled {
                            for (index, handle) in child_handles.iter().enumerate() {
                                if index != solver_index {
                                    handle.cancel(
                                        ospf_rust_core::solver::CancellationOrigin::FrameworkLoser,
                                    );
                                }
                            }
                            loser_cancelled = true;
                        }
                    } else {
                        results.push(lp_result);
                    }
                }
                Err(error) => log::warn!("Solver failed: {}", error),
            }
        }
        for handle in handles {
            if handle.join().is_err() {
                log::warn!("Solver thread panicked");
            }
        }

        if matches!(self.mode, ParallelCombinatorialMode::First) {
            if let Some(result) = first_result {
                return Ok(result);
            }
        } else if let Some(result) = Self::select_best_lp(results, Self::objective_category(model))
        {
            return Ok(result);
        }
        if options
            .cancellation_handle
            .as_ref()
            .is_some_and(ospf_rust_core::solver::SolveHandle::is_cancelled)
        {
            return Err(legacy_cancellation_error(&options));
        }
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid",
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::LinearDualSolution;
    use crate::solver::column_generation_solver::{RegistrationStatus, SolvingStatus};
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::{ConstraintRelation, MetaModel};
    use ospf_rust_core::solver::{SolveProgressReporter, SolveProgressSnapshot, SolveStage};
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::variable::ContinuousVariableItem;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    struct MockSolver {
        name: String,
        result: f64,
    }

    struct CancellationAwareLegacyColumnGenerationSolver {
        name: String,
        result: f64,
        wait_for_cancel: bool,
        cancelled: Arc<AtomicBool>,
    }

    impl CancellationAwareLegacyColumnGenerationSolver {
        fn record_cancellation(&self, options: &FrameworkSolveOptions) -> Result<()> {
            if options
                .cancellation_handle
                .as_ref()
                .is_some_and(ospf_rust_core::solver::SolveHandle::is_cancelled)
            {
                self.cancelled.store(true, Ordering::SeqCst);
                return Err(CoreError::cancelled("FRAMEWORK_LOSER"));
            }
            Ok(())
        }
    }

    #[cfg(feature = "async")]
    #[async_trait::async_trait]
    impl ColumnGenerationSolver for CancellationAwareLegacyColumnGenerationSolver {
        fn name(&self) -> &str {
            &self.name
        }

        async fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            if self.wait_for_cancel {
                while options
                    .cancellation_handle
                    .as_ref()
                    .is_none_or(|handle| !handle.is_cancelled())
                {
                    tokio::task::yield_now().await;
                }
            }
            self.record_cancellation(&options)?;
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        async fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            if self.wait_for_cancel {
                while options
                    .cancellation_handle
                    .as_ref()
                    .is_none_or(|handle| !handle.is_cancelled())
                {
                    tokio::task::yield_now().await;
                }
            }
            self.record_cancellation(&options)?;
            Ok(LPResult::new(
                FeasibleSolution::new(self.result, vec![self.result]),
                LinearDualSolution::default(),
            ))
        }
    }

    #[cfg(not(feature = "async"))]
    impl ColumnGenerationSolver for CancellationAwareLegacyColumnGenerationSolver {
        fn name(&self) -> &str {
            &self.name
        }

        fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            if self.wait_for_cancel {
                while options
                    .cancellation_handle
                    .as_ref()
                    .is_none_or(|handle| !handle.is_cancelled())
                {
                    std::thread::yield_now();
                }
            }
            self.record_cancellation(&options)?;
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            if self.wait_for_cancel {
                while options
                    .cancellation_handle
                    .as_ref()
                    .is_none_or(|handle| !handle.is_cancelled())
                {
                    std::thread::yield_now();
                }
            }
            self.record_cancellation(&options)?;
            Ok(LPResult::new(
                FeasibleSolution::new(self.result, vec![self.result]),
                LinearDualSolution::default(),
            ))
        }
    }

    fn assert_column_generation_progress(snapshots: &Arc<Mutex<Vec<SolveProgressSnapshot>>>) {
        let snapshots = snapshots
            .lock()
            .expect("parallel column-generation progress snapshots should not be poisoned");
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
    fn report_entry_publishes_parallel_column_generation_progress() {
        let snapshots = Arc::new(Mutex::new(Vec::<SolveProgressSnapshot>::new()));
        let snapshots_for_reporter = Arc::clone(&snapshots);
        let reporter: SolveProgressReporter = Arc::new(move |snapshot| {
            snapshots_for_reporter
                .lock()
                .expect("parallel column-generation progress mutex should not be poisoned")
                .push(snapshot.clone());
            Ok(())
        });
        let solver = ParallelCombinatorialColumnGenerationSolver::with_solvers(vec![
            Arc::new(MockSolver {
                name: "parallel-progress-0".to_owned(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "parallel-progress-1".to_owned(),
                result: 2.0,
            }),
        ]);
        solver
            .solve_milp_combinatorial_report_with_options(
                &LinearTriadModel::new("parallel_progress"),
                FrameworkSolveOptions::default().with_progress_reporter(Some(reporter)),
            )
            .expect("parallel column-generation report should succeed");
        assert_column_generation_progress(&snapshots);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn report_entry_publishes_parallel_column_generation_progress() {
        let snapshots = Arc::new(Mutex::new(Vec::<SolveProgressSnapshot>::new()));
        let snapshots_for_reporter = Arc::clone(&snapshots);
        let reporter: SolveProgressReporter = Arc::new(move |snapshot| {
            snapshots_for_reporter
                .lock()
                .expect("parallel column-generation progress mutex should not be poisoned")
                .push(snapshot.clone());
            Ok(())
        });
        let solver = ParallelCombinatorialColumnGenerationSolver::with_solvers(vec![
            Arc::new(MockSolver {
                name: "parallel-progress-0".to_owned(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "parallel-progress-1".to_owned(),
                result: 2.0,
            }),
        ]);
        solver
            .solve_milp_combinatorial_report_with_options(
                &LinearTriadModel::new("parallel_progress"),
                FrameworkSolveOptions::default().with_progress_reporter(Some(reporter)),
            )
            .await
            .expect("parallel column-generation report should succeed");
        assert_column_generation_progress(&snapshots);
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
            options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            if let Some(callback) = options.registration_status_callback {
                callback(&RegistrationStatus::new(self.name.clone(), 1, 1, 1))?;
            }
            if let Some(callback) = options.solving_status_callback {
                callback(&SolvingStatus::new(self.name.clone(), 0, self.result))?;
            }
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            if let Some(callback) = options.registration_status_callback {
                callback(&RegistrationStatus::new(self.name.clone(), 1, 1, 1))?;
            }
            if let Some(callback) = options.solving_status_callback {
                callback(&SolvingStatus::new(self.name.clone(), 0, self.result))?;
            }
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(feature = "async")]
        async fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            if let Some(callback) = options.registration_status_callback {
                callback(&RegistrationStatus::new(self.name.clone(), 1, 1, 1))?;
            }
            if let Some(callback) = options.solving_status_callback {
                callback(&SolvingStatus::new(self.name.clone(), 0, self.result))?;
            }
            Ok(LPResult::new(
                FeasibleSolution::new(self.result, vec![self.result]),
                LinearDualSolution::default(),
            ))
        }

        #[cfg(not(feature = "async"))]
        fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<LPResult> {
            if let Some(callback) = options.registration_status_callback {
                callback(&RegistrationStatus::new(self.name.clone(), 1, 1, 1))?;
            }
            if let Some(callback) = options.solving_status_callback {
                callback(&SolvingStatus::new(self.name.clone(), 0, self.result))?;
            }
            Ok(LPResult::new(
                FeasibleSolution::new(self.result, vec![self.result]),
                LinearDualSolution::default(),
            ))
        }
    }

    #[test]
    fn test_parallel_combinatorial_column_generation_solver() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                result: 2.0,
            }),
        ];

        let solver = ParallelCombinatorialColumnGenerationSolver::with_solvers(solvers);
        assert!(solver.name().contains("solver1"));
        assert!(solver.name().contains("solver2"));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn legacy_first_mode_cancels_in_flight_milp_and_lp_loser() {
        let loser_cancelled = Arc::new(AtomicBool::new(false));
        let solver = ParallelCombinatorialColumnGenerationSolver::new(
            vec![
                Arc::new(CancellationAwareLegacyColumnGenerationSolver {
                    name: "legacy-cg-winner".to_owned(),
                    result: 1.0,
                    wait_for_cancel: false,
                    cancelled: Arc::new(AtomicBool::new(false)),
                }),
                Arc::new(CancellationAwareLegacyColumnGenerationSolver {
                    name: "legacy-cg-loser".to_owned(),
                    result: 2.0,
                    wait_for_cancel: true,
                    cancelled: Arc::clone(&loser_cancelled),
                }),
            ],
            ParallelCombinatorialMode::First,
        );
        let model = LinearTriadModel::new("legacy_cg_cancellation");

        let milp = solver
            .solve_milp_with_options(&model, FrameworkSolveOptions::default())
            .expect("legacy MILP First should return the winner");
        assert!((milp.obj - 1.0).abs() <= 1e-9);
        assert!(loser_cancelled.load(Ordering::SeqCst));

        loser_cancelled.store(false, Ordering::SeqCst);
        let lp = solver
            .solve_lp_with_options(&model, FrameworkSolveOptions::default())
            .expect("legacy LP First should return the winner");
        assert!((lp.result.obj - 1.0).abs() <= 1e-9);
        assert!(loser_cancelled.load(Ordering::SeqCst));
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn legacy_first_mode_cancels_in_flight_milp_and_lp_loser() {
        let loser_cancelled = Arc::new(AtomicBool::new(false));
        let solver = ParallelCombinatorialColumnGenerationSolver::new(
            vec![
                Arc::new(CancellationAwareLegacyColumnGenerationSolver {
                    name: "legacy-cg-winner".to_owned(),
                    result: 1.0,
                    wait_for_cancel: false,
                    cancelled: Arc::new(AtomicBool::new(false)),
                }),
                Arc::new(CancellationAwareLegacyColumnGenerationSolver {
                    name: "legacy-cg-loser".to_owned(),
                    result: 2.0,
                    wait_for_cancel: true,
                    cancelled: Arc::clone(&loser_cancelled),
                }),
            ],
            ParallelCombinatorialMode::First,
        );
        let model = LinearTriadModel::new("legacy_cg_cancellation");

        let milp = solver
            .solve_milp_with_options(&model, FrameworkSolveOptions::default())
            .await
            .expect("legacy MILP First should return the winner");
        assert!((milp.obj - 1.0).abs() <= 1e-9);
        assert!(loser_cancelled.load(Ordering::SeqCst));

        loser_cancelled.store(false, Ordering::SeqCst);
        let lp = solver
            .solve_lp_with_options(&model, FrameworkSolveOptions::default())
            .await
            .expect("legacy LP First should return the winner");
        assert!((lp.result.obj - 1.0).abs() <= 1e-9);
        assert!(loser_cancelled.load(Ordering::SeqCst));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_best_mode_respects_maximum_objective_category() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                result: 2.0,
            }),
        ];
        let solver = ParallelCombinatorialColumnGenerationSolver::new(
            solvers,
            ParallelCombinatorialMode::Best,
        );
        let mut model = LinearTriadModel::new("max_cg");
        model.objective_category = ospf_rust_core::model::ObjectiveCategory::Maximum;

        let milp_result = solver
            .solve_milp_with_options(&model, FrameworkSolveOptions::new().with_name("max_case"))
            .expect("best mode should return one feasible solution");
        assert!((milp_result.obj - 2.0).abs() <= 1e-9);

        let lp_result = solver
            .solve_lp_with_options(&model, FrameworkSolveOptions::new().with_name("max_case"))
            .expect("best mode should return one feasible solution");
        assert!((lp_result.result.obj - 2.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_parallel_solver_forwards_callbacks_with_solver_index() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver_a".to_string(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "solver_b".to_string(),
                result: 2.0,
            }),
        ];
        let solver = ParallelCombinatorialColumnGenerationSolver::new(
            solvers,
            ParallelCombinatorialMode::Best,
        );
        let model = LinearTriadModel::new("callback_model");
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
            .with_name("callback_case")
            .with_log_model(true)
            .with_registration_callback(Some(registration_callback))
            .with_solving_callback(Some(solving_callback));
        let _ = solver
            .solve_milp_with_options(&model, options)
            .expect("parallel solver should return feasible solution");

        let solving_statuses = solving_statuses.lock().unwrap();
        let registration_statuses = registration_statuses.lock().unwrap();
        assert_eq!(solving_statuses.len(), 2);
        assert_eq!(registration_statuses.len(), 2);
        assert!(
            solving_statuses
                .iter()
                .any(|status| status.solver_index == 0)
        );
        assert!(
            solving_statuses
                .iter()
                .any(|status| status.solver_index == 1)
        );
        assert!(
            registration_statuses
                .iter()
                .any(|status| status.solver == "solver_a")
        );
        assert!(
            registration_statuses
                .iter()
                .any(|status| status.solver == "solver_b")
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_parallel_solver_supports_meta_model_shortcut() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![Arc::new(MockSolver {
            name: "solver_meta".to_string(),
            result: 2.0,
        })];
        let solver = ParallelCombinatorialColumnGenerationSolver::with_solvers(solvers);
        let mut meta_model = MetaModel::<f64>::new("parallel_cg_meta_shortcut");
        let x = ContinuousVariableItem::auto("parallel_cg_meta_shortcut_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "parallel_cg_meta_shortcut_c",
            )
            .unwrap();

        let result = solver
            .solve(&meta_model)
            .expect("parallel column-generation meta shortcut should succeed");
        assert!((result.obj - 2.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_parallel_solver_meta_shortcut_rejects_non_finite_value() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![Arc::new(MockSolver {
            name: "solver_meta_non_finite".to_string(),
            result: 2.0,
        })];
        let solver = ParallelCombinatorialColumnGenerationSolver::with_solvers(solvers);
        let mut meta_model = MetaModel::<f64>::new("parallel_cg_meta_non_finite");
        let x = ContinuousVariableItem::auto("parallel_cg_meta_non_finite_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, f64::NAN)],
                ConstraintRelation::LessEqual,
                1.0,
                "parallel_cg_meta_non_finite_c",
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
