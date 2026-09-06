//! 并行组合二次求解器
//! Parallel Combinatorial Quadratic Solver
//!
//! 本模块提供并行执行的组合二次求解器。
//! This module provides parallel-executing combinatorial quadratic solvers.

use super::{
    CombinatorialSelection, FeasibleSolution, FrameworkSolveOptions, ObjectiveCategory,
    ParallelCombinatorialMode, aggregate_combinatorial_reports_with_metadata,
    column_generation_solver::{preserve_cancelled_attempts, report_is_selectable},
    framework_solve_options::{child_cancellation_handle, legacy_cancellation_error},
};
use ospf_rust_core::error::{CoreError, Result, SolverError, SolverNotFoundError};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::intermediate::QuadraticTetradModel;
use ospf_rust_core::solver::{
    CombinatorialSolveReport, SolveReport, SolverProvenance, cancelled_solve_report,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[cfg(feature = "async")]
type AsyncQuadraticMultiSolution<'a> =
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

/// 二次求解器 trait / Quadratic Solver Trait
///
/// 定义二次求解器的接口。
/// Defines interface for quadratic solvers.
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait QuadraticSolver: Send + Sync {
    /// 获取求解器名称 / Get solver name
    fn name(&self) -> &str;

    /// 求解二次模型 / Solve quadratic model
    #[cfg(feature = "async")]
    async fn solve(&self, model: &QuadraticTetradModel) -> Result<FeasibleSolution>;

    /// 使用参数对象求解二次模型 / Solve quadratic model with options object
    #[cfg(feature = "async")]
    fn solve_with_options<'a>(
        &'a self,
        model: &'a QuadraticTetradModel,
        _options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    {
        Box::pin(async move { self.solve(model).await })
    }

    /// 以统一组合报告求解二次模型 / Solve a quadratic model as a combinatorial unified report.
    #[cfg(feature = "async")]
    async fn solve_combinatorial_report_with_options(
        &self,
        model: &QuadraticTetradModel,
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

    /// 以统一报告求解二次模型 / Solve a quadratic model as a unified report.
    #[cfg(feature = "async")]
    async fn solve_report_with_options(
        &self,
        model: &QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_combinatorial_report_with_options(model, options)
            .await
            .map(|aggregate| aggregate.report)
    }

    /// 求解二次模型（同步）/ Solve quadratic model (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve(&self, model: &QuadraticTetradModel) -> Result<FeasibleSolution>;

    /// 使用参数对象求解二次模型（同步）/ Solve quadratic model with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_with_options(
        &self,
        model: &QuadraticTetradModel,
        _options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve(model)
    }

    /// 以统一组合报告求解二次模型 / Solve a quadratic model as a combinatorial unified report.
    #[cfg(not(feature = "async"))]
    fn solve_combinatorial_report_with_options(
        &self,
        model: &QuadraticTetradModel,
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

    /// 以统一报告求解二次模型 / Solve a quadratic model as a unified report.
    #[cfg(not(feature = "async"))]
    fn solve_report_with_options(
        &self,
        model: &QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.solve_combinatorial_report_with_options(model, options)
            .map(|aggregate| aggregate.report)
    }

    /// 求解二次模型并返回多个解（参数对象）/
    /// Solve quadratic model and return multiple solutions with options object
    #[cfg(feature = "async")]
    fn solve_multi_with_options<'a>(
        &'a self,
        model: &'a QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> AsyncQuadraticMultiSolution<'a> {
        Box::pin(async move {
            let result = self.solve_with_options(model, options).await?;
            Ok((result.clone(), vec![result.solution]))
        })
    }

    /// 求解二次模型并返回多个解（参数对象，同步）/
    /// Solve quadratic model and return multiple solutions with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_multi_with_options(
        &self,
        model: &QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
        let result = self.solve_with_options(model, options)?;
        Ok((result.clone(), vec![result.solution]))
    }
}

/// 二次求解器 MetaModel 扩展入口 / MetaModel extension entry for quadratic solvers
///
/// 推荐应用层入口：`solve_meta` 与 `solve_meta_with_options`。
/// Recommended application-facing entries are `solve_meta` and `solve_meta_with_options`.
#[cfg(feature = "async")]
pub trait QuadraticMetaModelSolverExt: QuadraticSolver {
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
        let tetrad_model = match mechanism_model
            .try_into_quadratic_tetrad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        Box::pin(async move { self.solve_with_options(&tetrad_model, options).await })
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
        let tetrad_model = match mechanism_model
            .try_into_quadratic_tetrad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(error) => return Box::pin(std::future::ready(Err(error))),
        };
        Box::pin(async move { self.solve_report_with_options(&tetrad_model, options).await })
    }
}

