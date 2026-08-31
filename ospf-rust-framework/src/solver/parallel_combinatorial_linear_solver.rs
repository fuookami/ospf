//! 并行组合线性求解器
//! Parallel Combinatorial Linear Solver
//!
//! 本模块提供并行执行的组合线性求解器。
//! This module provides parallel-executing combinatorial linear solvers.

use super::{
    CombinatorialSelection, FeasibleSolution, FrameworkSolveOptions, ObjectiveCategory,
    ParallelCombinatorialMode, aggregate_attempt_id, aggregate_combinatorial_reports_with_metadata,
    child_attempt_id,
    column_generation_solver::{preserve_cancelled_attempts, report_is_selectable},
    framework_solve_options::{child_cancellation_handle, legacy_cancellation_error},
};
use ospf_rust_core::error::{CoreError, Result, SolverError, SolverNotFoundError};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{
    CombinatorialSolveReport, SolveReport, SolverProvenance, cancelled_solve_report,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[cfg(feature = "async")]
type AsyncLinearMultiSolution<'a> =
    Pin<Box<dyn Future<Output = Result<(FeasibleSolution, Vec<Vec<f64>>)>> + Send + 'a>>;

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

/// 线性求解器 trait / Linear Solver Trait
///
/// 定义线性求解器的接口。
/// Defines interface for linear solvers.
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait LinearSolver: Send + Sync {
    /// 获取求解器名称 / Get solver name
    fn name(&self) -> &str;

    /// 求解线性模型 / Solve linear model
    #[cfg(feature = "async")]
    async fn solve(&self, model: &LinearTriadModel) -> Result<FeasibleSolution>;

    /// 使用参数对象求解线性模型 / Solve linear model with options object
    #[cfg(feature = "async")]
    fn solve_with_options<'a>(
        &'a self,
        model: &'a LinearTriadModel,
        _options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    {
        Box::pin(async move { self.solve(model).await })
    }

    /// 以统一组合报告求解线性模型 / Solve a linear model as a combinatorial unified report.
    #[cfg(feature = "async")]
    async fn solve_combinatorial_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = pre_cancelled_report(self.name(), &options)? {
            return Ok(report);
        }
        let solution = self.solve_with_options(model, options).await?;
        let report = solution.to_solve_report(self.name())?;
        Ok(CombinatorialSolveReport::single(
            report,
            format!("{}#0", self.name()),
        ))
    }

    /// 以统一报告求解线性模型 / Solve a linear model as a unified report.
    #[cfg(feature = "async")]
    async fn solve_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_combinatorial_report_with_options(model, options)
            .await
            .map(|aggregate| aggregate.report)
    }

    /// 求解线性模型（同步）/ Solve linear model (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve(&self, model: &LinearTriadModel) -> Result<FeasibleSolution>;

    /// 使用参数对象求解线性模型（同步）/ Solve linear model with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_with_options(
        &self,
        model: &LinearTriadModel,
        _options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve(model)
    }

    /// 以统一组合报告求解线性模型 / Solve a linear model as a combinatorial unified report.
    #[cfg(not(feature = "async"))]
    fn solve_combinatorial_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = pre_cancelled_report(self.name(), &options)? {
            return Ok(report);
        }
        let solution = self.solve_with_options(model, options)?;
        let report = solution.to_solve_report(self.name())?;
        Ok(CombinatorialSolveReport::single(
            report,
            format!("{}#0", self.name()),
        ))
    }

    /// 以统一报告求解线性模型 / Solve a linear model as a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_combinatorial_report_with_options(model, options)
            .map(|aggregate| aggregate.report)
    }

    /// 求解线性模型并返回多个解（参数对象）/
    /// Solve linear model and return multiple solutions with options object
    #[cfg(feature = "async")]
    fn solve_multi_with_options<'a>(
        &'a self,
        model: &'a LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> AsyncLinearMultiSolution<'a> {
        Box::pin(async move {
            let result = self.solve_with_options(model, options).await?;
            Ok((result.clone(), vec![result.solution]))
        })
    }

    /// 求解线性模型并返回多个解（参数对象，同步）/
    /// Solve linear model and return multiple solutions with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_multi_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
        let result = self.solve_with_options(model, options)?;
        Ok((result.clone(), vec![result.solution]))
    }
}

