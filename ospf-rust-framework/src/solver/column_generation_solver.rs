//! 列生成求解器定义
//! Column Generation Solver Definitions
//!
//! 本模块提供列生成求解器的 trait 定义。
//! This module provides trait definitions for column generation solvers.

use ospf_rust_core::error::{CoreError, Result, SolverError};
#[cfg(all(feature = "nightly", not(feature = "async")))]
use ospf_rust_core::model::MetaModel;
use super::FrameworkSolveOptions;
use ospf_rust_core::solver::{

    FeasibleSolverOutput, SolveValue, SolveValueConversionPolicy, SolverOutput,
};
use std::sync::Arc;
use std::time::Duration;

/// 求解状态回调 / Solving Status Callback
pub type SolvingStatusCallback = Arc<dyn Fn(&SolvingStatus) -> Result<()> + Send + Sync>;

/// 注册状态回调 / Registration Status Callback
pub type RegistrationStatusCallback = Arc<dyn Fn(&RegistrationStatus) -> Result<()> + Send + Sync>;

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
#[derive(Debug, Clone)]
pub struct BendersIterationSnapshot {
    /// 当前轮次（从 1 开始）/ Iteration index (1-based)
    pub iteration: usize,
    /// 当前 master 目标值 / Current master objective value
    pub master_obj: f64,
    /// 当前 master gap（若可用）/ Current master gap (if available)
    pub master_gap: Option<f64>,
    /// 本轮新增 cut 数 / Newly added cuts in this iteration
    pub cuts_added: usize,
    /// 截止本轮累计 cut 数 / Total cuts up to this iteration
    pub total_cuts: usize,
    /// 本轮结束时连续无新 cut 轮数 / Consecutive no-cut iterations at end of this iteration
    pub no_cut_iterations: usize,
    /// 本轮结束时连续目标停滞轮数 / Consecutive objective-stall iterations at end of this iteration
    pub no_obj_improvement_iterations: usize,
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
        if !output.status.is_feasible() {
            return Err(CoreError::Solver(SolverError::SolveFailed(format!(
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
        let typed_output = Self::try_feasible_typed_output_from_output(output, policy).map_err(
            |err| match err {
                CoreError::Solver(SolverError::NoSolution) => {
                    CoreError::Solver(SolverError::SolveFailed(
                        "solver returned feasible status but missing solution vector".into(),
                    ))
                }
                other => other,
            },
        )?;
        let obj = typed_output.objective_value.ok_or_else(|| {
            CoreError::Solver(SolverError::SolveFailed(
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
        })
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
}

impl<V> FeasibleSolutionV<V>
where
    V: SolveValue,
{
    /// 从 core typed 输出创建 / Create from core typed output
    pub fn try_from_typed_output(output: FeasibleSolverOutput<V>) -> Result<Self> {
        let obj = output.objective_value.ok_or_else(|| {
            CoreError::Solver(SolverError::SolveFailed(
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
}

impl LPResult {
    /// 创建新的 LP 结果 / Create new LP result
    pub fn new(result: FeasibleSolution, dual_solution: LinearDualSolution) -> Self {
        Self {
            result,
            dual_solution,
        }
    }

    /// 转换为泛型 LP 结果 / Convert into generic LP result
    pub fn try_into_typed<V>(self, policy: SolveValueConversionPolicy) -> Result<LPResultV<V>>
    where
        V: SolveValue,
    {
        Ok(LPResultV {
            result: self.result.try_into_typed(policy)?,
            dual_solution: self.dual_solution.try_into_typed(policy)?,
        })
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

    /// 统一入口：求解 MetaModel（默认 MILP 路径）/ Unified entry: solve MetaModel (MILP by default)
    #[cfg(feature = "async")]
    fn solve<'a, V>(
        &'a self,
        meta_model: &'a ospf_rust_core::model::MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue,
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
        V: ospf_rust_core::solver::SolveValue,
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
        V: SolveValue,
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
        V: SolveValue,
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

    /// 统一入口：求解 MetaModel（默认 MILP 路径）/ Unified entry: solve MetaModel (MILP by default)
    #[cfg(not(feature = "async"))]
    fn solve<V>(&self, meta_model: &ospf_rust_core::model::MetaModel<V>) -> Result<FeasibleSolution>
    where
        Self: Sized,
        V: ospf_rust_core::solver::SolveValue,
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
        V: ospf_rust_core::solver::SolveValue,
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
        V: SolveValue,
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
        V: SolveValue,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveCategory {
    /// 最小化 / Minimum
    Minimum,
    /// 最大化 / Maximum
    Maximum,
}

impl Default for ObjectiveCategory {
    fn default() -> Self {
        Self::Minimum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::error::{CoreError, SolverError};
    use ospf_rust_core::model::MetaModel;
    use ospf_rust_core::model::intermediate::LinearTriadModel;
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::{
        ConstraintRelation, MechanismModel, ModelBuildingStage, ModelBuildingStatusCallback,
    };
    use ospf_rust_core::solver::{SolverOutput, SolverStatus};
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
        assert!(
            stages
                .iter()
                .any(|stage| *stage == ModelBuildingStage::FlattenLinearModel)
        );
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
        assert!(
            stages
                .iter()
                .any(|stage| *stage == ModelBuildingStage::RegisterTokens)
        );
        assert!(
            stages
                .iter()
                .any(|stage| *stage == ModelBuildingStage::FlattenLinearModel)
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
        assert!(
            stages
                .iter()
                .any(|stage| *stage == ModelBuildingStage::RegisterTokens)
        );
        assert!(
            stages
                .iter()
                .any(|stage| *stage == ModelBuildingStage::FlattenLinearModel)
        );
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