#[cfg(feature = "async")]
impl<T> QuadraticMetaModelSolverExt for T where T: QuadraticSolver + ?Sized {}

/// 二次求解器 MetaModel 扩展入口 / MetaModel extension entry for quadratic solvers
///
/// 推荐应用层入口：`solve_meta` 与 `solve_meta_with_options`。
/// Recommended application-facing entries are `solve_meta` and `solve_meta_with_options`.
#[cfg(not(feature = "async"))]
pub trait QuadraticMetaModelSolverExt: QuadraticSolver {
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
        let tetrad_model = mechanism_model.try_into_quadratic_tetrad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        self.solve_with_options(&tetrad_model, options)
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
        let tetrad_model = mechanism_model.try_into_quadratic_tetrad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        self.solve_report_with_options(&tetrad_model, options)
    }
}

#[cfg(not(feature = "async"))]
impl<T> QuadraticMetaModelSolverExt for T where T: QuadraticSolver + ?Sized {}

/// 并行组合二次求解器 / Parallel Combinatorial Quadratic Solver
///
/// 组合多个二次求解器并行执行，根据模式选择结果。
/// Combines multiple quadratic solvers to execute in parallel, selecting result based on mode.
pub struct ParallelCombinatorialQuadraticSolver {
    /// 求解器列表 / Solver list
    solvers: Vec<Arc<dyn QuadraticSolver>>,
    /// 组合模式 / Combinatorial mode
    mode: ParallelCombinatorialMode,
    /// 名称缓存 / Cached name
    name: String,
}

impl ParallelCombinatorialQuadraticSolver {
    /// 创建新的并行组合二次求解器 / Create new parallel combinatorial quadratic solver
    pub fn new(solvers: Vec<Arc<dyn QuadraticSolver>>, mode: ParallelCombinatorialMode) -> Self {
        let names: Vec<&str> = solvers.iter().map(|s| s.name()).collect();
        let name = format!("ParallelCombinatorial({})", names.join(","));
        Self {
            solvers,
            mode,
            name,
        }
    }

