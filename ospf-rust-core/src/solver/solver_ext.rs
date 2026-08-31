//! 求解器扩展入口 / Solver Extension Entry Points

#[cfg(feature = "async")]
use super::CancellationOrigin;
use super::value::boundary::{value_from_backend_f64, value_to_backend_f64};
use super::value::conversion_context::SolveValueConversionContext;
use super::{
    FeasibleSolverOutput, InfeasibilityEvidence, InfeasibilityEvidenceSource,
    InfeasibilityMinimality, ProofCompleteness, ProofReliability, SolveHandle, SolveIssue,
    SolveProgressReporter, SolveReport, SolveValue, SolveValueConversionPolicy, Solver,
    SolverOutput, SolverOutputWithIIS, SolvingStatusCallback, attach_linear_model_mapping,
    attach_quadratic_model_mapping, convert_report_value, solve_report_to_solver_output,
    solver_output_to_report,
};
use crate::error::Result;
use crate::model::intermediate::{LinearTriadModel, QuadraticTetradModel};
use crate::model::mechanism::{
    BasicMechanismModel, LinearConstraint, LinearInequality, MechanismModel, QuadraticConstraint,
    QuadraticInequality,
};
use crate::model::{MetaModel, ModelBuildingStatusCallback};
use crate::model::{Objective, SubObjective};
use crate::solver::iis::LinearIISModel;
use crate::solver::iis::{IISConfig, compute_iis};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use crate::token::{AnyVariable, Token, TokenVariableData};
use std::collections::BTreeSet;
#[cfg(feature = "async")]
use std::future::Future;
use std::ops::Add;
#[cfg(feature = "async")]
use std::pin::Pin;
#[cfg(feature = "async")]
use std::sync::Arc;
#[cfg(feature = "async")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "async")]
use std::task::{Context, Poll};
use std::time::Duration;

/// 统一求解参数 / Unified solve options
#[derive(Clone, Copy, Default)]
pub struct SolveOptions<'a> {
    /// 期望的解数量（多解接口）/ Expected solution amount (for multi-solution APIs)
    pub solution_amount: usize,
    /// 单次求解时间上限 / Per-solve time limit.
    pub time_limit: Option<Duration>,
    /// 单次求解节点上限 / Per-solve node limit.
    pub node_limit: Option<usize>,
    /// 单次求解可行解数量上限 / Per-solve feasible-solution limit.
    pub solution_limit: Option<usize>,
    /// 建模阶段回调 / Model-building status callback
    pub model_building_status_callback: Option<&'a ModelBuildingStatusCallback>,
    /// 求解阶段回调 / Solving status callback
    pub solving_status_callback: Option<&'a SolvingStatusCallback>,
    /// 数值转换策略 / Numeric conversion policy
    pub value_conversion_policy: SolveValueConversionPolicy,
    /// 独立取消句柄 / Independent cancellation handle
    pub cancellation_handle: Option<&'a SolveHandle>,
    /// 统一进度上报器 / Unified progress reporter
    pub progress_reporter: Option<&'a SolveProgressReporter>,
}

/// 统一求解参数构建器 / Unified solve options builder
#[derive(Clone, Copy, Default)]
pub struct SolveOptionsBuilder<'a> {
    solution_amount: usize,
    time_limit: Option<Duration>,
    node_limit: Option<usize>,
    solution_limit: Option<usize>,
    model_building_status_callback: Option<&'a ModelBuildingStatusCallback>,
    solving_status_callback: Option<&'a SolvingStatusCallback>,
    value_conversion_policy: SolveValueConversionPolicy,
    cancellation_handle: Option<&'a SolveHandle>,
    progress_reporter: Option<&'a SolveProgressReporter>,
}

impl<'a> SolveOptionsBuilder<'a> {
    /// 创建构建器 / Create builder
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置解数量 / Set solution amount
    pub fn solution_amount(mut self, solution_amount: usize) -> Self {
        self.solution_amount = solution_amount.max(1);
        self
    }

    /// 设置单次求解时间上限 / Set the per-solve time limit.
    pub fn time_limit(mut self, limit: Option<Duration>) -> Self {
        self.time_limit = limit;
        self
    }

    /// 设置单次求解节点上限 / Set the per-solve node limit.
    pub fn node_limit(mut self, limit: Option<usize>) -> Self {
        self.node_limit = limit;
        self
    }

    /// 设置单次求解可行解数量上限 / Set the per-solve feasible-solution limit.
    pub fn solution_limit(mut self, limit: Option<usize>) -> Self {
        self.solution_limit = limit;
        self
    }

    /// 设置建模回调 / Set model-building callback
    pub fn building_callback(mut self, callback: Option<&'a ModelBuildingStatusCallback>) -> Self {
        self.model_building_status_callback = callback;
        self
    }

    /// 设置求解回调 / Set solving callback
    pub fn solving_callback(mut self, callback: Option<&'a SolvingStatusCallback>) -> Self {
        self.solving_status_callback = callback;
        self
    }

    /// 设置数值转换策略 / Set numeric conversion policy
    pub fn value_conversion_policy(
        mut self,
        value_conversion_policy: SolveValueConversionPolicy,
    ) -> Self {
        self.value_conversion_policy = value_conversion_policy;
        self
    }

    /// 设置取消句柄 / Set cancellation handle
    pub fn cancellation_handle(mut self, handle: Option<&'a SolveHandle>) -> Self {
        self.cancellation_handle = handle;
        self
    }

    /// 设置统一进度上报器 / Set the unified progress reporter
    pub fn progress_reporter(mut self, reporter: Option<&'a SolveProgressReporter>) -> Self {
        self.progress_reporter = reporter;
        self
    }

    /// 构建参数对象 / Build options object
    pub fn finish(self) -> SolveOptions<'a> {
        SolveOptions {
            solution_amount: self.solution_amount,
            time_limit: self.time_limit,
            node_limit: self.node_limit,
            solution_limit: self.solution_limit,
            model_building_status_callback: self.model_building_status_callback,
            solving_status_callback: self.solving_status_callback,
            value_conversion_policy: self.value_conversion_policy,
            cancellation_handle: self.cancellation_handle,
            progress_reporter: self.progress_reporter,
        }
    }
}

impl<'a> SolveOptions<'a> {
    /// 创建默认配置 / Create default options
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建求解参数构建器 / Create solve options builder
    pub fn builder() -> SolveOptionsBuilder<'a> {
        SolveOptionsBuilder::new()
    }

    /// 通过闭包配置求解参数 / Build solve options with a configuration closure
    pub fn build<F>(configure: F) -> Self
    where
        F: FnOnce(SolveOptionsBuilder<'a>) -> SolveOptionsBuilder<'a>,
    {
        configure(Self::builder()).finish()
    }

    /// 设置解数量 / Set solution amount
    pub fn with_solution_amount(mut self, solution_amount: usize) -> Self {
        self.solution_amount = solution_amount.max(1);
        self
    }

    /// 设置单次求解时间上限 / Set the per-solve time limit.
    pub fn with_time_limit(mut self, limit: Option<Duration>) -> Self {
        self.time_limit = limit;
        self
    }

    /// 设置单次求解节点上限 / Set the per-solve node limit.
    pub fn with_node_limit(mut self, limit: Option<usize>) -> Self {
        self.node_limit = limit;
        self
    }

    /// 设置单次求解可行解数量上限 / Set the per-solve feasible-solution limit.
    pub fn with_solution_limit(mut self, limit: Option<usize>) -> Self {
        self.solution_limit = limit;
        self
    }

    /// 设置建模回调 / Set model-building callback
    pub fn with_building_callback(
        mut self,
        callback: Option<&'a ModelBuildingStatusCallback>,
    ) -> Self {
        self.model_building_status_callback = callback;
        self
    }

    /// 设置求解回调 / Set solving callback
    pub fn with_solving_callback(mut self, callback: Option<&'a SolvingStatusCallback>) -> Self {
        self.solving_status_callback = callback;
        self
    }

    /// 设置数值转换策略 / Set numeric conversion policy
    pub fn with_value_conversion_policy(
        mut self,
        value_conversion_policy: SolveValueConversionPolicy,
    ) -> Self {
        self.value_conversion_policy = value_conversion_policy;
        self
    }

    /// 设置取消句柄 / Set cancellation handle
    pub fn with_cancellation_handle(mut self, handle: Option<&'a SolveHandle>) -> Self {
        self.cancellation_handle = handle;
        self
    }

    /// 设置统一进度上报器 / Set the unified progress reporter
    pub fn with_progress_reporter(mut self, reporter: Option<&'a SolveProgressReporter>) -> Self {
        self.progress_reporter = reporter;
        self
    }
}

/// 异步求解参数 / Async solve options
#[cfg(feature = "async")]
#[derive(Clone, Default)]
pub struct AsyncSolveOptions {
    /// 期望的解数量 / Expected solution amount
    pub solution_amount: usize,
    /// 单次求解时间上限 / Per-solve time limit.
    pub time_limit: Option<Duration>,
    /// 单次求解节点上限 / Per-solve node limit.
    pub node_limit: Option<usize>,
    /// 单次求解可行解数量上限 / Per-solve feasible-solution limit.
    pub solution_limit: Option<usize>,
    /// 建模阶段回调 / Model-building status callback
    pub model_building_status_callback: Option<ModelBuildingStatusCallback>,
    /// 求解阶段回调 / Solving status callback
    pub solving_status_callback: Option<SolvingStatusCallback>,
    /// 数值转换策略 / Numeric conversion policy
    pub value_conversion_policy: SolveValueConversionPolicy,
    /// 独立取消句柄 / Independent cancellation handle
    pub cancellation_handle: Option<SolveHandle>,
    /// 统一进度上报器 / Unified progress reporter
    pub progress_reporter: Option<SolveProgressReporter>,
}