/// 线性求解器 MetaModel 扩展入口 / MetaModel extension entry for linear solvers
///
/// 推荐应用层入口：`solve_meta` 与 `solve_meta_with_options`。
/// Recommended application-facing entries are `solve_meta` and `solve_meta_with_options`.
#[cfg(feature = "async")]
pub trait LinearMetaModelSolverExt: LinearSolver {
    /// 简化入口：求解 MetaModel / Simplified entry: solve MetaModel
    fn solve_meta<'a, V>(
        &'a self,
        meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 简化入口：求解 MetaModel（参数对象）/
    /// Simplified entry: solve MetaModel with options object
    fn solve_meta_with_options<'a, V>(
        &'a self,
        meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
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
        Box::pin(async move { self.solve_with_options(&triad_model, options).await })
    }

    /// 使用 MetaModel 返回统一报告 / Solve a MetaModel and return a unified report.
    fn solve_meta_report<'a, V>(
        &'a self,
        meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SolveReport<f64>>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_report_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 使用 MetaModel 和参数返回统一报告 / Solve a MetaModel with options and return a unified report.
    fn solve_meta_report_with_options<'a, V>(
        &'a self,
        meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SolveReport<f64>>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        let mechanism_model = match meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(error) => return Box::pin(std::future::ready(Err(error))),
        };
        let mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &mechanism_model,
            options.value_conversion_policy,
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
        Box::pin(async move { self.solve_report_with_options(&triad_model, options).await })
    }
}

#[cfg(feature = "async")]
impl<T> LinearMetaModelSolverExt for T where T: LinearSolver + ?Sized {}

/// 线性求解器 MetaModel 扩展入口 / MetaModel extension entry for linear solvers
///
/// 推荐应用层入口：`solve_meta` 与 `solve_meta_with_options`。
/// Recommended application-facing entries are `solve_meta` and `solve_meta_with_options`.
#[cfg(not(feature = "async"))]
pub trait LinearMetaModelSolverExt: LinearSolver {
    /// 简化入口：求解 MetaModel / Simplified entry: solve MetaModel
    fn solve_meta<V>(&self, meta_model: &MetaModel<V>) -> Result<FeasibleSolution>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 简化入口：求解 MetaModel（参数对象）/
    /// Simplified entry: solve MetaModel with options object
    fn solve_meta_with_options<V>(
        &self,
        meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution>
    where
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
        self.solve_with_options(&triad_model, options)
    }

    /// 使用 MetaModel 返回统一报告 / Solve a MetaModel and return a unified report.
    fn solve_meta_report<V>(&self, meta_model: &MetaModel<V>) -> Result<SolveReport<f64>>
    where
        V: ospf_rust_core::solver::SolveValue + std::ops::Add<Output = V>,
    {
        self.solve_meta_report_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 使用 MetaModel 和参数返回统一报告 / Solve a MetaModel with options and return a unified report.
    fn solve_meta_report_with_options<V>(
        &self,
        meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>>
    where
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
        self.solve_report_with_options(&triad_model, options)
    }
}

#[cfg(not(feature = "async"))]
impl<T> LinearMetaModelSolverExt for T where T: LinearSolver + ?Sized {}

/// 并行组合线性求解器 / Parallel Combinatorial Linear Solver
///
/// 组合多个线性求解器并行执行，根据模式选择结果。
/// Combines multiple linear solvers to execute in parallel, selecting result based on mode.
pub struct ParallelCombinatorialLinearSolver {
    /// 求解器列表 / Solver list
    solvers: Vec<Arc<dyn LinearSolver>>,
    /// 组合模式 / Combinatorial mode
    mode: ParallelCombinatorialMode,
    /// 名称缓存 / Cached name
    name: String,
}

impl ParallelCombinatorialLinearSolver {
    /// 创建新的并行组合线性求解器 / Create new parallel combinatorial linear solver
    pub fn new(solvers: Vec<Arc<dyn LinearSolver>>, mode: ParallelCombinatorialMode) -> Self {
        let names: Vec<&str> = solvers.iter().map(|s| s.name()).collect();
        let name = format!("ParallelCombinatorial({})", names.join(","));
        Self {
            solvers,
            mode,
            name,
        }
    }