    /// 使用默认模式创建 / Create with default mode
    pub fn with_solvers(solvers: Vec<Arc<dyn QuadraticSolver>>) -> Self {
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

    fn objective_category(model: &QuadraticTetradModel) -> ObjectiveCategory {
        match model.objective_category {
            ospf_rust_core::model::ObjectiveCategory::Minimum => ObjectiveCategory::Minimum,
            ospf_rust_core::model::ObjectiveCategory::Maximum => ObjectiveCategory::Maximum,
        }
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl QuadraticSolver for ParallelCombinatorialQuadraticSolver {
    fn name(&self) -> &str {
        &self.name
    }

    async fn solve_combinatorial_report_with_options(
        &self,
        model: &QuadraticTetradModel,
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
                            "quadratic combinatorial worker panicked".to_owned(),
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

    async fn solve(&self, model: &QuadraticTetradModel) -> Result<FeasibleSolution> {
        self.solve_with_options(model, FrameworkSolveOptions::default())
            .await
    }

    fn solve_with_options<'a>(
        &'a self,
        model: &'a QuadraticTetradModel,
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
        model: &'a QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(FeasibleSolution, Vec<Vec<f64>>)>> + Send + 'a,
        >,
    > {
        Box::pin(async move {
            let result = QuadraticSolver::solve_with_options(self, model, options).await?;
            Ok((result.clone(), vec![result.solution]))
        })
    }
}

#[cfg(not(feature = "async"))]
impl QuadraticSolver for ParallelCombinatorialQuadraticSolver {
    fn name(&self) -> &str {
        &self.name
    }

    fn solve_combinatorial_report_with_options(
        &self,
        model: &QuadraticTetradModel,
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
                        "quadratic combinatorial worker panicked".to_owned(),
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

    fn solve(&self, model: &QuadraticTetradModel) -> Result<FeasibleSolution> {
        self.solve_with_options(model, FrameworkSolveOptions::default())
    }

    fn solve_with_options(
        &self,
        model: &QuadraticTetradModel,
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
        model: &QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
        let result = QuadraticSolver::solve_with_options(self, model, options)?;
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

    struct LegacyCancellationAwareMockSolver {
        name: String,
        result: f64,
        wait_for_cancel: bool,
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl QuadraticSolver for LegacyCancellationAwareMockSolver {
        fn name(&self) -> &str {
            &self.name
        }

        #[cfg(feature = "async")]
        async fn solve(&self, _model: &QuadraticTetradModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(not(feature = "async"))]
        fn solve(&self, _model: &QuadraticTetradModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(feature = "async")]
        fn solve_with_options<'a>(
            &'a self,
            _model: &'a QuadraticTetradModel,
            options: FrameworkSolveOptions,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>,
        > {
            let wait_for_cancel = self.wait_for_cancel;
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

        #[cfg(not(feature = "async"))]
        fn solve_with_options(
            &self,
            _model: &QuadraticTetradModel,
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
                return Err(CoreError::cancelled(
                    handle
                        .cancellation()
                        .map(|record| record.origin.to_string())
                        .unwrap_or_else(|| "UNKNOWN".to_owned()),
                ));
            }
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl QuadraticSolver for MockSolver {
        fn name(&self) -> &str {
            &self.name
        }

        #[cfg(feature = "async")]
        async fn solve(&self, _model: &QuadraticTetradModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(not(feature = "async"))]
        fn solve(&self, _model: &QuadraticTetradModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(feature = "async")]
        fn solve_multi_with_options<'a>(
            &'a self,
            model: &'a QuadraticTetradModel,
            options: FrameworkSolveOptions,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<(FeasibleSolution, Vec<Vec<f64>>)>>
                    + Send
                    + 'a,
            >,
        > {
            Box::pin(async move {
                let result = QuadraticSolver::solve_with_options(self, model, options).await?;
                Ok((result.clone(), vec![result.solution]))
            })
        }

        #[cfg(not(feature = "async"))]
        fn solve_multi_with_options(
            &self,
            model: &QuadraticTetradModel,
            options: FrameworkSolveOptions,
        ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
            let result = QuadraticSolver::solve_with_options(self, model, options)?;
            Ok((result.clone(), vec![result.solution]))
        }
    }

    #[test]
    fn test_parallel_combinatorial_quadratic_solver() {
        let solvers: Vec<Arc<dyn QuadraticSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                result: 2.0,
            }),
        ];

        let solver = ParallelCombinatorialQuadraticSolver::with_solvers(solvers);
        assert!(solver.name().contains("solver1"));
        assert!(solver.name().contains("solver2"));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_parallel_combinatorial_solver_supports_meta_model_shortcut() {
        let solvers: Vec<Arc<dyn QuadraticSolver>> = vec![Arc::new(MockSolver {
            name: "solver_meta".to_string(),
            result: 8.0,
        })];
        let solver = ParallelCombinatorialQuadraticSolver::with_solvers(solvers);
        let mut meta_model = MetaModel::<f64>::new("parallel_quadratic_meta_shortcut");
        let x = ContinuousVariableItem::auto("parallel_quadratic_meta_shortcut_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "parallel_quadratic_meta_shortcut_c",
            )
            .unwrap();

        let result = solver
            .solve_meta(&meta_model)
            .expect("parallel quadratic combinatorial solver meta shortcut should succeed");
        assert!((result.obj - 8.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_best_mode_respects_maximum_objective_category() {
        let solvers: Vec<Arc<dyn QuadraticSolver>> = vec![
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
            ParallelCombinatorialQuadraticSolver::new(solvers, ParallelCombinatorialMode::Best);
        let mut model = QuadraticTetradModel::new("max_quadratic");
        model.objective_category = ospf_rust_core::model::ObjectiveCategory::Maximum;

        let result = solver
            .solve(&model)
            .expect("best mode should return one feasible solution");
        assert!((result.obj - 2.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn legacy_first_mode_cancels_in_flight_loser_before_returning() {
        let solvers: Vec<Arc<dyn QuadraticSolver>> = vec![
            Arc::new(LegacyCancellationAwareMockSolver {
                name: "legacy-winner".to_owned(),
                result: 1.0,
                wait_for_cancel: false,
            }),
            Arc::new(LegacyCancellationAwareMockSolver {
                name: "legacy-loser".to_owned(),
                result: 2.0,
                wait_for_cancel: true,
            }),
        ];
        let solver =
            ParallelCombinatorialQuadraticSolver::new(solvers, ParallelCombinatorialMode::First);
        let solution = solver
            .solve_with_options(
                &QuadraticTetradModel::new("parallel_quadratic_legacy_cancellation"),
                FrameworkSolveOptions::default(),
            )
            .expect("legacy First should return the winner after cancelling the loser");
        assert!((solution.obj - 1.0).abs() <= 1e-9);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn legacy_first_mode_cancels_in_flight_loser_before_returning() {
        let solvers: Vec<Arc<dyn QuadraticSolver>> = vec![
            Arc::new(LegacyCancellationAwareMockSolver {
                name: "legacy-winner".to_owned(),
                result: 1.0,
                wait_for_cancel: false,
            }),
            Arc::new(LegacyCancellationAwareMockSolver {
                name: "legacy-loser".to_owned(),
                result: 2.0,
                wait_for_cancel: true,
            }),
        ];
        let solver =
            ParallelCombinatorialQuadraticSolver::new(solvers, ParallelCombinatorialMode::First);
        let solution = solver
            .solve_with_options(
                &QuadraticTetradModel::new("parallel_quadratic_legacy_cancellation"),
                FrameworkSolveOptions::default(),
            )
            .await
            .expect("legacy First should return the winner after cancelling the loser");
        assert!((solution.obj - 1.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_meta_model_shortcut_builds_and_solves() {
        let solver = MockSolver {
            name: "quadratic_meta_shortcut_solver".to_string(),
            result: 6.0,
        };
        let mut meta_model = MetaModel::<f64>::new("quadratic_meta_shortcut");
        let x = ContinuousVariableItem::auto("quadratic_meta_shortcut_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "quadratic_meta_shortcut_c",
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
            .expect("meta-model quadratic shortcut solve should succeed");
        assert!((result.obj - 6.0).abs() <= 1e-9);

        let stages = stages.lock().unwrap();
        assert!(stages.contains(&ModelBuildingStage::RegisterTokens));
        assert!(stages.contains(&ModelBuildingStage::FlattenQuadraticModel));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_meta_model_simplified_alias_builds_and_solves() {
        let solver = MockSolver {
            name: "quadratic_meta_shortcut_solver_alias".to_string(),
            result: 6.5,
        };
        let mut meta_model = MetaModel::<f64>::new("quadratic_meta_shortcut_alias");
        let x = ContinuousVariableItem::auto("quadratic_meta_shortcut_alias_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "quadratic_meta_shortcut_alias_c",
            )
            .unwrap();

        let result = solver
            .solve_meta(&meta_model)
            .expect("meta-model quadratic simplified alias solve should succeed");
        assert!((result.obj - 6.5).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_meta_model_shortcut_rejects_non_finite_value_in_strict_mode() {
        let solver = MockSolver {
            name: "quadratic_meta_shortcut_solver_non_finite".to_string(),
            result: 6.0,
        };
        let mut meta_model = MetaModel::<f64>::new("quadratic_meta_shortcut_non_finite");
        let x = ContinuousVariableItem::auto("quadratic_meta_shortcut_non_finite_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, f64::NAN)],
                ConstraintRelation::LessEqual,
                1.0,
                "quadratic_meta_shortcut_non_finite_c",
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
            name: "quadratic_meta_shortcut_async_solver".to_string(),
            result: 5.0,
        };
        let meta_model = MetaModel::<f64>::new("quadratic_meta_shortcut_async");

        let future = solver.solve_meta(&meta_model);
        assert_send(&future);
        let result = future
            .await
            .expect("meta-model quadratic shortcut async solve should succeed");
        assert!((result.obj - 5.0).abs() <= 1e-9);
    }
}