#[cfg(feature = "async")]
impl AsyncSolveOptions {
    /// 创建默认配置 / Create default options
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置建模回调 / Set model-building callback
    pub fn with_building_callback(mut self, callback: Option<ModelBuildingStatusCallback>) -> Self {
        self.model_building_status_callback = callback;
        self
    }

    /// 设置解数量 / Set solution amount
    pub fn with_solution_amount(mut self, solution_amount: usize) -> Self {
        self.solution_amount = solution_amount.max(1);
        self
    }

    /// 设置单次求解时间上限 / Set the per-solve time limit.
    pub fn with_time_limit(mut self, limit: Option<Duration>) -> Self {
        self.time_limit = limit;
        self
    }

    /// 设置单次求解节点上限 / Set the per-solve node limit.
    pub fn with_node_limit(mut self, limit: Option<usize>) -> Self {
        self.node_limit = limit;
        self
    }

    /// 设置单次求解可行解数量上限 / Set the per-solve feasible-solution limit.
    pub fn with_solution_limit(mut self, limit: Option<usize>) -> Self {
        self.solution_limit = limit;
        self
    }

    /// 设置求解回调 / Set solving callback
    pub fn with_solving_callback(mut self, callback: Option<SolvingStatusCallback>) -> Self {
        self.solving_status_callback = callback;
        self
    }

    /// 设置数值转换策略 / Set numeric conversion policy
    pub fn with_value_conversion_policy(
        mut self,
        value_conversion_policy: SolveValueConversionPolicy,
    ) -> Self {
        self.value_conversion_policy = value_conversion_policy;
        self
    }

    /// 设置取消句柄 / Set cancellation handle
    pub fn with_cancellation_handle(mut self, handle: Option<SolveHandle>) -> Self {
        self.cancellation_handle = handle;
        self
    }

    /// 设置统一进度上报器 / Set the unified progress reporter
    pub fn with_progress_reporter(mut self, reporter: Option<SolveProgressReporter>) -> Self {
        self.progress_reporter = reporter;
        self
    }

    fn as_solve_options(&self) -> SolveOptions<'_> {
        SolveOptions::new()
            .with_solution_amount(self.solution_amount)
            .with_time_limit(self.time_limit)
            .with_node_limit(self.node_limit)
            .with_solution_limit(self.solution_limit)
            .with_building_callback(self.model_building_status_callback.as_ref())
            .with_solving_callback(self.solving_status_callback.as_ref())
            .with_value_conversion_policy(self.value_conversion_policy)
            .with_cancellation_handle(self.cancellation_handle.as_ref())
            .with_progress_reporter(self.progress_reporter.as_ref())
    }

    fn as_solving_solve_options(&self) -> SolveOptions<'_> {
        SolveOptions::new()
            .with_solution_amount(self.solution_amount)
            .with_time_limit(self.time_limit)
            .with_node_limit(self.node_limit)
            .with_solution_limit(self.solution_limit)
            .with_solving_callback(self.solving_status_callback.as_ref())
            .with_cancellation_handle(self.cancellation_handle.as_ref())
            .with_progress_reporter(self.progress_reporter.as_ref())
    }
}

#[cfg(feature = "async")]
enum PreparedSolveModel {
    Linear(LinearTriadModel),
    Quadratic(QuadraticTetradModel),
}

#[cfg(feature = "async")]
fn prepare_solve_model<V>(
    model: &MetaModel<V>,
    options: &AsyncSolveOptions,
) -> Result<PreparedSolveModel>
where
    V: SolveValue + Add<Output = V>,
{
    let solve_options = options.as_solve_options();
    let mechanism_model = model.try_to_mechanism_model_with_status_callback(
        solve_options.model_building_status_callback,
    )?;
    let mechanism_model =
        convert_mechanism_model_to_f64(&mechanism_model, solve_options.value_conversion_policy)?;
    if mechanism_model.num_quadratic_constraints() > 0 {
        let quadratic_model = mechanism_model
            .try_into_quadratic_tetrad_model_with_status_callback(
                solve_options.model_building_status_callback,
            )?;
        return Ok(PreparedSolveModel::Quadratic(quadratic_model));
    }

    let linear_model = mechanism_model.try_into_linear_triad_model_with_status_callback(
        solve_options.model_building_status_callback,
    )?;
    Ok(PreparedSolveModel::Linear(linear_model))
}

/// 可请求 backend 取消的后台求解句柄 / Background solve handle with backend cancellation.
#[cfg(feature = "async")]
pub struct SolveTaskHandle<T> {
    inner: tokio::task::JoinHandle<Result<T>>,
    cancellation_handle: SolveHandle,
    finished: Arc<AtomicBool>,
}

#[cfg(feature = "async")]
impl<T> Unpin for SolveTaskHandle<T> {}

#[cfg(feature = "async")]
impl<T> Future for SolveTaskHandle<T> {
    type Output = std::result::Result<Result<T>, tokio::task::JoinError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let result = Pin::new(&mut self.inner).poll(context);
        if result.is_ready() {
            self.finished.store(true, Ordering::SeqCst);
        }
        result
    }
}

#[cfg(feature = "async")]
impl<T> SolveTaskHandle<T> {
    fn new(
        inner: tokio::task::JoinHandle<Result<T>>,
        cancellation_handle: SolveHandle,
        finished: Arc<AtomicBool>,
    ) -> Self {
        Self {
            inner,
            cancellation_handle,
            finished,
        }
    }

    /// 获取本次求解的独立取消句柄 / Get the independent cancellation handle for this solve.
    pub fn cancellation_handle(&self) -> &SolveHandle {
        &self.cancellation_handle
    }

    /// 请求取消但保留任务以便后续等待 / Request cancellation while retaining the task for a later join.
    pub fn cancel(&self) -> bool {
        self.cancellation_handle
            .cancel(CancellationOrigin::External)
    }

    /// 兼容 Tokio `abort` 名称，但先请求 backend 中断而不丢弃 blocking task / Keep the Tokio `abort` spelling while requesting backend interruption instead of dropping the blocking task.
    pub fn abort(&self) {
        self.cancellation_handle
            .cancel(CancellationOrigin::TokioTaskAbort);
    }

    /// 请求取消并等待 backend 和 blocking task 释放资源 / Request cancellation and await backend/task cleanup.
    pub async fn cancel_and_wait(self) -> std::result::Result<Result<T>, tokio::task::JoinError> {
        self.cancel();
        self.await
    }

    /// 查询 blocking task 是否已完成 / Check whether the blocking task has finished.
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }
}

#[cfg(feature = "async")]
impl<T> Drop for SolveTaskHandle<T> {
    fn drop(&mut self) {
        if !self.finished.load(Ordering::SeqCst) && !self.inner.is_finished() {
            self.cancellation_handle
                .cancel(CancellationOrigin::TokioTaskAbort);
        }
    }
}

/// 旧线性求解任务句柄别名 / Legacy linear solve task-handle alias.
#[cfg(feature = "async")]
pub type SolveJoinHandle = SolveTaskHandle<SolverOutput>;

/// 统一报告后台求解任务句柄别名 / Unified-report solve task-handle alias.
#[cfg(feature = "async")]
pub type SolveReportJoinHandle<V> = SolveTaskHandle<SolveReport<V>>;

#[cfg(feature = "async")]
fn ensure_async_cancellation_handle(
    mut options: AsyncSolveOptions,
) -> (AsyncSolveOptions, SolveHandle) {
    let cancellation_handle = options
        .cancellation_handle
        .clone()
        .unwrap_or_else(SolveHandle::new);
    options.cancellation_handle = Some(cancellation_handle.clone());
    (options, cancellation_handle)
}

/// 在后台线程池中求解并返回统一报告 / Solve on the blocking pool and return a unified report
#[cfg(feature = "async")]
pub fn spawn_solve_report<S, V>(solver: Arc<S>, model: MetaModel<V>) -> SolveReportJoinHandle<V>
where
    S: SolverExt + Send + Sync + 'static,
    V: SolveValue + Add<Output = V>,
{
    spawn_solve_report_with_options(solver, model, AsyncSolveOptions::default())
}

/// 统一报告及其不可行诊断 / Unified report with infeasibility diagnostics
#[derive(Debug, Clone)]
pub struct SolveReportWithIIS<V> {
    /// 求解报告 / Solve report
    pub report: SolveReport<V>,
    /// 可选 IIS 结果 / Optional IIS result
    pub iis: Option<LinearIISModel>,
}