    /// 使用默认模式创建 / Create with default mode
    pub fn with_solvers(solvers: Vec<Arc<dyn LinearSolver>>) -> Self {
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

    fn objective_category(model: &LinearTriadModel) -> ObjectiveCategory {
        match model.objective_category {
            ospf_rust_core::model::ObjectiveCategory::Minimum => ObjectiveCategory::Minimum,
            ospf_rust_core::model::ObjectiveCategory::Maximum => ObjectiveCategory::Maximum,
        }
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl LinearSolver for ParallelCombinatorialLinearSolver {
    fn name(&self) -> &str {
        &self.name
    }

    async fn solve_combinatorial_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = pre_cancelled_report(self.name(), &options)? {
            return Ok(report);
        }
        use tokio::task::JoinSet;

        let mut tasks: JoinSet<(usize, String, Result<SolveReport<f64>>)> = JoinSet::new();
        let mut task_workers = std::collections::HashMap::new();
        let mut child_handles = Vec::with_capacity(self.solvers.len());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver = Arc::clone(solver);
            let solver_name = solver.name().to_owned();
            let model = model.clone();
            let child_handle = child_cancellation_handle(&options);
            child_handles.push(child_handle.clone());
            let worker_handle = child_handle.clone();
            let options = options.clone().with_cancellation_handle(Some(child_handle));
            let worker_name = solver_name.clone();
            let task_id = tasks
                .spawn(async move {
                    let result = solver.solve_report_with_options(&model, options).await;
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
                            "linear combinatorial worker panicked".to_owned(),
                        )),
                    ));
                }
            }
        }

        aggregate_combinatorial_reports_with_metadata(
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
        )
    }

    async fn solve(&self, model: &LinearTriadModel) -> Result<FeasibleSolution> {
        self.solve_with_options(model, FrameworkSolveOptions::default())
            .await
    }

    fn solve_with_options<'a>(
        &'a self,
        model: &'a LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    {
        Box::pin(async move {
            if options
                .cancellation_handle
                .as_ref()
                .is_some_and(ospf_rust_core::solver::SolveHandle::is_cancelled)
            {
                return Err(legacy_cancellation_error(&options));
            }

            use tokio::task::JoinSet;

            let mut tasks: JoinSet<(usize, Result<FeasibleSolution>)> = JoinSet::new();
            let mut child_handles = Vec::with_capacity(self.solvers.len());
            for (solver_index, solver) in self.solvers.iter().enumerate() {
                let solver = Arc::clone(solver);
                let model = model.clone();
                let child_handle = child_cancellation_handle(&options);
                child_handles.push(child_handle.clone());
                let child_options = options.clone().with_cancellation_handle(Some(child_handle));
                tasks.spawn(async move {
                    let result = solver.solve_with_options(&model, child_options).await;
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
                                first_solution = Some(solution.clone());
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
                    Ok((_, Err(error))) => {
                        log::warn!("Solver failed: {}", error);
                    }
                    Err(error) => {
                        log::warn!("Solver task failed: {}", error);
                    }
                }
            }

            if matches!(self.mode, ParallelCombinatorialMode::First) {
                if let Some(solution) = first_solution {
                    return Ok(solution);
                }
            } else if let Some(solution) =
                Self::select_best(solutions, Self::objective_category(model))
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
        })
    }

    fn solve_multi_with_options<'a>(
        &'a self,
        model: &'a LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(FeasibleSolution, Vec<Vec<f64>>)>> + Send + 'a,
        >,
    > {
        Box::pin(async move {
            let result = LinearSolver::solve_with_options(self, model, options).await?;
            Ok((result.clone(), vec![result.solution]))
        })
    }
}

#[cfg(not(feature = "async"))]
impl LinearSolver for ParallelCombinatorialLinearSolver {
    fn name(&self) -> &str {
        &self.name
    }

    fn solve_combinatorial_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = pre_cancelled_report(self.name(), &options)? {
            return Ok(report);
        }
        use std::thread;