/// 在后台线程池中求解并返回统一报告（参数对象） / Solve on the blocking pool and return a unified report with options.
#[cfg(feature = "async")]
pub fn spawn_solve_report_with_options<S, V>(
    solver: Arc<S>,
    model: MetaModel<V>,
    options: AsyncSolveOptions,
) -> SolveReportJoinHandle<V>
where
    S: SolverExt + Send + Sync + 'static,
    V: SolveValue + Add<Output = V>,
{
    let (options, cancellation_handle) = ensure_async_cancellation_handle(options);
    let prepared_model = prepare_solve_model(&model, &options);
    let finished = Arc::new(AtomicBool::new(false));
    let finished_for_task = Arc::clone(&finished);
    let completion_handle = cancellation_handle.clone();
    let inner = tokio::task::spawn_blocking(move || {
        let policy = options.value_conversion_policy;
        let report = match prepared_model {
            Ok(PreparedSolveModel::Linear(model)) => {
                solver.solve_linear_report_with_options(&model, &options.as_solving_solve_options())
            }
            Ok(PreparedSolveModel::Quadratic(model)) => solver
                .solve_quadratic_report_with_options(&model, &options.as_solving_solve_options()),
            Err(error) => Err(error),
        };
        completion_handle.mark_completed();
        finished_for_task.store(true, Ordering::SeqCst);
        report.and_then(|report| convert_report_value(report, policy))
    });
    SolveTaskHandle::new(inner, cancellation_handle, finished)
}

/// 在后台阻塞线程池中求解模型 / Solve model on the blocking thread pool
#[cfg(feature = "async")]
pub fn spawn_solve<S, V>(solver: Arc<S>, model: MetaModel<V>) -> SolveJoinHandle
where
    S: SolverExt + Send + Sync + 'static,
    V: SolveValue + Add<Output = V>,
{
    spawn_solve_with_options(solver, model, AsyncSolveOptions::default())
}

/// 在后台阻塞线程池中求解模型（参数对象） / Solve model on the blocking thread pool (options object)
#[cfg(feature = "async")]
pub fn spawn_solve_with_options<S, V>(
    solver: Arc<S>,
    model: MetaModel<V>,
    options: AsyncSolveOptions,
) -> SolveJoinHandle
where
    S: SolverExt + Send + Sync + 'static,
    V: SolveValue + Add<Output = V>,
{
    let (options, cancellation_handle) = ensure_async_cancellation_handle(options);
    let prepared_model = prepare_solve_model(&model, &options);
    let finished = Arc::new(AtomicBool::new(false));
    let finished_for_task = Arc::clone(&finished);
    let completion_handle = cancellation_handle.clone();
    let inner = tokio::task::spawn_blocking(move || {
        let result = match prepared_model {
            Ok(PreparedSolveModel::Linear(model)) => {
                let solve_options = options.as_solving_solve_options();
                solver.solve_linear_with_options(&model, &solve_options)
            }
            Ok(PreparedSolveModel::Quadratic(model)) => {
                let solve_options = options.as_solving_solve_options();
                solver.solve_quadratic_with_options(&model, &solve_options)
            }
            Err(error) => Err(error),
        };
        completion_handle.mark_completed();
        finished_for_task.store(true, Ordering::SeqCst);
        result
    });
    SolveTaskHandle::new(inner, cancellation_handle, finished)
}

/// 在后台阻塞线程池中求解模型（求解回调） / Solve model on the blocking thread pool (solving callback)
#[cfg(feature = "async")]
pub fn spawn_solve_with_callback<S, V>(
    solver: Arc<S>,
    model: MetaModel<V>,
    callback: SolvingStatusCallback,
) -> SolveJoinHandle
where
    S: SolverExt + Send + Sync + 'static,
    V: SolveValue + Add<Output = V>,
{
    spawn_solve_with_options(
        solver,
        model,
        AsyncSolveOptions::new().with_solving_callback(Some(callback)),
    )
}

/// 异步求解模型（求解回调） / Solve model asynchronously (solving callback)
#[cfg(feature = "async")]
pub async fn solve_async_with_callback<S, V>(
    solver: Arc<S>,
    model: MetaModel<V>,
    callback: SolvingStatusCallback,
) -> Result<SolverOutput>
where
    S: SolverExt + Send + Sync + 'static,
    V: SolveValue + Add<Output = V>,
{
    solve_async_with_options(
        solver,
        model,
        AsyncSolveOptions::new().with_solving_callback(Some(callback)),
    )
    .await
}

/// 异步求解模型（参数对象） / Solve model asynchronously (options object)
#[cfg(feature = "async")]
pub async fn solve_async_with_options<S, V>(
    solver: Arc<S>,
    model: MetaModel<V>,
    options: AsyncSolveOptions,
) -> Result<SolverOutput>
where
    S: SolverExt + Send + Sync + 'static,
    V: SolveValue + Add<Output = V>,
{
    spawn_solve_with_options(solver, model, options)
        .await
        .map_err(|error| {
            crate::error::CoreError::Internal(format!("async solve task failed: {}", error))
        })?
}

/// 异步求解并返回统一报告（求解回调） / Solve asynchronously and return a unified report with a solving callback.
#[cfg(feature = "async")]
pub async fn solve_async_report_with_callback<S, V>(
    solver: Arc<S>,
    model: MetaModel<V>,
    callback: SolvingStatusCallback,
) -> Result<SolveReport<V>>
where
    S: SolverExt + Send + Sync + 'static,
    V: SolveValue + Add<Output = V>,
{
    solve_async_report_with_options(
        solver,
        model,
        AsyncSolveOptions::new().with_solving_callback(Some(callback)),
    )
    .await
}

/// 异步求解并返回统一报告（参数对象） / Solve asynchronously and return a unified report with options.
#[cfg(feature = "async")]
pub async fn solve_async_report_with_options<S, V>(
    solver: Arc<S>,
    model: MetaModel<V>,
    options: AsyncSolveOptions,
) -> Result<SolveReport<V>>
where
    S: SolverExt + Send + Sync + 'static,
    V: SolveValue + Add<Output = V>,
{
    spawn_solve_report_with_options(solver, model, options)
        .await
        .map_err(|error| {
            crate::error::CoreError::Internal(format!("async solve-report task failed: {}", error))
        })?
}

fn convert_linear_to_f64<V>(
    polynomial: &Linear<V>,
    policy: SolveValueConversionPolicy,
    field_path: &str,
) -> Result<Linear<f64>>
where
    V: SolveValue,
{
    let mut monomials: Vec<LinearMonomial<f64>> = Vec::with_capacity(polynomial.monomials().len());
    for (index, monomial) in polynomial.monomials().iter().enumerate() {
        let coefficient_context = SolveValueConversionContext::new(format!(
            "{}.monomials[{}].coefficient",
            field_path, index
        ));
        monomials.push(LinearMonomial::new(
            value_to_backend_f64(monomial.coefficient(), policy, &coefficient_context)?,
            monomial.var_index(),
        ));
    }
    let constant_context =
        SolveValueConversionContext::new(format!("{}.constant_term", field_path));
    Ok(Linear::new(
        monomials,
        value_to_backend_f64(polynomial.constant_term(), policy, &constant_context)?,
    ))
}

fn convert_quadratic_to_f64<V>(
    polynomial: &Quadratic<V>,
    policy: SolveValueConversionPolicy,
    field_path: &str,
) -> Result<Quadratic<f64>>
where
    V: SolveValue,
{
    let mut monomials: Vec<QuadraticMonomial<f64>> =
        Vec::with_capacity(polynomial.monomials().len());
    for (index, monomial) in polynomial.monomials().iter().enumerate() {
        let coefficient_context = SolveValueConversionContext::new(format!(
            "{}.monomials[{}].coefficient",
            field_path, index
        ));
        let coefficient =
            value_to_backend_f64(monomial.coefficient(), policy, &coefficient_context)?;
        let converted = if let Some(var_index2) = monomial.var_index2() {
            QuadraticMonomial::new_quadratic(coefficient, monomial.var_index1(), var_index2)
        } else {
            QuadraticMonomial::new_linear(coefficient, monomial.var_index1())
        };
        monomials.push(converted);
    }
    let constant_context = SolveValueConversionContext::new(format!("{}.constant", field_path));
    Ok(Quadratic::new(
        monomials,
        value_to_backend_f64(polynomial.constant(), policy, &constant_context)?,
    ))
}

fn convert_solution_pool_from_f64<V>(
    solutions: Vec<Vec<f64>>,
    policy: SolveValueConversionPolicy,
) -> Result<Vec<Vec<V>>>
where
    V: SolveValue,
{
    solutions
        .into_iter()
        .enumerate()
        .map(|(solution_index, solution)| {
            solution
                .into_iter()
                .enumerate()
                .map(|(value_index, value)| {
                    let context = SolveValueConversionContext::new(format!(
                        "solutions[{}][{}]",
                        solution_index, value_index
                    ));
                    value_from_backend_f64(value, policy, &context)
                })
                .collect::<Result<Vec<V>>>()
        })
        .collect::<Result<Vec<Vec<V>>>>()
}