        let mut child_handles = Vec::with_capacity(self.solvers.len());
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut handles = Vec::with_capacity(self.solvers.len());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver = Arc::clone(solver);
            let solver_name = solver.name().to_owned();
            let model = model.clone();
            let child_handle = child_cancellation_handle(&options);
            child_handles.push(child_handle.clone());
            let worker_handle = child_handle.clone();
            let options = options.clone().with_cancellation_handle(Some(child_handle));
            let sender = sender.clone();
            handles.push((
                solver_index,
                solver_name.clone(),
                thread::spawn(move || {
                    let result = solver.solve_report_with_options(&model, options);
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
                        "linear combinatorial worker panicked".to_owned(),
                    )),
                ));
            }
        }

        aggregate_combinatorial_reports_with_metadata(
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
        )
    }

    fn solve(&self, model: &LinearTriadModel) -> Result<FeasibleSolution> {
        self.solve_with_options(model, FrameworkSolveOptions::default())
    }

    fn solve_with_options(
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
        let mut child_handles = Vec::with_capacity(self.solvers.len());
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut handles = Vec::with_capacity(self.solvers.len());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver = Arc::clone(solver);
            let model = model.clone();
            let child_handle = child_cancellation_handle(&options);
            child_handles.push(child_handle.clone());
            let child_options = options.clone().with_cancellation_handle(Some(child_handle));
            let sender = sender.clone();
            handles.push(thread::spawn(move || {
                let result = solver.solve_with_options(&model, child_options);
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

    fn solve_multi_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
        let result = LinearSolver::solve_with_options(self, model, options)?;
        Ok((result.clone(), vec![result.solution]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::error::{CoreError, SolverError};
    use ospf_rust_core::model::MetaModel;
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::{
        ConstraintRelation, ModelBuildingStage, ModelBuildingStatusCallback,
    };
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::variable::ContinuousVariableItem;
    #[cfg(not(feature = "async"))]
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    #[cfg(not(feature = "async"))]
    use std::time::Instant;

    struct MockSolver {
        name: String,
        result: f64,
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl LinearSolver for MockSolver {
        fn name(&self) -> &str {
            &self.name
        }

        #[cfg(feature = "async")]
        async fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(not(feature = "async"))]
        fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(feature = "async")]
        fn solve_multi_with_options<'a>(
            &'a self,
            model: &'a LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<(FeasibleSolution, Vec<Vec<f64>>)>>
                    + Send
                    + 'a,
            >,
        > {
            Box::pin(async move {
                let result = LinearSolver::solve_with_options(self, model, options).await?;
                Ok((result.clone(), vec![result.solution]))
            })
        }

        #[cfg(not(feature = "async"))]
        fn solve_multi_with_options(
            &self,
            model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
            let result = LinearSolver::solve_with_options(self, model, options)?;
            Ok((result.clone(), vec![result.solution]))
        }
    }

    #[cfg(not(feature = "async"))]
    struct CancellationAwareMockSolver {
        name: String,
        result: f64,
        wait_for_cancel: bool,
        error_on_cancel: bool,
    }

    #[cfg(not(feature = "async"))]
    impl LinearSolver for CancellationAwareMockSolver {
        fn name(&self) -> &str {
            &self.name
        }

        fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        fn solve_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<FeasibleSolution> {
            let Some(handle) = options.cancellation_handle.as_ref() else {
                return Err(CoreError::contract_error(
                    "cancellation-aware worker requires a cancellation handle",
                ));
            };
            if self.wait_for_cancel {
                let deadline = Instant::now() + Duration::from_secs(2);
                while !handle.is_cancelled() && Instant::now() < deadline {
                    std::thread::yield_now();
                }
                if !handle.is_cancelled() {
                    return Err(CoreError::Internal(
                        "parallel First did not cancel the in-flight legacy loser".to_owned(),
                    ));
                }
                if self.error_on_cancel {
                    return Err(CoreError::Solver(SolverError::SolveFailed(
                        "legacy backend returned an error after loser cancellation".to_owned(),
                    )));
                }
                return Err(CoreError::cancelled(
                    handle
                        .cancellation()
                        .map(|record| record.origin.to_string())
                        .unwrap_or_else(|| "UNKNOWN".to_owned()),
                ));
            }
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        fn solve_report_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<SolveReport<f64>> {
            let Some(handle) = options.cancellation_handle.as_ref() else {
                return Err(CoreError::contract_error(
                    "cancellation-aware worker requires a cancellation handle",
                ));
            };

            if self.wait_for_cancel {
                let deadline = Instant::now() + Duration::from_secs(2);
                while !handle.is_cancelled() && Instant::now() < deadline {
                    std::thread::yield_now();
                }
                if !handle.is_cancelled() {
                    return Err(CoreError::Internal(
                        "parallel First did not cancel the in-flight loser".to_owned(),
                    ));
                }
                if self.error_on_cancel {
                    return Err(CoreError::Solver(SolverError::SolveFailed(
                        "backend returned an error after loser cancellation".to_owned(),
                    )));
                }
                return cancelled_solve_report(
                    SolverProvenance {
                        solver_id: self.name.clone(),
                        backend_name: "test-cancellation-aware".to_owned(),
                        ..SolverProvenance::default()
                    },
                    handle,
                );
            }

            FeasibleSolution::new(self.result, vec![self.result]).to_solve_report(&self.name)
        }
    }

    #[cfg(feature = "async")]
    struct AsyncCancellationAwareMockSolver {
        name: String,
        result: f64,
        wait_for_cancel: bool,
        error_on_cancel: bool,
    }

    #[cfg(feature = "async")]
    #[async_trait::async_trait]
    impl LinearSolver for AsyncCancellationAwareMockSolver {
        fn name(&self) -> &str {
            &self.name
        }

        async fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        fn solve_with_options<'a>(
            &'a self,
            _model: &'a LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>,
        > {
            let wait_for_cancel = self.wait_for_cancel;
            let error_on_cancel = self.error_on_cancel;
            let result = self.result;
            Box::pin(async move {
                let Some(handle) = options.cancellation_handle.as_ref() else {
                    return Err(CoreError::contract_error(
                        "cancellation-aware worker requires a cancellation handle",
                    ));
                };
                if wait_for_cancel {
                    let cancelled = tokio::time::timeout(Duration::from_secs(2), async {
                        while !handle.is_cancelled() {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await
                    .is_ok();
                    if !cancelled {
                        return Err(CoreError::Internal(
                            "parallel First did not cancel the in-flight legacy loser".to_owned(),
                        ));
                    }
                    if error_on_cancel {
                        return Err(CoreError::Solver(SolverError::SolveFailed(
                            "legacy backend returned an error after loser cancellation".to_owned(),
                        )));
                    }
                    return Err(CoreError::cancelled(
                        handle
                            .cancellation()
                            .map(|record| record.origin.to_string())
                            .unwrap_or_else(|| "UNKNOWN".to_owned()),
                    ));
                }
                Ok(FeasibleSolution::new(result, vec![result]))
            })
        }

        async fn solve_report_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<SolveReport<f64>> {
            let Some(handle) = options.cancellation_handle.as_ref() else {
                return Err(CoreError::contract_error(
                    "cancellation-aware worker requires a cancellation handle",
                ));
            };

            if self.wait_for_cancel {
                let cancelled = tokio::time::timeout(Duration::from_secs(2), async {
                    while !handle.is_cancelled() {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .is_ok();
                if !cancelled {
                    return Err(CoreError::Internal(
                        "parallel First did not cancel the in-flight loser".to_owned(),
                    ));
                }
                if self.error_on_cancel {
                    return Err(CoreError::Solver(SolverError::SolveFailed(
                        "backend returned an error after loser cancellation".to_owned(),
                    )));
                }
                return cancelled_solve_report(
                    SolverProvenance {
                        solver_id: self.name.clone(),
                        backend_name: "test-cancellation-aware".to_owned(),
                        ..SolverProvenance::default()
                    },
                    handle,
                );
            }

            FeasibleSolution::new(self.result, vec![self.result]).to_solve_report(&self.name)
        }
    }

    struct ReportWorker {
        name: String,
        result: f64,
        delay: Duration,
        panic_in_report: bool,
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl LinearSolver for ReportWorker {
        fn name(&self) -> &str {
            &self.name
        }

        #[cfg(feature = "async")]
        async fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(not(feature = "async"))]
        fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(feature = "async")]
        async fn solve_report_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<SolveReport<f64>> {
            tokio::time::sleep(self.delay).await;
            if self.panic_in_report {
                panic!("report worker panic");
            }
            FeasibleSolution::new(self.result, vec![self.result]).to_solve_report(&self.name)
        }

        #[cfg(not(feature = "async"))]
        fn solve_report_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> Result<SolveReport<f64>> {
            std::thread::sleep(self.delay);
            if self.panic_in_report {
                panic!("report worker panic");
            }
            FeasibleSolution::new(self.result, vec![self.result]).to_solve_report(&self.name)
        }
    }

    #[test]
    fn test_parallel_combinatorial_solver() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                result: 2.0,
            }),
        ];

        let solver = ParallelCombinatorialLinearSolver::with_solvers(solvers);
        assert!(solver.name().contains("solver1"));
        assert!(solver.name().contains("solver2"));
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn report_panic_keeps_the_actual_high_index_attempt() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(ReportWorker {
                name: "slow-worker".to_owned(),
                result: 1.0,
                delay: Duration::from_millis(25),
                panic_in_report: false,
            }),
            Arc::new(ReportWorker {
                name: "panic-worker".to_owned(),
                result: 2.0,
                delay: Duration::ZERO,
                panic_in_report: true,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::Best);
        let aggregate = solver
            .solve_combinatorial_report_with_options(
                &LinearTriadModel::new("panic_attempt_identity"),
                FrameworkSolveOptions::default(),
            )
            .await
            .expect("worker panic should remain an attempt failure");

        assert_eq!(aggregate.attempts.len(), 2);
        assert!(aggregate.attempts.iter().any(|attempt| {
            attempt.attempt_id
                == child_attempt_id(&aggregate_attempt_id(solver.name()), "panic-worker", 1)
        }));
        assert_eq!(
            aggregate.selected_attempt_id.as_deref(),
            Some(child_attempt_id(&aggregate_attempt_id(solver.name()), "slow-worker", 0).as_str())
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn report_panic_keeps_the_actual_high_index_attempt() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(ReportWorker {
                name: "slow-worker".to_owned(),
                result: 1.0,
                delay: Duration::from_millis(25),
                panic_in_report: false,
            }),
            Arc::new(ReportWorker {
                name: "panic-worker".to_owned(),
                result: 2.0,
                delay: Duration::ZERO,
                panic_in_report: true,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::Best);
        let aggregate = solver
            .solve_combinatorial_report_with_options(
                &LinearTriadModel::new("panic_attempt_identity"),
                FrameworkSolveOptions::default(),
            )
            .expect("worker panic should remain an attempt failure");

        assert_eq!(aggregate.attempts.len(), 2);
        assert!(aggregate.attempts.iter().any(|attempt| {
            attempt.attempt_id
                == child_attempt_id(&aggregate_attempt_id(solver.name()), "panic-worker", 1)
        }));
        assert_eq!(
            aggregate.selected_attempt_id.as_deref(),
            Some(child_attempt_id(&aggregate_attempt_id(solver.name()), "slow-worker", 0).as_str())
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn report_entry_collects_all_parallel_attempts_and_selects_deterministically() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(MockSolver {
                name: "parallel-first".to_owned(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "parallel-second".to_owned(),
                result: 2.0,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::Best);
        let mut model = LinearTriadModel::new("parallel_report_contract");
        model.objective_category = ospf_rust_core::model::ObjectiveCategory::Minimum;
        let aggregate = solver
            .solve_combinatorial_report_with_options(&model, FrameworkSolveOptions::default())
            .expect("parallel report entry should aggregate worker reports");

        assert_eq!(aggregate.attempts.len(), 2);
        assert!(
            aggregate
                .attempts
                .iter()
                .all(|attempt| attempt.completed_at_epoch_ms.is_some())
        );
        assert_eq!(
            aggregate.selected_attempt_id.as_deref(),
            Some(
                child_attempt_id(&aggregate_attempt_id(solver.name()), "parallel-first", 0,)
                    .as_str(),
            )
        );
        assert_eq!(aggregate.report.incumbent(), None);
        assert_eq!(
            aggregate
                .report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective_value),
            Some(1.0)
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn first_mode_cancels_in_flight_loser_and_keeps_cancelled_trace() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(CancellationAwareMockSolver {
                name: "winner".to_owned(),
                result: 1.0,
                wait_for_cancel: false,
                error_on_cancel: false,
            }),
            Arc::new(CancellationAwareMockSolver {
                name: "loser".to_owned(),
                result: 2.0,
                wait_for_cancel: true,
                error_on_cancel: false,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::First);
        let mut model = LinearTriadModel::new("parallel_first_cancellation_contract");
        model.objective_category = ospf_rust_core::model::ObjectiveCategory::Minimum;

        let aggregate = solver
            .solve_combinatorial_report_with_options(&model, FrameworkSolveOptions::default())
            .expect("parallel First should return the winner report");

        assert_eq!(
            aggregate.selected_attempt_id.as_deref(),
            Some(child_attempt_id(&aggregate_attempt_id(solver.name()), "winner", 0).as_str())
        );
        let loser = aggregate
            .attempts
            .iter()
            .find(|attempt| {
                attempt.attempt_id
                    == child_attempt_id(&aggregate_attempt_id(solver.name()), "loser", 1)
            })
            .expect("loser attempt should remain in the aggregate trace");
        assert_eq!(
            loser.outcome,
            ospf_rust_core::solver::SolveAttemptOutcome::Cancelled
        );
        assert_eq!(
            loser.cancellation_reason.as_deref(),
            Some("FRAMEWORK_LOSER")
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn first_mode_preserves_loser_origin_when_backend_returns_error() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(CancellationAwareMockSolver {
                name: "winner-error-test".to_owned(),
                result: 1.0,
                wait_for_cancel: false,
                error_on_cancel: false,
            }),
            Arc::new(CancellationAwareMockSolver {
                name: "loser-error-test".to_owned(),
                result: 2.0,
                wait_for_cancel: true,
                error_on_cancel: true,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::First);
        let aggregate = solver
            .solve_combinatorial_report_with_options(
                &LinearTriadModel::new("parallel_error_cancellation_contract"),
                FrameworkSolveOptions::default(),
            )
            .expect("winner report should survive a cancelled loser error");
        let loser = aggregate
            .attempts
            .iter()
            .find(|attempt| {
                attempt.attempt_id
                    == child_attempt_id(&aggregate_attempt_id(solver.name()), "loser-error-test", 1)
            })
            .expect("cancelled loser error should remain in the trace");
        assert_eq!(
            loser.outcome,
            ospf_rust_core::solver::SolveAttemptOutcome::Cancelled
        );
        assert_eq!(
            loser.cancellation_reason.as_deref(),
            Some("FRAMEWORK_LOSER")
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn legacy_first_mode_cancels_in_flight_loser_before_returning() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(CancellationAwareMockSolver {
                name: "legacy-winner".to_owned(),
                result: 1.0,
                wait_for_cancel: false,
                error_on_cancel: false,
            }),
            Arc::new(CancellationAwareMockSolver {
                name: "legacy-loser".to_owned(),
                result: 2.0,
                wait_for_cancel: true,
                error_on_cancel: true,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::First);
        let solution = solver
            .solve_with_options(
                &LinearTriadModel::new("parallel_legacy_first_cancellation"),
                FrameworkSolveOptions::default(),
            )
            .expect("legacy First should return the winner after cancelling the loser");
        assert!((solution.obj - 1.0).abs() <= 1e-9);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn legacy_first_mode_cancels_in_flight_loser_before_returning() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(AsyncCancellationAwareMockSolver {
                name: "legacy-winner".to_owned(),
                result: 1.0,
                wait_for_cancel: false,
                error_on_cancel: false,
            }),
            Arc::new(AsyncCancellationAwareMockSolver {
                name: "legacy-loser".to_owned(),
                result: 2.0,
                wait_for_cancel: true,
                error_on_cancel: true,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::First);
        let solution = solver
            .solve_with_options(
                &LinearTriadModel::new("parallel_legacy_first_cancellation"),
                FrameworkSolveOptions::default(),
            )
            .await
            .expect("legacy First should return the winner after cancelling the loser");
        assert!((solution.obj - 1.0).abs() <= 1e-9);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn report_entry_collects_all_parallel_attempts_and_selects_deterministically() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(MockSolver {
                name: "parallel-first".to_owned(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "parallel-second".to_owned(),
                result: 2.0,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::Best);
        let mut model = LinearTriadModel::new("parallel_report_contract");
        model.objective_category = ospf_rust_core::model::ObjectiveCategory::Minimum;
        let aggregate = solver
            .solve_combinatorial_report_with_options(&model, FrameworkSolveOptions::default())
            .await
            .expect("parallel report entry should aggregate worker reports");

        assert_eq!(aggregate.attempts.len(), 2);
        assert!(
            aggregate
                .attempts
                .iter()
                .all(|attempt| attempt.completed_at_epoch_ms.is_some())
        );
        assert_eq!(
            aggregate.selected_attempt_id.as_deref(),
            Some(
                child_attempt_id(&aggregate_attempt_id(solver.name()), "parallel-first", 0,)
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

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn first_mode_cancels_in_flight_async_loser_and_keeps_cancelled_trace() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(AsyncCancellationAwareMockSolver {
                name: "winner".to_owned(),
                result: 1.0,
                wait_for_cancel: false,
                error_on_cancel: false,
            }),
            Arc::new(AsyncCancellationAwareMockSolver {
                name: "loser".to_owned(),
                result: 2.0,
                wait_for_cancel: true,
                error_on_cancel: false,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::First);
        let mut model = LinearTriadModel::new("parallel_first_async_cancellation_contract");
        model.objective_category = ospf_rust_core::model::ObjectiveCategory::Minimum;

        let aggregate = solver
            .solve_combinatorial_report_with_options(&model, FrameworkSolveOptions::default())
            .await
            .expect("parallel First should return the async winner report");

        assert_eq!(
            aggregate.selected_attempt_id.as_deref(),
            Some(child_attempt_id(&aggregate_attempt_id(solver.name()), "winner", 0).as_str())
        );
        let loser = aggregate
            .attempts
            .iter()
            .find(|attempt| {
                attempt.attempt_id
                    == child_attempt_id(&aggregate_attempt_id(solver.name()), "loser", 1)
            })
            .expect("async loser attempt should remain in the aggregate trace");
        assert_eq!(
            loser.outcome,
            ospf_rust_core::solver::SolveAttemptOutcome::Cancelled
        );
        assert_eq!(
            loser.cancellation_reason.as_deref(),
            Some("FRAMEWORK_LOSER")
        );
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn first_mode_preserves_async_loser_origin_when_backend_returns_error() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(AsyncCancellationAwareMockSolver {
                name: "winner-error-test".to_owned(),
                result: 1.0,
                wait_for_cancel: false,
                error_on_cancel: false,
            }),
            Arc::new(AsyncCancellationAwareMockSolver {
                name: "loser-error-test".to_owned(),
                result: 2.0,
                wait_for_cancel: true,
                error_on_cancel: true,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::First);
        let aggregate = solver
            .solve_combinatorial_report_with_options(
                &LinearTriadModel::new("parallel_async_error_cancellation_contract"),
                FrameworkSolveOptions::default(),
            )
            .await
            .expect("winner report should survive an async cancelled loser error");
        let loser = aggregate
            .attempts
            .iter()
            .find(|attempt| {
                attempt.attempt_id
                    == child_attempt_id(&aggregate_attempt_id(solver.name()), "loser-error-test", 1)
            })
            .expect("async cancelled loser error should remain in the trace");
        assert_eq!(
            loser.outcome,
            ospf_rust_core::solver::SolveAttemptOutcome::Cancelled
        );
        assert_eq!(
            loser.cancellation_reason.as_deref(),
            Some("FRAMEWORK_LOSER")
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_parallel_combinatorial_solver_supports_meta_model_shortcut() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![Arc::new(MockSolver {
            name: "solver_meta".to_string(),
            result: 7.0,
        })];
        let solver = ParallelCombinatorialLinearSolver::with_solvers(solvers);
        let mut meta_model = MetaModel::<f64>::new("parallel_linear_meta_shortcut");
        let x = ContinuousVariableItem::auto("parallel_linear_meta_shortcut_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "parallel_linear_meta_shortcut_c",
            )
            .unwrap();

        let result = solver
            .solve_meta(&meta_model)
            .expect("parallel linear combinatorial solver meta shortcut should succeed");
        assert!((result.obj - 7.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_best_mode_respects_maximum_objective_category() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                result: 2.0,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::Best);
        let mut model = LinearTriadModel::new("max_linear");
        model.objective_category = ospf_rust_core::model::ObjectiveCategory::Maximum;

        let result = solver
            .solve(&model)
            .expect("best mode should return one feasible solution");
        assert!((result.obj - 2.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_meta_model_shortcut_builds_and_solves() {
        let solver = MockSolver {
            name: "meta_shortcut_solver".to_string(),
            result: 3.0,
        };
        let mut meta_model = MetaModel::<f64>::new("linear_meta_shortcut");
        let x = ContinuousVariableItem::auto("linear_meta_shortcut_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "linear_meta_shortcut_c",
            )
            .unwrap();

        let stages = Arc::new(Mutex::new(Vec::new()));
        let stages_for_callback = stages.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            stages_for_callback.lock().unwrap().push(status.stage);
            Ok(())
        });

        let options = FrameworkSolveOptions::new().with_building_callback(Some(callback));
        let result = solver
            .solve_meta_with_options(&meta_model, options)
            .expect("meta-model shortcut solve should succeed");
        assert!((result.obj - 3.0).abs() <= 1e-9);

        let stages = stages.lock().unwrap();
        assert!(stages.contains(&ModelBuildingStage::RegisterTokens));
        assert!(stages.contains(&ModelBuildingStage::FlattenLinearModel));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_meta_model_simplified_alias_builds_and_solves() {
        let solver = MockSolver {
            name: "meta_shortcut_solver_alias".to_string(),
            result: 3.5,
        };
        let mut meta_model = MetaModel::<f64>::new("linear_meta_shortcut_alias");
        let x = ContinuousVariableItem::auto("linear_meta_shortcut_alias_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "linear_meta_shortcut_alias_c",
            )
            .unwrap();

        let result = solver
            .solve_meta(&meta_model)
            .expect("meta-model simplified alias solve should succeed");
        assert!((result.obj - 3.5).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_meta_model_shortcut_rejects_non_finite_value_in_strict_mode() {
        let solver = MockSolver {
            name: "meta_shortcut_solver_non_finite".to_string(),
            result: 3.0,
        };
        let mut meta_model = MetaModel::<f64>::new("linear_meta_shortcut_non_finite");
        let x = ContinuousVariableItem::auto("linear_meta_shortcut_non_finite_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, f64::NAN)],
                ConstraintRelation::LessEqual,
                1.0,
                "linear_meta_shortcut_non_finite_c",
            )
            .unwrap();

        let options = FrameworkSolveOptions::new().with_value_conversion_policy(
            ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
        );
        let error = solver
            .solve_meta_with_options(&meta_model, options)
            .expect_err("strict mode should reject non-finite conversion");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::NonFinite(_))
        ));
    }

    #[cfg(feature = "async")]
    fn assert_send<T: Send>(_: &T) {}

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_meta_model_shortcut_returns_send_future() {
        let solver = MockSolver {
            name: "meta_shortcut_async_solver".to_string(),
            result: 4.0,
        };
        let meta_model = MetaModel::<f64>::new("linear_meta_shortcut_async");

        let future = solver.solve_meta(&meta_model);
        assert_send(&future);
        let result = future
            .await
            .expect("meta-model shortcut async solve should succeed");
        assert!((result.obj - 4.0).abs() <= 1e-9);
    }
}