/// 将泛型机理模型按策略转换为 `f64` 机理模型 / Convert generic mechanism model into `f64` mechanism model with policy
pub fn convert_mechanism_model_to_f64<V>(
    mechanism_model: &MechanismModel<V>,
    policy: SolveValueConversionPolicy,
) -> Result<MechanismModel<f64>>
where
    V: SolveValue,
{
    let mut basic_mechanism_model = BasicMechanismModel::new(&mechanism_model.basic.name);
    for token in mechanism_model.basic.tokens() {
        let variable_data = token.variable.data();
        let converted_data = TokenVariableData {
            id: variable_data.id,
            index: variable_data.index,
            name: variable_data.name.clone(),
            display_name: variable_data.display_name.clone(),
            var_type: variable_data.var_type,
            lower_bound: variable_data
                .lower_bound
                .as_ref()
                .map(|value| {
                    let context = SolveValueConversionContext::new(format!(
                        "tokens[{}].lower_bound",
                        variable_data.index
                    ));
                    value_to_backend_f64(value, policy, &context)
                })
                .transpose()?,
            upper_bound: variable_data
                .upper_bound
                .as_ref()
                .map(|value| {
                    let context = SolveValueConversionContext::new(format!(
                        "tokens[{}].upper_bound",
                        variable_data.index
                    ));
                    value_to_backend_f64(value, policy, &context)
                })
                .transpose()?,
        };
        let converted_token = Token::new(AnyVariable::new(converted_data), token.solver_index);
        if let Some(result) = token.get_result() {
            let context =
                SolveValueConversionContext::new(format!("tokens[{}].result", variable_data.index));
            converted_token.set_result(value_to_backend_f64(&result, policy, &context)?);
        }
        basic_mechanism_model.add_token(converted_token);
    }

    for (index, constraint) in mechanism_model.basic.constraints().iter().enumerate() {
        let inequality = &constraint.inequality;
        let polynomial_field = format!("constraints[{}].polynomial", index);
        let rhs_context = SolveValueConversionContext::new(format!("constraints[{}].rhs", index));
        let converted_inequality = LinearInequality::new(
            convert_linear_to_f64(&inequality.polynomial, policy, &polynomial_field)?,
            inequality.relation,
            value_to_backend_f64(&inequality.rhs, policy, &rhs_context)?,
        );
        let mut converted_constraint =
            LinearConstraint::new(converted_inequality, &constraint.name);
        converted_constraint.group = constraint.group.clone();
        converted_constraint.lazy = constraint.lazy;
        converted_constraint.priority = constraint.priority;
        converted_constraint.args = constraint.args.clone();
        basic_mechanism_model.add_constraint(converted_constraint);
    }

    for (index, constraint) in mechanism_model
        .basic
        .quadratic_constraints()
        .iter()
        .enumerate()
    {
        let inequality = &constraint.inequality;
        let polynomial_field = format!("quadratic_constraints[{}].polynomial", index);
        let rhs_context =
            SolveValueConversionContext::new(format!("quadratic_constraints[{}].rhs", index));
        let converted_inequality = QuadraticInequality::new(
            convert_quadratic_to_f64(&inequality.polynomial, policy, &polynomial_field)?,
            inequality.relation,
            value_to_backend_f64(&inequality.rhs, policy, &rhs_context)?,
        );
        let mut converted_constraint =
            QuadraticConstraint::new(converted_inequality, &constraint.name);
        converted_constraint.group = constraint.group.clone();
        converted_constraint.lazy = constraint.lazy;
        converted_constraint.priority = constraint.priority;
        converted_constraint.args = constraint.args.clone();
        basic_mechanism_model.add_quadratic_constraint(converted_constraint);
    }

    let objective = mechanism_model.objective();
    let mut converted_objective = Objective::new(objective.category);
    for (index, sub_objective) in objective.sub_objectives.iter().enumerate() {
        let polynomial_field = format!("objective.sub_objectives[{}].polynomial", index);
        let weight_context =
            SolveValueConversionContext::new(format!("objective.sub_objectives[{}].weight", index));
        converted_objective.add_sub_objective(SubObjective::new_with_weight(
            sub_objective.category,
            convert_linear_to_f64(&sub_objective.polynomial, policy, &polynomial_field)?,
            &sub_objective.name,
            value_to_backend_f64(&sub_objective.weight, policy, &weight_context)?,
        ));
    }

    let mut converted_model = MechanismModel::from_basic(basic_mechanism_model);
    converted_model.set_objective(converted_objective);
    Ok(converted_model)
}

fn attach_iis_diagnostics(report: &mut SolveReport<f64>, iis: &LinearIISModel) {
    let evidence = iis.to_infeasibility_evidence(report.model_mapping.as_ref());
    if report
        .diagnostics
        .infeasibility_evidence
        .as_ref()
        .is_some_and(InfeasibilityEvidence::is_authoritative)
    {
        report.diagnostics.issues.push(SolveIssue::new(
            "IisEvidencePreserved",
            "authoritative infeasibility evidence was preserved; legacy IIS was not promoted",
        ));
        return;
    }
    report.diagnostics.infeasibility_evidence = Some(evidence);
}

fn attach_iis_failure(report: &mut SolveReport<f64>, error: &crate::error::CoreError) {
    if report.diagnostics.infeasibility_evidence.is_none() {
        report.diagnostics.infeasibility_evidence = Some(InfeasibilityEvidence {
            source: InfeasibilityEvidenceSource::Unavailable,
            reliability: ProofReliability::Unknown,
            completeness: ProofCompleteness::Unavailable,
            constraint_ids: BTreeSet::new(),
            members: BTreeSet::new(),
            minimality: InfeasibilityMinimality::NotChecked,
            computation_time: std::time::Duration::ZERO,
            unavailable_reason: Some(error.to_string()),
        });
    }
    report.diagnostics.issues.push(SolveIssue::new(
        "InfeasibilityDiagnosticsUnavailable",
        format!(
            "IIS computation failed after the solve conclusion: {}",
            error
        ),
    ));
}

/// Nightly: 可调用求解器包装器 / Nightly: callable solver wrapper
#[cfg(feature = "nightly")]
#[derive(Clone, Copy)]
pub struct SolveFn<'a, S>
where
    S: Solver + ?Sized,
{
    solver: &'a S,
    options: SolveOptions<'a>,
}

#[cfg(feature = "nightly")]
impl<'a, S> SolveFn<'a, S>
where
    S: Solver + ?Sized,
{
    /// 创建可调用包装器 / Create callable wrapper
    pub fn new(solver: &'a S) -> Self {
        Self {
            solver,
            options: SolveOptions::default(),
        }
    }

    /// 设置参数 / Set options
    pub fn with_options(mut self, options: SolveOptions<'a>) -> Self {
        self.options = options;
        self
    }
}

#[cfg(feature = "nightly")]
impl<'a, S> FnOnce<(&MetaModel<f64>,)> for SolveFn<'a, S>
where
    S: Solver + ?Sized,
{
    type Output = Result<SolverOutput>;

    extern "rust-call" fn call_once(self, args: (&MetaModel<f64>,)) -> Self::Output {
        self.solver.solve_with_options(args.0, &self.options)
    }
}

#[cfg(feature = "nightly")]
impl<'a, S> FnMut<(&MetaModel<f64>,)> for SolveFn<'a, S>
where
    S: Solver + ?Sized,
{
    extern "rust-call" fn call_mut(&mut self, args: (&MetaModel<f64>,)) -> Self::Output {
        self.solver.solve_with_options(args.0, &self.options)
    }
}

#[cfg(feature = "nightly")]
impl<'a, S> Fn<(&MetaModel<f64>,)> for SolveFn<'a, S>
where
    S: Solver + ?Sized,
{
    extern "rust-call" fn call(&self, args: (&MetaModel<f64>,)) -> Self::Output {
        self.solver.solve_with_options(args.0, &self.options)
    }
}

/// Flt64 多解输出（兼容接口）/ Flt64 multi-solution output (compatibility interface)
#[derive(Debug, Clone)]
pub struct Flt64MultiSolutionOutput {
    /// 主求解输出 / Primary solver output
    pub output: SolverOutput,
    /// 解池 / Solution pool
    pub solutions: Vec<Vec<f64>>,
}

/// 多解输出 / Multi-solution output
#[derive(Debug, Clone)]
pub struct MultiSolutionOutput<V>
where
    V: SolveValue,
{
    /// typed 主可行输出 / Typed primary feasible output
    pub output: FeasibleSolverOutput<V>,
    /// typed 解池 / Typed solution pool
    pub solutions: Vec<Vec<V>>,
}

/// 求解器扩展 trait / Solver extension trait
#[allow(async_fn_in_trait)]
pub trait SolverExt: Solver {
    /// Nightly: 获取可调用包装器 / Nightly: get callable wrapper
    #[cfg(feature = "nightly")]
    fn as_fn<'a>(&'a self) -> SolveFn<'a, Self>
    where
        Self: Sized,
    {
        SolveFn::new(self)
    }

    /// Nightly: 获取带参数的可调用包装器 / Nightly: get callable wrapper with options
    #[cfg(feature = "nightly")]
    fn as_fn_with_options<'a>(&'a self, options: SolveOptions<'a>) -> SolveFn<'a, Self>
    where
        Self: Sized,
    {
        SolveFn::new(self).with_options(options)
    }

    /// 统一 MetaModel 入口 / Unified MetaModel entry
    fn solve<V>(&self, model: &MetaModel<V>) -> Result<SolverOutput>
    where
        V: SolveValue + Add<Output = V>,
    {
        self.solve_with_options(model, &SolveOptions::default())
    }

    /// 统一报告 MetaModel 入口 / Unified-report MetaModel entry.
    ///
    /// 新算法应优先使用该入口；旧 `solve` 保留为兼容 facade / New algorithms should prefer this entry; the legacy `solve` remains a compatibility facade.
    fn solve_report<V>(&self, model: &MetaModel<V>) -> Result<SolveReport<V>>
    where
        V: SolveValue + Add<Output = V>,
    {
        self.solve_report_with_options(model, &SolveOptions::default())
    }

    /// 统一报告 MetaModel 入口（参数对象）/ Unified-report MetaModel entry with options.
    fn solve_report_with_options<V>(
        &self,
        model: &MetaModel<V>,
        options: &SolveOptions<'_>,
    ) -> Result<SolveReport<V>>
    where
        V: SolveValue + Add<Output = V>,
    {
        let mechanism_model = model
            .try_to_mechanism_model_with_status_callback(options.model_building_status_callback)?;
        let mechanism_model =
            convert_mechanism_model_to_f64(&mechanism_model, options.value_conversion_policy)?;
        let report = if mechanism_model.num_quadratic_constraints() > 0 {
            let quadratic_model = mechanism_model
                .clone()
                .try_into_quadratic_tetrad_model_with_status_callback(
                    options.model_building_status_callback,
                )?;
            self.solve_quadratic_report_with_options(&quadratic_model, options)?
        } else {
            let linear_model = mechanism_model
                .clone()
                .try_into_linear_triad_model_with_status_callback(
                    options.model_building_status_callback,
                )?;
            self.solve_linear_report_with_options(&linear_model, options)?
        };
        convert_report_value(report, options.value_conversion_policy)
    }

    /// 统一 MetaModel 入口（参数对象）/ Unified MetaModel entry (options object)
    fn solve_with_options<V>(
        &self,
        model: &MetaModel<V>,
        options: &SolveOptions<'_>,
    ) -> Result<SolverOutput>
    where
        V: SolveValue + Add<Output = V>,
    {
        let mechanism_model = model
            .try_to_mechanism_model_with_status_callback(options.model_building_status_callback)?;
        let mechanism_model =
            convert_mechanism_model_to_f64(&mechanism_model, options.value_conversion_policy)?;
        if mechanism_model.num_quadratic_constraints() > 0 {
            let quadratic_model = mechanism_model
                .try_into_quadratic_tetrad_model_with_status_callback(
                    options.model_building_status_callback,
                )?;
            return self.solve_quadratic_with_options(&quadratic_model, options);
        }

        let linear_model = mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback,
        )?;
        self.solve_linear_with_options(&linear_model, options)
    }

    /// 统一 typed 可行输出入口 / Unified typed feasible output entry
    fn solve_typed<V>(&self, model: &MetaModel<V>) -> Result<FeasibleSolverOutput<V>>
    where
        V: SolveValue + Add<Output = V>,
    {
        self.solve_typed_with_options(model, &SolveOptions::default())
    }

    /// 统一 typed 可行输出入口（参数对象）/ Unified typed feasible output entry (options object)
    fn solve_typed_with_options<V>(
        &self,
        model: &MetaModel<V>,
        options: &SolveOptions<'_>,
    ) -> Result<FeasibleSolverOutput<V>>
    where
        V: SolveValue + Add<Output = V>,
    {
        self.solve_with_options(model, options)?
            .try_into_feasible_typed(options.value_conversion_policy)
    }

    /// 统一 MetaModel 多解入口 / Unified MetaModel multi-solution entry
    fn solve_multi<V>(
        &self,
        model: &MetaModel<V>,
        solution_amount: usize,
    ) -> Result<Flt64MultiSolutionOutput>
    where
        V: SolveValue + Add<Output = V>,
    {
        let options = SolveOptions::new().with_solution_amount(solution_amount);
        self.solve_multi_with_options(model, &options)
    }

    /// 统一 MetaModel 多解入口（参数对象）/ Unified MetaModel multi-solution entry (options object)
    fn solve_multi_with_options<V>(
        &self,
        model: &MetaModel<V>,
        options: &SolveOptions<'_>,
    ) -> Result<Flt64MultiSolutionOutput>
    where
        V: SolveValue + Add<Output = V>,
    {
        let mechanism_model = model
            .try_to_mechanism_model_with_status_callback(options.model_building_status_callback)?;
        let mechanism_model =
            convert_mechanism_model_to_f64(&mechanism_model, options.value_conversion_policy)?;
        if mechanism_model.num_quadratic_constraints() > 0 {
            let quadratic_model = mechanism_model
                .try_into_quadratic_tetrad_model_with_status_callback(
                    options.model_building_status_callback,
                )?;
            return self.solve_quadratic_multi_with_options(&quadratic_model, options);
        }

        let linear_model = mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback,
        )?;
        self.solve_linear_multi_with_options(&linear_model, options)
    }

    /// 统一 typed MetaModel 多解入口 / Unified typed MetaModel multi-solution entry
    fn solve_typed_multi<V>(
        &self,
        model: &MetaModel<V>,
        solution_amount: usize,
    ) -> Result<MultiSolutionOutput<V>>
    where
        V: SolveValue + Add<Output = V>,
    {
        let options = SolveOptions::new().with_solution_amount(solution_amount);
        self.solve_typed_multi_with_options(model, &options)
    }

    /// 统一 typed MetaModel 多解入口（参数对象）/ Unified typed MetaModel multi-solution entry (options object)
    fn solve_typed_multi_with_options<V>(
        &self,
        model: &MetaModel<V>,
        options: &SolveOptions<'_>,
    ) -> Result<MultiSolutionOutput<V>>
    where
        V: SolveValue + Add<Output = V>,
    {
        let multi_output = self.solve_multi_with_options(model, options)?;
        let output = multi_output
            .output
            .try_into_feasible_typed(options.value_conversion_policy)?;
        let solutions = convert_solution_pool_from_f64(
            multi_output.solutions,
            options.value_conversion_policy,
        )?;
        Ok(MultiSolutionOutput { output, solutions })
    }

    /// 统一 MetaModel 求解并附加 IIS diagnostics / Solve a MetaModel and attach IIS diagnostics to the unified report.
    fn solve_report_with_iis<V>(
        &self,
        model: &MetaModel<V>,
        iis_config: &IISConfig,
    ) -> Result<SolveReportWithIIS<V>>
    where
        V: SolveValue + Add<Output = V>,
    {
        self.solve_report_with_options_and_iis(model, &SolveOptions::default(), iis_config)
    }

    /// 统一 MetaModel 求解并附加 IIS diagnostics（参数对象） / Solve a MetaModel with options and attach IIS diagnostics.
    fn solve_report_with_options_and_iis<V>(
        &self,
        model: &MetaModel<V>,
        options: &SolveOptions<'_>,
        iis_config: &IISConfig,
    ) -> Result<SolveReportWithIIS<V>>
    where
        V: SolveValue + Add<Output = V>,
    {
        let mechanism_model = model
            .try_to_mechanism_model_with_status_callback(options.model_building_status_callback)?;
        let mechanism_model =
            convert_mechanism_model_to_f64(&mechanism_model, options.value_conversion_policy)?;
        let mut report = if mechanism_model.num_quadratic_constraints() > 0 {
            let quadratic_model = mechanism_model
                .clone()
                .try_into_quadratic_tetrad_model_with_status_callback(
                    options.model_building_status_callback,
                )?;
            self.solve_quadratic_report_with_options(&quadratic_model, options)?
        } else {
            let linear_model = mechanism_model
                .clone()
                .try_into_linear_triad_model_with_status_callback(
                    options.model_building_status_callback,
                )?;
            self.solve_linear_report_with_options(&linear_model, options)?
        };

        let iis = if report.problem_status == super::ProblemStatus::Infeasible {
            let result = if mechanism_model.num_quadratic_constraints() > 0 {
                let quadratic_model =
                    mechanism_model.try_into_quadratic_tetrad_model_with_status_callback(None)?;
                compute_iis(&quadratic_model.basic.linear, iis_config)
            } else {
                let linear_model =
                    mechanism_model.try_into_linear_triad_model_with_status_callback(None)?;
                compute_iis(&linear_model, iis_config)
            };
            match result {
                Ok(iis) => {
                    attach_iis_diagnostics(&mut report, &iis);
                    Some(iis)
                }
                Err(error) => {
                    attach_iis_failure(&mut report, &error);
                    None
                }
            }
        } else {
            None
        };
        report.validate()?;
        Ok(SolveReportWithIIS {
            report: convert_report_value(report, options.value_conversion_policy)?,
            iis,
        })
    }

    /// 统一 MetaModel 求解 + IIS fallback / Unified MetaModel solve with IIS fallback
    fn solve_with_iis<V>(
        &self,
        model: &MetaModel<V>,
        iis_config: &IISConfig,
    ) -> Result<SolverOutputWithIIS>
    where
        V: SolveValue + Add<Output = V>,
    {
        self.solve_with_options_and_iis(model, &SolveOptions::default(), iis_config)
    }

    /// 统一 MetaModel 求解 + IIS fallback（参数对象）/ Unified MetaModel solve with IIS fallback (options object)
    fn solve_with_options_and_iis<V>(
        &self,
        model: &MetaModel<V>,
        options: &SolveOptions<'_>,
        iis_config: &IISConfig,
    ) -> Result<SolverOutputWithIIS>
    where
        V: SolveValue + Add<Output = V>,
    {
        let mechanism_model = model
            .try_to_mechanism_model_with_status_callback(options.model_building_status_callback)?;
        let mechanism_model =
            convert_mechanism_model_to_f64(&mechanism_model, options.value_conversion_policy)?;
        if mechanism_model.num_quadratic_constraints() > 0 {
            let quadratic_model = mechanism_model
                .try_into_quadratic_tetrad_model_with_status_callback(
                    options.model_building_status_callback,
                )?;
            let mut report = attach_quadratic_model_mapping(
                self.solve_quadratic_report_with_options(&quadratic_model, options)?,
                &quadratic_model,
            )?;
            let iis = if report.problem_status == super::ProblemStatus::Infeasible {
                match compute_iis(&quadratic_model.basic.linear, iis_config) {
                    Ok(iis) => {
                        attach_iis_diagnostics(&mut report, &iis);
                        Some(iis)
                    }
                    Err(error) => {
                        attach_iis_failure(&mut report, &error);
                        None
                    }
                }
            } else {
                None
            };
            report.validate()?;
            let output = solve_report_to_solver_output(&report);
            return Ok(SolverOutputWithIIS {
                output,
                iis,
                report: Some(report),
            });
        }

        let linear_model = mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback,
        )?;
        let mut report = attach_linear_model_mapping(
            self.solve_linear_report_with_options(&linear_model, options)?,
            &linear_model,
        )?;
        let iis = if report.problem_status == super::ProblemStatus::Infeasible {
            match compute_iis(linear_model.as_basic(), iis_config) {
                Ok(iis) => {
                    attach_iis_diagnostics(&mut report, &iis);
                    Some(iis)
                }
                Err(error) => {
                    attach_iis_failure(&mut report, &error);
                    None
                }
            }
        } else {
            None
        };
        report.validate()?;
        let output = solve_report_to_solver_output(&report);
        Ok(SolverOutputWithIIS {
            output,
            iis,
            report: Some(report),
        })
    }

    /// 线性模型 + IIS fallback / Linear solve with IIS fallback
    fn solve_linear_with_iis(
        &self,
        model: &LinearTriadModel,
        iis_config: &IISConfig,
    ) -> Result<SolverOutputWithIIS> {
        let output = self.solve_linear(model)?;
        let mut report =
            attach_linear_model_mapping(solver_output_to_report(output.clone())?, model)?;
        let iis = if report.problem_status == super::ProblemStatus::Infeasible {
            match compute_iis(model.as_basic(), iis_config) {
                Ok(iis) => {
                    attach_iis_diagnostics(&mut report, &iis);
                    Some(iis)
                }
                Err(error) => {
                    attach_iis_failure(&mut report, &error);
                    None
                }
            }
        } else {
            None
        };
        report.validate()?;
        Ok(SolverOutputWithIIS {
            output,
            iis,
            report: Some(report),
        })
    }

    /// 二次模型 + IIS fallback（针对线性约束部分）/ Quadratic solve with IIS fallback (linear constraints only)
    fn solve_quadratic_with_iis(
        &self,
        model: &QuadraticTetradModel,
        iis_config: &IISConfig,
    ) -> Result<SolverOutputWithIIS> {
        let output = self.solve_quadratic(model)?;
        let mut report =
            attach_quadratic_model_mapping(solver_output_to_report(output.clone())?, model)?;
        let iis = if report.problem_status == super::ProblemStatus::Infeasible {
            match compute_iis(&model.basic.linear, iis_config) {
                Ok(iis) => {
                    attach_iis_diagnostics(&mut report, &iis);
                    Some(iis)
                }
                Err(error) => {
                    attach_iis_failure(&mut report, &error);
                    None
                }
            }
        } else {
            None
        };
        report.validate()?;
        Ok(SolverOutputWithIIS {
            output,
            iis,
            report: Some(report),
        })
    }

    /// 线性模型多解接口（默认返回主解）/ Multi-solution API for linear model (returns primary solution by default)
    fn solve_linear_multi_with_options(
        &self,
        model: &LinearTriadModel,
        options: &SolveOptions<'_>,
    ) -> Result<Flt64MultiSolutionOutput> {
        if options.solution_amount > 1
            && let Some((output, solutions)) = self.solve_linear_with_solution_pool_with_options(
                model,
                options.solution_amount,
                options,
            )?
        {
            return Ok(Flt64MultiSolutionOutput { output, solutions });
        }

        let output = self.solve_linear(model)?;
        let mut solutions = Vec::new();
        if options.solution_amount > 0
            && let Some(solution) = output.solution.clone()
        {
            solutions.push(solution);
        }
        Ok(Flt64MultiSolutionOutput { output, solutions })
    }

    /// 二次模型多解接口（默认返回主解）/ Multi-solution API for quadratic model (returns primary solution by default)
    fn solve_quadratic_multi_with_options(
        &self,
        model: &QuadraticTetradModel,
        options: &SolveOptions<'_>,
    ) -> Result<Flt64MultiSolutionOutput> {
        if options.solution_amount > 1
            && let Some((output, solutions)) = self
                .solve_quadratic_with_solution_pool_with_options(
                    model,
                    options.solution_amount,
                    options,
                )?
        {
            return Ok(Flt64MultiSolutionOutput { output, solutions });
        }

        let output = self.solve_quadratic(model)?;
        let mut solutions = Vec::new();
        if options.solution_amount > 0
            && let Some(solution) = output.solution.clone()
        {
            solutions.push(solution);
        }
        Ok(Flt64MultiSolutionOutput { output, solutions })
    }

    /// 统一异步入口（线性）/ Unified async entry (linear)
    #[cfg(feature = "async")]
    async fn solve_linear_async(&self, model: &LinearTriadModel) -> Result<SolverOutput>
    where
        Self: Sync,
    {
        self.solve_linear(model)
    }

    /// 统一异步入口（二次）/ Unified async entry (quadratic)
    #[cfg(feature = "async")]
    async fn solve_quadratic_async(&self, model: &QuadraticTetradModel) -> Result<SolverOutput>
    where
        Self: Sync,
    {
        self.solve_quadratic(model)
    }

    /// 统一异步入口（MetaModel）/ Unified async entry (MetaModel)
    #[cfg(feature = "async")]
    async fn solve_async<V>(&self, model: &MetaModel<V>) -> Result<SolverOutput>
    where
        Self: Sync,
        V: SolveValue + Add<Output = V>,
    {
        self.solve(model)
    }
}

impl<T> SolverExt for T where T: Solver + ?Sized {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    #[cfg(feature = "async")]
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    #[cfg(feature = "async")]
    use std::thread;

    use super::*;
    use crate::error::{CoreError, SolverError};
    use crate::model::intermediate::{BasicLinearTriadModel, BasicQuadraticTetradModel};
    use crate::model::{
        ConstraintRelation, MetaModel, ModelBuildingStage, ModelBuildingStatusCallback,
    };
    use crate::solver::{
        LinearSolver, QuadraticSolver, SolverCapability, SolverInfo, SolverStatus,
    };
    use crate::symbol::flatten::{Linear, LinearMonomial};
    use crate::variable::ContinuousVariableItem;
    use bigdecimal::BigDecimal;
    use num_rational::BigRational;

    #[derive(Debug)]
    struct DummySolver;

    impl SolverInfo for DummySolver {
        fn name(&self) -> &str {
            "dummy"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for DummySolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![2.0, 3.0]))
        }
    }

    impl QuadraticSolver for DummySolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::new(SolverStatus::Optimal).with_solution(vec![4.0]))
        }
    }

    #[cfg(feature = "async")]
    struct CancellationAwareBlockingSolver {
        started: Arc<AtomicBool>,
        finished: Arc<AtomicBool>,
    }

    #[cfg(feature = "async")]
    impl SolverInfo for CancellationAwareBlockingSolver {
        fn name(&self) -> &str {
            "cancellation-aware-blocking"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear]
        }
    }

    #[cfg(feature = "async")]
    impl LinearSolver for CancellationAwareBlockingSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(0.0, vec![0.0]))
        }

        fn solve_linear_with_options(
            &self,
            _model: &LinearTriadModel,
            options: &SolveOptions<'_>,
        ) -> Result<SolverOutput> {
            let handle = options.cancellation_handle.ok_or_else(|| {
                CoreError::contract_error(
                    "blocking cancellation probe did not receive a SolveHandle".to_owned(),
                )
            })?;
            self.started.store(true, Ordering::SeqCst);
            while !handle.is_cancelled() {
                thread::yield_now();
            }
            self.finished.store(true, Ordering::SeqCst);
            Ok(SolverOutput::new(SolverStatus::UserInterrupt))
        }
    }

    #[cfg(feature = "async")]
    impl QuadraticSolver for CancellationAwareBlockingSolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(0.0, vec![0.0]))
        }
    }

    #[cfg(feature = "async")]
    #[derive(Debug)]
    struct ErrorReturningSolver;

    #[cfg(feature = "async")]
    impl SolverInfo for ErrorReturningSolver {
        fn name(&self) -> &str {
            "error-returning-solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear]
        }
    }

    #[cfg(feature = "async")]
    impl LinearSolver for ErrorReturningSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Err(CoreError::solver_backend("synthetic backend failure"))
        }
    }

    #[cfg(feature = "async")]
    impl QuadraticSolver for ErrorReturningSolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Err(CoreError::solver_backend("synthetic backend failure"))
        }
    }

    #[derive(Debug)]
    struct NativePoolSolver;

    impl SolverInfo for NativePoolSolver {
        fn name(&self) -> &str {
            "native_pool_solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for NativePoolSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![10.0]))
        }

        fn solve_linear_with_solution_pool(
            &self,
            _model: &LinearTriadModel,
            _solution_amount: usize,
        ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
            Ok(Some((
                SolverOutput::optimal(1.0, vec![10.0]),
                vec![vec![10.0], vec![11.0], vec![12.0]],
            )))
        }
    }

    impl QuadraticSolver for NativePoolSolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(2.0, vec![20.0]))
        }
    }

    #[test]
    fn multi_solution_output_returns_primary_solution_for_linear() {
        let solver = DummySolver;
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("dummy_linear"));
        let output = solver
            .solve_linear_multi_with_options(&model, &SolveOptions::new().with_solution_amount(5))
            .expect("multi solution should succeed");
        assert_eq!(output.solutions.len(), 1);
        assert_eq!(output.solutions[0], vec![2.0, 3.0]);
    }

    #[test]
    fn multi_solution_output_returns_primary_solution_for_quadratic() {
        let solver = DummySolver;
        let model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("dummy_q"));
        let output = solver
            .solve_quadratic_multi_with_options(
                &model,
                &SolveOptions::new().with_solution_amount(3),
            )
            .expect("quadratic multi solution should succeed");
        assert_eq!(output.solutions.len(), 1);
        assert_eq!(output.solutions[0], vec![4.0]);
    }

    #[test]
    fn multi_solution_prefers_native_solution_pool_when_available() {
        let solver = NativePoolSolver;
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("native_pool_linear"));
        let output = solver
            .solve_linear_multi_with_options(&model, &SolveOptions::new().with_solution_amount(3))
            .expect("native pool multi solution should succeed");
        assert_eq!(output.solutions.len(), 3);
        assert_eq!(output.solutions[0], vec![10.0]);
        assert_eq!(output.solutions[1], vec![11.0]);
        assert_eq!(output.solutions[2], vec![12.0]);
    }

    #[test]
    fn report_multi_solution_callback_is_not_emitted_twice() {
        let solver = NativePoolSolver;
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("native_pool_report"));
        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: SolvingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.status);
            Ok(())
        });
        let options = SolveOptions::new()
            .with_solution_amount(3)
            .with_solving_callback(Some(&callback));

        let report = solver
            .solve_linear_report_with_options(&model, &options)
            .expect("native pool report should succeed");

        assert_eq!(
            statuses.lock().unwrap().as_slice(),
            &[SolverStatus::Solving, SolverStatus::Optimal]
        );
        assert_eq!(
            report
                .solution
                .as_ref()
                .map(|solution| solution.pool.clone()),
            Some(vec![vec![10.0], vec![11.0], vec![12.0]])
        );
    }

    #[test]
    fn solve_with_options_supports_meta_model_entry_with_callbacks() {
        let mut model = MetaModel::<f64>::new("dummy_meta");
        let x = ContinuousVariableItem::auto("dummy_meta_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_c",
            )
            .unwrap();

        let model_stages = Arc::new(Mutex::new(Vec::new()));
        let model_stages_for_callback = model_stages.clone();
        let model_callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            model_stages_for_callback.lock().unwrap().push(status.stage);
            Ok(())
        });

        let solving_statuses = Arc::new(Mutex::new(Vec::new()));
        let solving_statuses_for_callback = solving_statuses.clone();
        let solving_callback: SolvingStatusCallback = Arc::new(move |status| {
            solving_statuses_for_callback
                .lock()
                .unwrap()
                .push(status.status);
            Ok(())
        });

        let solver = DummySolver;
        let options = SolveOptions::new()
            .with_building_callback(Some(&model_callback))
            .with_solving_callback(Some(&solving_callback));
        let output = solver
            .solve_with_options(&model, &options)
            .expect("meta model solve with callbacks should succeed");
        assert!(output.status.is_optimal());

        let model_stages = model_stages.lock().unwrap();
        assert!(model_stages.contains(&ModelBuildingStage::RegisterTokens));
        assert!(model_stages.contains(&ModelBuildingStage::FlattenLinearModel));

        let solving_statuses = solving_statuses.lock().unwrap();
        assert_eq!(solving_statuses.len(), 2);
        assert_eq!(solving_statuses[0], SolverStatus::Solving);
        assert_eq!(solving_statuses[1], SolverStatus::Optimal);
    }

    #[test]
    fn solve_with_options_supports_meta_model_entry() {
        let mut model = MetaModel::<f64>::new("dummy_meta_options");
        let x = ContinuousVariableItem::auto("dummy_meta_options_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_options_c",
            )
            .unwrap();

        let solver = DummySolver;
        let options = SolveOptions::new();
        let output = solver
            .solve_with_options(&model, &options)
            .expect("meta model solve with options should succeed");
        assert!(output.status.is_optimal());
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn spawn_solve_runs_meta_model_on_background_pool() {
        let mut model = MetaModel::<f64>::new("dummy_meta_spawn_async");
        let x = ContinuousVariableItem::auto("dummy_meta_spawn_async_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_spawn_async_c",
            )
            .unwrap();

        let output = spawn_solve(Arc::new(DummySolver), model)
            .await
            .expect("async solve task should join")
            .expect("async solve should succeed");
        assert!(output.status.is_optimal());
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn spawn_solve_marks_backend_error_completed_before_task_cleanup() {
        let mut model = MetaModel::<f64>::new("blocking_error_completion");
        let x = ContinuousVariableItem::auto("blocking_error_completion_x");
        model.register_variable(x).unwrap();
        let external_handle = SolveHandle::new();
        let task = spawn_solve_with_options(
            Arc::new(ErrorReturningSolver),
            model,
            AsyncSolveOptions::new().with_cancellation_handle(Some(external_handle.clone())),
        );

        let joined = task.await.expect("backend error task should join");
        assert!(joined.is_err());
        assert!(!external_handle.is_cancelled());
        assert!(external_handle.completed_at_epoch_ms().is_some());
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn abort_requests_backend_cancellation_before_joining_blocking_task() {
        let mut model = MetaModel::<f64>::new("blocking_cancel_abort");
        let x = ContinuousVariableItem::auto("blocking_cancel_abort_x");
        model.register_variable(x).unwrap();
        let started = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let external_handle = SolveHandle::new();
        let task = spawn_solve_with_options(
            Arc::new(CancellationAwareBlockingSolver {
                started: Arc::clone(&started),
                finished: Arc::clone(&finished),
            }),
            model,
            AsyncSolveOptions::new().with_cancellation_handle(Some(external_handle.clone())),
        );

        for _ in 0..10_000 {
            if started.load(Ordering::SeqCst) {
                break;
            }
            thread::yield_now();
        }
        assert!(started.load(Ordering::SeqCst));
        task.abort();
        let joined = task.await.expect("blocking task should join after abort");
        assert!(joined.is_ok());
        assert!(finished.load(Ordering::SeqCst));
        assert_eq!(
            external_handle
                .cancellation()
                .expect("abort should record cancellation")
                .origin,
            CancellationOrigin::TokioTaskAbort
        );
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn dropping_solve_task_handle_requests_backend_cleanup() {
        let mut model = MetaModel::<f64>::new("blocking_cancel_drop");
        let x = ContinuousVariableItem::auto("blocking_cancel_drop_x");
        model.register_variable(x).unwrap();
        let started = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let external_handle = SolveHandle::new();
        let task = spawn_solve_with_options(
            Arc::new(CancellationAwareBlockingSolver {
                started: Arc::clone(&started),
                finished: Arc::clone(&finished),
            }),
            model,
            AsyncSolveOptions::new().with_cancellation_handle(Some(external_handle.clone())),
        );

        for _ in 0..10_000 {
            if started.load(Ordering::SeqCst) {
                break;
            }
            thread::yield_now();
        }
        assert!(started.load(Ordering::SeqCst));
        drop(task);
        for _ in 0..10_000 {
            if finished.load(Ordering::SeqCst) {
                break;
            }
            thread::yield_now();
        }
        assert!(finished.load(Ordering::SeqCst));
        assert_eq!(
            external_handle
                .cancellation()
                .expect("dropping task should record cancellation")
                .origin,
            CancellationOrigin::TokioTaskAbort
        );
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn solve_async_with_callback_forwards_solving_callback() {
        let mut model = MetaModel::<f64>::new("dummy_meta_async_callback");
        let x = ContinuousVariableItem::auto("dummy_meta_async_callback_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_async_callback_c",
            )
            .unwrap();

        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: SolvingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.status);
            Ok(())
        });

        let output = solve_async_with_callback(Arc::new(DummySolver), model, callback)
            .await
            .expect("async solve with callback should succeed");
        assert!(output.status.is_optimal());

        let statuses = statuses.lock().unwrap();
        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0], SolverStatus::Solving);
        assert_eq!(statuses[1], SolverStatus::Optimal);
    }

    #[test]
    fn solve_multi_supports_meta_model_entry() {
        let mut model = MetaModel::<f64>::new("dummy_meta_multi");
        let x = ContinuousVariableItem::auto("dummy_meta_multi_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_multi_c",
            )
            .unwrap();

        let solver = NativePoolSolver;
        let output = solver
            .solve_multi(&model, 3)
            .expect("meta model multi solve should succeed");
        assert_eq!(output.solutions.len(), 3);
        assert_eq!(output.solutions[0], vec![10.0]);
        assert_eq!(output.solutions[1], vec![11.0]);
        assert_eq!(output.solutions[2], vec![12.0]);
    }

    #[test]
    fn solve_typed_multi_supports_meta_model_entry() {
        let mut model = MetaModel::<BigDecimal>::new("dummy_meta_typed_multi");
        let x = ContinuousVariableItem::auto("dummy_meta_typed_multi_x");
        let x_index = model.register_variable(x).unwrap();
        let polynomial = Linear::new(
            vec![LinearMonomial::new(
                BigDecimal::from_str("1.0").expect("create decimal coefficient"),
                x_index,
            )],
            BigDecimal::from_str("0.0").expect("create decimal constant"),
        );
        model
            .add_linear_polynomial_constraint(
                polynomial,
                ConstraintRelation::LessEqual,
                BigDecimal::from_str("2.0").expect("create decimal rhs"),
                "dummy_meta_typed_multi_c",
            )
            .unwrap();

        let solver = NativePoolSolver;
        let options = SolveOptions::new()
            .with_solution_amount(3)
            .with_value_conversion_policy(SolveValueConversionPolicy::AllowRounding);
        let output = solver
            .solve_typed_multi_with_options(&model, &options)
            .expect("typed multi solve should succeed");
        assert_eq!(output.solutions.len(), 3);
        assert_eq!(output.solutions[0].len(), 1);
        assert_eq!(output.output.solution.len(), 1);
        assert!(output.output.status.is_feasible());
    }

    #[test]
    fn solve_with_iis_supports_feasible_meta_model_entry() {
        let mut model = MetaModel::<f64>::new("dummy_meta_iis");
        let x = ContinuousVariableItem::auto("dummy_meta_iis_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_iis_c",
            )
            .unwrap();

        let solver = DummySolver;
        let output = solver
            .solve_with_iis(&model, &IISConfig::default())
            .expect("meta model solve with IIS should succeed");
        assert!(output.output.status.is_optimal());
        assert!(output.iis.is_none());
    }

    #[test]
    fn solve_with_options_supports_big_rational_meta_model_entry() {
        let mut model = MetaModel::<BigRational>::new("dummy_meta_big_rational");
        let x = ContinuousVariableItem::auto("dummy_meta_big_rational_x");
        let x_index = model.register_variable(x).unwrap();
        let polynomial = Linear::new(
            vec![LinearMonomial::new(
                BigRational::from_integer(1.into()),
                x_index,
            )],
            BigRational::from_integer(0.into()),
        );
        model
            .add_linear_polynomial_constraint(
                polynomial,
                ConstraintRelation::LessEqual,
                BigRational::from_integer(2.into()),
                "dummy_meta_big_rational_c",
            )
            .unwrap();

        let solver = DummySolver;
        let options =
            SolveOptions::new().with_value_conversion_policy(SolveValueConversionPolicy::Strict);
        let output = solver
            .solve_with_options(&model, &options)
            .expect("big rational meta model solve with options should succeed");
        assert!(output.status.is_optimal());
    }

    #[test]
    fn solve_with_options_supports_big_decimal_meta_model_entry() {
        let mut model = MetaModel::<BigDecimal>::new("dummy_meta_big_decimal");
        let x = ContinuousVariableItem::auto("dummy_meta_big_decimal_x");
        let x_index = model.register_variable(x).unwrap();
        let polynomial = Linear::new(
            vec![LinearMonomial::new(
                BigDecimal::from_str("1.0").expect("create decimal coefficient"),
                x_index,
            )],
            BigDecimal::from_str("0.0").expect("create decimal constant"),
        );
        model
            .add_linear_polynomial_constraint(
                polynomial,
                ConstraintRelation::LessEqual,
                BigDecimal::from_str("2.0").expect("create decimal rhs"),
                "dummy_meta_big_decimal_c",
            )
            .unwrap();

        let solver = DummySolver;
        let options =
            SolveOptions::new().with_value_conversion_policy(SolveValueConversionPolicy::Strict);
        let output = solver
            .solve_with_options(&model, &options)
            .expect("big decimal meta model solve with options should succeed");
        assert!(output.status.is_optimal());
    }

    #[test]
    fn solve_with_options_rejects_big_rational_precision_loss_in_strict_mode() {
        let mut model =
            MetaModel::<BigRational>::new("dummy_meta_big_rational_precision_loss_strict");
        let x = ContinuousVariableItem::auto("dummy_meta_big_rational_precision_loss_strict_x");
        let x_index = model.register_variable(x).unwrap();
        let polynomial = Linear::new(
            vec![LinearMonomial::new(
                BigRational::new(1.into(), 3.into()),
                x_index,
            )],
            BigRational::from_integer(0.into()),
        );
        model
            .add_linear_polynomial_constraint(
                polynomial,
                ConstraintRelation::LessEqual,
                BigRational::from_integer(2.into()),
                "dummy_meta_big_rational_precision_loss_strict_c",
            )
            .unwrap();

        let solver = DummySolver;
        let options =
            SolveOptions::new().with_value_conversion_policy(SolveValueConversionPolicy::Strict);
        let error = solver
            .solve_with_options(&model, &options)
            .expect_err("strict mode should reject precision-loss big rational conversion");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::PrecisionLoss(_))
        ));
    }

    #[test]
    fn solve_with_options_rejects_big_decimal_precision_loss_in_strict_mode() {
        let mut model =
            MetaModel::<BigDecimal>::new("dummy_meta_big_decimal_precision_loss_strict");
        let x = ContinuousVariableItem::auto("dummy_meta_big_decimal_precision_loss_strict_x");
        let x_index = model.register_variable(x).unwrap();
        let polynomial = Linear::new(
            vec![LinearMonomial::new(
                BigDecimal::from_str("0.12345678901234567890123456789")
                    .expect("create high precision decimal coefficient"),
                x_index,
            )],
            BigDecimal::from_str("0.0").expect("create decimal constant"),
        );
        model
            .add_linear_polynomial_constraint(
                polynomial,
                ConstraintRelation::LessEqual,
                BigDecimal::from_str("2.0").expect("create decimal rhs"),
                "dummy_meta_big_decimal_precision_loss_strict_c",
            )
            .unwrap();

        let solver = DummySolver;
        let options =
            SolveOptions::new().with_value_conversion_policy(SolveValueConversionPolicy::Strict);
        let error = solver
            .solve_with_options(&model, &options)
            .expect_err("strict mode should reject precision-loss big decimal conversion");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::PrecisionLoss(_))
        ));
    }

    #[test]
    fn solve_typed_with_options_supports_big_decimal_meta_model_entry() {
        let mut model = MetaModel::<BigDecimal>::new("dummy_meta_big_decimal_typed");
        let x = ContinuousVariableItem::auto("dummy_meta_big_decimal_typed_x");
        let x_index = model.register_variable(x).unwrap();
        let polynomial = Linear::new(
            vec![LinearMonomial::new(
                BigDecimal::from_str("1.0").expect("create decimal coefficient"),
                x_index,
            )],
            BigDecimal::from_str("0.0").expect("create decimal constant"),
        );
        model
            .add_linear_polynomial_constraint(
                polynomial,
                ConstraintRelation::LessEqual,
                BigDecimal::from_str("2.0").expect("create decimal rhs"),
                "dummy_meta_big_decimal_typed_c",
            )
            .unwrap();

        let solver = DummySolver;
        let options = SolveOptions::new()
            .with_value_conversion_policy(SolveValueConversionPolicy::AllowRounding);
        let output = solver
            .solve_typed_with_options(&model, &options)
            .expect("typed solve should succeed");
        assert!(output.status.is_feasible());
        assert!(output.objective_value.is_some());
        assert_eq!(output.solution.len(), 2);
    }

    #[cfg(feature = "nightly")]
    #[test]
    fn solve_fn_shortcut_supports_meta_model_call() {
        let mut model = MetaModel::<f64>::new("dummy_meta_fn");
        let x = ContinuousVariableItem::auto("dummy_meta_fn_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_fn_c",
            )
            .unwrap();

        let solver = DummySolver;
        let callable = solver.as_fn();
        let output = callable(&model).expect("nightly callable solver should succeed");
        assert!(output.status.is_optimal());
    }

    #[cfg(feature = "nightly")]
    #[test]
    fn solve_fn_with_options_forwards_solving_callback() {
        let mut model = MetaModel::<f64>::new("dummy_meta_fn_options");
        let x = ContinuousVariableItem::auto("dummy_meta_fn_options_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_fn_options_c",
            )
            .unwrap();

        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: SolvingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.status);
            Ok(())
        });

        let solver = DummySolver;
        let options = SolveOptions::new().with_solving_callback(Some(&callback));
        let callable = solver.as_fn_with_options(options);
        let output = callable(&model).expect("nightly callable solver with options should succeed");
        assert!(output.status.is_optimal());

        let statuses = statuses.lock().unwrap();
        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0], SolverStatus::Solving);
        assert_eq!(statuses[1], SolverStatus::Optimal);
    }
}
