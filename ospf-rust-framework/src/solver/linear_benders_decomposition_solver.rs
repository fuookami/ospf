//! Benders 分解求解器
//! Benders Decomposition Solver
//!
//! 本模块提供 Benders 分解算法的求解器接口。
//! This module provides solver interfaces for Benders decomposition algorithms.

use super::{
    BendersIterationSnapshot, BendersRuntimeMetrics, BendersStopReason, FeasibleSolution,
    FeasibleSolutionV, FrameworkSolveOptions, LinearDualSolution, LinearDualSolutionV,
};
use ospf_rust_core::error::{CoreError, Result, SolverError};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::intermediate::{LinearTriadModel, QuadraticTetradModel};
use ospf_rust_core::solver::{SolveValue, SolveValueConversionPolicy, SolverOutput};

/// 线性子问题结果 / Linear Sub-problem Result
#[derive(Debug, Clone)]
pub enum LinearSubResult {
    /// 可行解 / Feasible solution
    Feasible(LinearFeasibleResult),
    /// 不可行解 / Infeasible solution
    Infeasible(LinearInfeasibleResult),
}

/// 线性子问题结果（typed）/ Linear sub-problem result (typed)
#[derive(Debug, Clone)]
pub enum LinearSubResultV<V>
where
    V: SolveValue,
{
    /// 可行解 / Feasible solution
    Feasible(LinearFeasibleResultV<V>),
    /// 不可行解 / Infeasible solution
    Infeasible(LinearInfeasibleResultV<V>),
}

/// 线性可行结果 / Linear Feasible Result
#[derive(Debug, Clone)]
pub struct LinearFeasibleResult {
    /// 求解结果 / Solver result
    pub result: FeasibleSolution,
    /// 对偶解 / Dual solution
    pub dual_solution: LinearDualSolution,
    /// Benders 切割 / Benders cuts
    pub cuts: Option<Vec<LinearCut>>,
}

/// 线性可行结果（typed）/ Linear feasible result (typed)
#[derive(Debug, Clone)]
pub struct LinearFeasibleResultV<V>
where
    V: SolveValue,
{
    /// 求解结果 / Solver result
    pub result: FeasibleSolutionV<V>,
    /// 对偶解 / Dual solution
    pub dual_solution: LinearDualSolutionV<V>,
    /// Benders 切割 / Benders cuts
    pub cuts: Option<Vec<LinearCut>>,
}

impl LinearFeasibleResult {
    /// 创建新的线性可行结果 / Create new linear feasible result
    pub fn new(result: FeasibleSolution, dual_solution: LinearDualSolution) -> Self {
        Self {
            result,
            dual_solution,
            cuts: None,
        }
    }

    /// 添加切割 / Add cuts
    pub fn with_cuts(mut self, cuts: Vec<LinearCut>) -> Self {
        self.cuts = Some(cuts);
        self
    }

    /// 转换为 typed 可行结果 / Convert into typed feasible result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<LinearFeasibleResultV<V>>
    where
        V: SolveValue,
    {
        Ok(LinearFeasibleResultV {
            result: self.result.try_into_typed(policy)?,
            dual_solution: self.dual_solution.try_into_typed(policy)?,
            cuts: self.cuts,
        })
    }
}

/// 线性不可行结果 / Linear Infeasible Result
#[derive(Debug, Clone)]
pub struct LinearInfeasibleResult {
    /// Farkas 对偶解 / Farkas dual solution
    pub farkas_dual_solution: LinearDualSolution,
    /// Benders 切割 / Benders cuts
    pub cuts: Option<Vec<LinearCut>>,
}

/// 线性不可行结果（typed）/ Linear infeasible result (typed)
#[derive(Debug, Clone)]
pub struct LinearInfeasibleResultV<V>
where
    V: SolveValue,
{
    /// Farkas 对偶解 / Farkas dual solution
    pub farkas_dual_solution: LinearDualSolutionV<V>,
    /// Benders 切割 / Benders cuts
    pub cuts: Option<Vec<LinearCut>>,
}

impl LinearInfeasibleResult {
    /// 创建新的线性不可行结果 / Create new linear infeasible result
    pub fn new(farkas_dual_solution: LinearDualSolution) -> Self {
        Self {
            farkas_dual_solution,
            cuts: None,
        }
    }

    /// 添加切割 / Add cuts
    pub fn with_cuts(mut self, cuts: Vec<LinearCut>) -> Self {
        self.cuts = Some(cuts);
        self
    }

    /// 转换为 typed 不可行结果 / Convert into typed infeasible result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<LinearInfeasibleResultV<V>>
    where
        V: SolveValue,
    {
        Ok(LinearInfeasibleResultV {
            farkas_dual_solution: self.farkas_dual_solution.try_into_typed(policy)?,
            cuts: self.cuts,
        })
    }
}

impl LinearSubResult {
    /// 转换为 typed 子结果 / Convert into typed sub result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<LinearSubResultV<V>>
    where
        V: SolveValue,
    {
        match self {
            LinearSubResult::Feasible(feasible) => {
                Ok(LinearSubResultV::Feasible(feasible.try_into_typed(policy)?))
            }
            LinearSubResult::Infeasible(infeasible) => Ok(LinearSubResultV::Infeasible(
                infeasible.try_into_typed(policy)?,
            )),
        }
    }
}

/// 线性切割 / Linear Cut
///
/// Benders 切割，用于添加到主问题中。
/// Benders cut, used to add to the master problem.
#[derive(Debug, Clone)]
pub struct LinearCut {
    /// 切割名称 / Cut name
    pub name: String,
    /// 切割系数 / Cut coefficients
    pub coefficients: Vec<(usize, f64)>,
    /// 切割右端项 / Cut right-hand side
    pub rhs: f64,
    /// 切割方向 / Cut sense (<=, >=, =)
    pub sense: CutSense,
}

impl LinearCut {
    /// 创建新的线性切割 / Create new linear cut
    pub fn new(
        name: impl Into<String>,
        coefficients: Vec<(usize, f64)>,
        rhs: f64,
        sense: CutSense,
    ) -> Self {
        Self {
            name: name.into(),
            coefficients,
            rhs,
            sense,
        }
    }

    /// 创建小于等于切割 / Create less-or-equal cut
    pub fn le(name: impl Into<String>, coefficients: Vec<(usize, f64)>, rhs: f64) -> Self {
        Self::new(name, coefficients, rhs, CutSense::LessOrEqual)
    }

    /// 创建大于等于切割 / Create greater-or-equal cut
    pub fn ge(name: impl Into<String>, coefficients: Vec<(usize, f64)>, rhs: f64) -> Self {
        Self::new(name, coefficients, rhs, CutSense::GreaterOrEqual)
    }

    /// 创建等式切割 / Create equality cut
    pub fn eq(name: impl Into<String>, coefficients: Vec<(usize, f64)>, rhs: f64) -> Self {
        Self::new(name, coefficients, rhs, CutSense::Equal)
    }
}

/// 切割方向 / Cut Sense
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutSense {
    /// 小于等于 / Less or equal
    LessOrEqual,
    /// 大于等于 / Greater or equal
    GreaterOrEqual,
    /// 等于 / Equal
    Equal,
}

fn build_benders_runtime_metrics(
    executed_iterations: usize,
    best_solution_iteration: Option<usize>,
    total_cuts: usize,
    no_cut_iterations: usize,
    no_obj_improvement_iterations: usize,
    stop_reason: Option<BendersStopReason>,
    iteration_snapshots: Vec<BendersIterationSnapshot>,
) -> BendersRuntimeMetrics {
    BendersRuntimeMetrics {
        executed_iterations,
        best_solution_iteration,
        total_cuts,
        no_cut_iterations,
        no_obj_improvement_iterations,
        stop_reason,
        iteration_snapshots,
    }
}

fn master_solution_from_output(output: &SolverOutput) -> Result<Vec<f64>> {
    output
        .solution
        .clone()
        .ok_or(CoreError::Solver(SolverError::Infeasible))
}

fn master_objective_from_output(output: &SolverOutput) -> f64 {
    output.objective_value.unwrap_or(0.0)
}

/// 线性 Benders 分解求解器 trait / Linear Benders Decomposition Solver Trait
///
/// 定义线性 Benders 分解算法的求解器接口。
/// Defines solver interface for linear Benders decomposition algorithms.
///
/// 推荐应用层入口：`solve_meta` 与 `solve_meta_with_options`。
/// Recommended application-facing entries are `solve_meta` and `solve_meta_with_options`.
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait LinearBendersDecompositionSolver: Send + Sync {
    /// 获取求解器名称 / Get solver name
    fn name(&self) -> &str;

    /// 求解主问题 / Solve master problem
    #[cfg(feature = "async")]
    async fn solve_master(
        &self,
        model: &LinearTriadModel,
        cuts: &[LinearCut],
    ) -> Result<SolverOutput>;

    /// 求解主问题（同步）/ Solve master problem (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_master(&self, model: &LinearTriadModel, cuts: &[LinearCut]) -> Result<SolverOutput>;

    /// 求解子问题 / Solve sub-problem
    #[cfg(feature = "async")]
    async fn solve_sub(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
    ) -> Result<LinearSubResult>;

    /// 求解子问题（typed，async）/ Solve sub-problem with typed output (async)
    #[cfg(feature = "async")]
    fn solve_sub_typed<'a, V>(
        &'a self,
        model: &'a LinearTriadModel,
        master_solution: &'a [f64],
        policy: SolveValueConversionPolicy,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LinearSubResultV<V>>> + Send + 'a>>
    where
        V: SolveValue,
    {
        Box::pin(async move {
            self.solve_sub(model, master_solution)
                .await?
                .try_into_typed(policy)
        })
    }

    /// 求解子问题（同步）/ Solve sub-problem (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_sub(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
    ) -> Result<LinearSubResult>;

    /// 求解子问题（typed，同步）/ Solve sub-problem with typed output (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_sub_typed<V>(
        &self,
        model: &LinearTriadModel,
        master_solution: &[f64],
        policy: SolveValueConversionPolicy,
    ) -> Result<LinearSubResultV<V>>
    where
        V: SolveValue,
    {
        self.solve_sub(model, master_solution)?
            .try_into_typed(policy)
    }

    /// 使用 MetaModel 执行 Benders 分解（async）/
    /// Execute Benders decomposition from MetaModel (async)
    #[cfg(feature = "async")]
    fn solve_meta<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        self.solve_meta_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行 Benders 分解（简化参数对象，async）/
    /// Execute Benders decomposition from MetaModel with simplified options object (async)
    #[cfg(feature = "async")]
    fn solve_meta_with_options<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        let master_mechanism_model = match master_meta_model
            .try_to_mechanism_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_mechanism_model = match sub_meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let master_mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &master_mechanism_model,
            options.value_conversion_policy,
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &sub_mechanism_model,
            options.value_conversion_policy,
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let master_model = match master_mechanism_model
            .try_into_linear_triad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_model = match sub_mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        Box::pin(async move {
            self.solve_with_options(&master_model, &sub_model, options)
                .await
        })
    }

    /// 使用 MetaModel 执行 Benders 分解并返回 typed 结果（async）/
    /// Execute Benders decomposition from MetaModel and return typed result (async)
    #[cfg(feature = "async")]
    fn solve_meta_typed<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<FeasibleSolutionV<V>>> + Send + 'a>,
    >
    where
        V: SolveValue,
    {
        self.solve_meta_typed_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行 Benders 分解并返回 typed 结果（参数对象，async）/
    /// Execute Benders decomposition from MetaModel and return typed result with options (async)
    #[cfg(feature = "async")]
    fn solve_meta_typed_with_options<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<FeasibleSolutionV<V>>> + Send + 'a>,
    >
    where
        V: SolveValue,
    {
        let policy = options.value_conversion_policy;
        let solve_future = self.solve_meta_with_options(master_meta_model, sub_meta_model, options);
        Box::pin(async move { solve_future.await?.try_into_typed(policy) })
    }

    /// 使用 MetaModel 执行 Benders 分解（同步）/
    /// Execute Benders decomposition from MetaModel (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
    ) -> Result<FeasibleSolution>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        self.solve_meta_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行 Benders 分解（简化参数对象，同步）/
    /// Execute Benders decomposition from MetaModel with simplified options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_with_options<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        let master_mechanism_model = master_meta_model
            .try_to_mechanism_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            )?;
        let sub_mechanism_model = sub_meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        let master_mechanism_model = ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &master_mechanism_model,
            options.value_conversion_policy,
        )?;
        let sub_mechanism_model = ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &sub_mechanism_model,
            options.value_conversion_policy,
        )?;
        let master_model = master_mechanism_model
            .try_into_linear_triad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            )?;
        let sub_model = sub_mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        self.solve_with_options(&master_model, &sub_model, options)
    }

    /// 使用 MetaModel 执行 Benders 分解并返回 typed 结果（同步）/
    /// Execute Benders decomposition from MetaModel and return typed result (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_typed<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
    ) -> Result<FeasibleSolutionV<V>>
    where
        V: SolveValue,
    {
        self.solve_meta_typed_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行 Benders 分解并返回 typed 结果（参数对象，同步）/
    /// Execute Benders decomposition from MetaModel and return typed result with options (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_typed_with_options<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolutionV<V>>
    where
        V: SolveValue,
    {
        let policy = options.value_conversion_policy;
        self.solve_meta_with_options(master_meta_model, sub_meta_model, options)?
            .try_into_typed(policy)
    }

    /// 使用参数对象执行 Benders 分解（async）/
    /// Execute Benders decomposition with options object (async)
    #[cfg(feature = "async")]
    fn solve_with_options<'a>(
        &'a self,
        master_model: &'a LinearTriadModel,
        sub_model: &'a LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    {
        Box::pin(async move {
            self.solve(
                master_model,
                sub_model,
                options.max_iterations,
                options.tolerance,
                options.max_stall_iterations,
                options.objective_stall_iterations,
            )
            .await
        })
    }

    /// 使用参数对象执行 Benders 分解（同步）/
    /// Execute Benders decomposition with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_with_options(
        &self,
        master_model: &LinearTriadModel,
        sub_model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve(
            master_model,
            sub_model,
            options.max_iterations,
            options.tolerance,
            options.max_stall_iterations,
            options.objective_stall_iterations,
        )
    }

    /// 执行 Benders 分解迭代 / Execute Benders decomposition iteration
    #[cfg(feature = "async")]
    async fn solve(
        &self,
        master_model: &LinearTriadModel,
        sub_model: &LinearTriadModel,
        max_iterations: usize,
        tolerance: f64,
        max_stall_iterations: Option<usize>,
        objective_stall_iterations: Option<usize>,
    ) -> Result<FeasibleSolution> {
        let mut cuts = Vec::new();
        let mut best_solution: Option<FeasibleSolution> = None;
        let mut best_solution_iterations: Option<usize> = None;
        let mut prev_obj = f64::NEG_INFINITY;
        let max_stall_iterations = max_stall_iterations.filter(|window| *window > 0);
        let objective_stall_iterations = objective_stall_iterations.unwrap_or(1).max(1);
        let mut no_cut_iterations = 0usize;
        let mut no_obj_improvement_iterations = 0usize;
        let mut executed_iterations = 0usize;
        let mut stop_reason: Option<BendersStopReason> = None;
        let mut iteration_snapshots: Vec<BendersIterationSnapshot> = Vec::new();

        for iter in 0..max_iterations {
            executed_iterations = iter + 1;
            // 求解主问题 / Solve master problem
            let master_result = self.solve_master(master_model, &cuts).await?;

            let master_solution = master_solution_from_output(&master_result)?;
            let master_obj = master_objective_from_output(&master_result);

            if (master_obj - prev_obj).abs() < tolerance {
                no_obj_improvement_iterations += 1;
            } else {
                no_obj_improvement_iterations = 0;
            }
            prev_obj = master_obj;

            // 求解子问题 / Solve sub-problem
            let cuts_before = cuts.len();
            let sub_result = self.solve_sub(sub_model, &master_solution).await?;

            match sub_result {
                LinearSubResult::Feasible(feasible) => {
                    // 更新最优解 / Update best solution
                    best_solution = Some(feasible.result.clone());
                    best_solution_iterations = Some(iter + 1);

                    // 添加切割（如果有）/ Add cuts if present
                    if let Some(new_cuts) = feasible.cuts {
                        cuts.extend(new_cuts);
                    }
                }
                LinearSubResult::Infeasible(infeasible) => {
                    // 添加可行性切割 / Add feasibility cuts
                    if let Some(new_cuts) = infeasible.cuts {
                        cuts.extend(new_cuts);
                    }
                }
            }
            let cuts_added = cuts.len().saturating_sub(cuts_before);

            if cuts.len() > cuts_before {
                no_cut_iterations = 0;
            } else {
                no_cut_iterations += 1;
            }
            iteration_snapshots.push(BendersIterationSnapshot {
                iteration: iter + 1,
                master_obj,
                master_gap: master_result.mip_gap,
                cuts_added,
                total_cuts: cuts.len(),
                no_cut_iterations,
                no_obj_improvement_iterations,
            });
            if let Some(window) = max_stall_iterations {
                if no_cut_iterations >= window {
                    stop_reason = Some(BendersStopReason::CutStall);
                    log::info!(
                        "Benders decomposition stopped at iteration {} due to cut stagnation window {}",
                        iter + 1,
                        window
                    );
                    break;
                }
            }

            if no_obj_improvement_iterations >= objective_stall_iterations {
                stop_reason = Some(BendersStopReason::ObjectiveStall);
                log::info!(
                    "Benders decomposition stopped at iteration {} due to objective stall window {}",
                    iter + 1,
                    objective_stall_iterations
                );
                break;
            }
        }

        if stop_reason.is_none() && executed_iterations >= max_iterations && max_iterations > 0 {
            stop_reason = Some(BendersStopReason::IterationLimit);
        }

        match best_solution {
            Some(solution) => {
                let runtime_metrics = build_benders_runtime_metrics(
                    executed_iterations,
                    best_solution_iterations,
                    cuts.len(),
                    no_cut_iterations,
                    no_obj_improvement_iterations,
                    stop_reason,
                    iteration_snapshots,
                );
                Ok(match best_solution_iterations {
                    Some(iterations) => solution
                        .with_benders_iterations(iterations)
                        .with_benders_runtime_metrics(runtime_metrics),
                    None => solution.with_benders_runtime_metrics(runtime_metrics),
                })
            }
            None => Err(CoreError::Solver(SolverError::SolveFailed(
                "Benders decomposition did not converge".into(),
            ))),
        }
    }

    /// 执行 Benders 分解迭代（同步）/ Execute Benders decomposition iteration (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve(
        &self,
        master_model: &LinearTriadModel,
        sub_model: &LinearTriadModel,
        max_iterations: usize,
        tolerance: f64,
        max_stall_iterations: Option<usize>,
        objective_stall_iterations: Option<usize>,
    ) -> Result<FeasibleSolution> {
        let mut cuts = Vec::new();
        let mut best_solution: Option<FeasibleSolution> = None;
        let mut best_solution_iterations: Option<usize> = None;
        let mut prev_obj = f64::NEG_INFINITY;
        let max_stall_iterations = max_stall_iterations.filter(|window| *window > 0);
        let objective_stall_iterations = objective_stall_iterations.unwrap_or(1).max(1);
        let mut no_cut_iterations = 0usize;
        let mut no_obj_improvement_iterations = 0usize;
        let mut executed_iterations = 0usize;
        let mut stop_reason: Option<BendersStopReason> = None;
        let mut iteration_snapshots: Vec<BendersIterationSnapshot> = Vec::new();

        for iter in 0..max_iterations {
            executed_iterations = iter + 1;
            // 求解主问题 / Solve master problem
            let master_result = self.solve_master(master_model, &cuts)?;

            let master_solution = master_solution_from_output(&master_result)?;
            let master_obj = master_objective_from_output(&master_result);

            if (master_obj - prev_obj).abs() < tolerance {
                no_obj_improvement_iterations += 1;
            } else {
                no_obj_improvement_iterations = 0;
            }
            prev_obj = master_obj;

            // 求解子问题 / Solve sub-problem
            let cuts_before = cuts.len();
            let sub_result = self.solve_sub(sub_model, &master_solution)?;

            match sub_result {
                LinearSubResult::Feasible(feasible) => {
                    // 更新最优解 / Update best solution
                    best_solution = Some(feasible.result.clone());
                    best_solution_iterations = Some(iter + 1);

                    // 添加切割（如果有）/ Add cuts if present
                    if let Some(new_cuts) = feasible.cuts {
                        cuts.extend(new_cuts);
                    }
                }
                LinearSubResult::Infeasible(infeasible) => {
                    // 添加可行性切割 / Add feasibility cuts
                    if let Some(new_cuts) = infeasible.cuts {
                        cuts.extend(new_cuts);
                    }
                }
            }
            let cuts_added = cuts.len().saturating_sub(cuts_before);

            if cuts.len() > cuts_before {
                no_cut_iterations = 0;
            } else {
                no_cut_iterations += 1;
            }
            iteration_snapshots.push(BendersIterationSnapshot {
                iteration: iter + 1,
                master_obj,
                master_gap: master_result.mip_gap,
                cuts_added,
                total_cuts: cuts.len(),
                no_cut_iterations,
                no_obj_improvement_iterations,
            });
            if let Some(window) = max_stall_iterations {
                if no_cut_iterations >= window {
                    stop_reason = Some(BendersStopReason::CutStall);
                    log::info!(
                        "Benders decomposition stopped at iteration {} due to cut stagnation window {}",
                        iter + 1,
                        window
                    );
                    break;
                }
            }

            if no_obj_improvement_iterations >= objective_stall_iterations {
                stop_reason = Some(BendersStopReason::ObjectiveStall);
                log::info!(
                    "Benders decomposition stopped at iteration {} due to objective stall window {}",
                    iter + 1,
                    objective_stall_iterations
                );
                break;
            }
        }

        if stop_reason.is_none() && executed_iterations >= max_iterations && max_iterations > 0 {
            stop_reason = Some(BendersStopReason::IterationLimit);
        }

        match best_solution {
            Some(solution) => {
                let runtime_metrics = build_benders_runtime_metrics(
                    executed_iterations,
                    best_solution_iterations,
                    cuts.len(),
                    no_cut_iterations,
                    no_obj_improvement_iterations,
                    stop_reason,
                    iteration_snapshots,
                );
                Ok(match best_solution_iterations {
                    Some(iterations) => solution
                        .with_benders_iterations(iterations)
                        .with_benders_runtime_metrics(runtime_metrics),
                    None => solution.with_benders_runtime_metrics(runtime_metrics),
                })
            }
            None => Err(CoreError::Solver(SolverError::SolveFailed(
                "Benders decomposition did not converge".into(),
            ))),
        }
    }
}

/// 二次切割 / Quadratic Cut
///
/// 扩展线性切割，支持二次项。
/// Extends linear cut to support quadratic terms.
#[derive(Debug, Clone)]
pub struct QuadraticCut {
    /// 基础线性切割 / Base linear cut
    pub linear: LinearCut,
    /// 二次项系数 / Quadratic coefficients
    pub quadratic_coefficients: Vec<((usize, usize), f64)>,
}

impl QuadraticCut {
    /// 创建新的二次切割 / Create new quadratic cut
    pub fn new(
        name: impl Into<String>,
        linear_coefficients: Vec<(usize, f64)>,
        quadratic_coefficients: Vec<((usize, usize), f64)>,
        rhs: f64,
        sense: CutSense,
    ) -> Self {
        Self {
            linear: LinearCut::new(name, linear_coefficients, rhs, sense),
            quadratic_coefficients,
        }
    }
}

/// 二次子问题结果 / Quadratic Sub-problem Result
#[derive(Debug, Clone)]
pub enum QuadraticSubResult {
    /// 可行解 / Feasible solution
    Feasible(QuadraticFeasibleResult),
    /// 不可行解 / Infeasible solution
    Infeasible(QuadraticInfeasibleResult),
}

/// 二次子问题结果（typed）/ Quadratic sub-problem result (typed)
#[derive(Debug, Clone)]
pub enum QuadraticSubResultV<V>
where
    V: SolveValue,
{
    /// 可行解 / Feasible solution
    Feasible(QuadraticFeasibleResultV<V>),
    /// 不可行解 / Infeasible solution
    Infeasible(QuadraticInfeasibleResultV<V>),
}

/// 二次可行结果 / Quadratic Feasible Result
#[derive(Debug, Clone)]
pub struct QuadraticFeasibleResult {
    /// 线性可行结果 / Linear feasible result
    pub linear: LinearFeasibleResult,
    /// 二次切割 / Quadratic cuts
    pub quadratic_cuts: Option<Vec<QuadraticCut>>,
}

/// 二次可行结果（typed）/ Quadratic feasible result (typed)
#[derive(Debug, Clone)]
pub struct QuadraticFeasibleResultV<V>
where
    V: SolveValue,
{
    /// 线性可行结果 / Linear feasible result
    pub linear: LinearFeasibleResultV<V>,
    /// 二次切割 / Quadratic cuts
    pub quadratic_cuts: Option<Vec<QuadraticCut>>,
}

impl QuadraticFeasibleResult {
    /// 创建新的二次可行结果 / Create new quadratic feasible result
    pub fn new(result: LinearFeasibleResult) -> Self {
        Self {
            linear: result,
            quadratic_cuts: None,
        }
    }

    /// 添加二次切割 / Add quadratic cuts
    pub fn with_quadratic_cuts(mut self, cuts: Vec<QuadraticCut>) -> Self {
        self.quadratic_cuts = Some(cuts);
        self
    }

    /// 转换为 typed 可行结果 / Convert into typed feasible result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<QuadraticFeasibleResultV<V>>
    where
        V: SolveValue,
    {
        Ok(QuadraticFeasibleResultV {
            linear: self.linear.try_into_typed(policy)?,
            quadratic_cuts: self.quadratic_cuts,
        })
    }
}

/// 二次不可行结果 / Quadratic Infeasible Result
#[derive(Debug, Clone)]
pub struct QuadraticInfeasibleResult {
    /// 线性不可行结果 / Linear infeasible result
    pub linear: LinearInfeasibleResult,
    /// 二次切割 / Quadratic cuts
    pub quadratic_cuts: Option<Vec<QuadraticCut>>,
}

/// 二次不可行结果（typed）/ Quadratic infeasible result (typed)
#[derive(Debug, Clone)]
pub struct QuadraticInfeasibleResultV<V>
where
    V: SolveValue,
{
    /// 线性不可行结果 / Linear infeasible result
    pub linear: LinearInfeasibleResultV<V>,
    /// 二次切割 / Quadratic cuts
    pub quadratic_cuts: Option<Vec<QuadraticCut>>,
}

impl QuadraticInfeasibleResult {
    /// 创建新的二次不可行结果 / Create new quadratic infeasible result
    pub fn new(result: LinearInfeasibleResult) -> Self {
        Self {
            linear: result,
            quadratic_cuts: None,
        }
    }

    /// 添加二次切割 / Add quadratic cuts
    pub fn with_quadratic_cuts(mut self, cuts: Vec<QuadraticCut>) -> Self {
        self.quadratic_cuts = Some(cuts);
        self
    }

    /// 转换为 typed 不可行结果 / Convert into typed infeasible result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<QuadraticInfeasibleResultV<V>>
    where
        V: SolveValue,
    {
        Ok(QuadraticInfeasibleResultV {
            linear: self.linear.try_into_typed(policy)?,
            quadratic_cuts: self.quadratic_cuts,
        })
    }
}

impl QuadraticSubResult {
    /// 转换为 typed 子结果 / Convert into typed sub result
    pub fn try_into_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<QuadraticSubResultV<V>>
    where
        V: SolveValue,
    {
        match self {
            QuadraticSubResult::Feasible(feasible) => Ok(QuadraticSubResultV::Feasible(
                feasible.try_into_typed(policy)?,
            )),
            QuadraticSubResult::Infeasible(infeasible) => Ok(QuadraticSubResultV::Infeasible(
                infeasible.try_into_typed(policy)?,
            )),
        }
    }
}

/// 二次 Benders 分解求解器 trait / Quadratic Benders Decomposition Solver Trait
///
/// 在 `LinearBendersDecompositionSolver` 基础上扩展二次主问题/子问题接口。
/// Extends `LinearBendersDecompositionSolver` with quadratic master/sub interfaces.
///
/// 推荐应用层入口：`solve_meta_quadratic` 与 `solve_meta_quadratic_with_options`。
/// Recommended application-facing entries are `solve_meta_quadratic` and `solve_meta_quadratic_with_options`.
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait QuadraticBendersDecompositionSolver: LinearBendersDecompositionSolver {
    /// 求解二次主问题 / Solve quadratic master problem
    #[cfg(feature = "async")]
    async fn solve_master_quadratic(
        &self,
        model: &QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
    ) -> Result<SolverOutput>;

    /// 求解二次主问题（同步）/ Solve quadratic master problem (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_master_quadratic(
        &self,
        model: &QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
    ) -> Result<SolverOutput>;

    /// 求解二次子问题 / Solve quadratic sub-problem
    #[cfg(feature = "async")]
    async fn solve_sub_quadratic(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
    ) -> Result<QuadraticSubResult>;

    /// 求解二次子问题（typed，async）/
    /// Solve quadratic sub-problem with typed output (async)
    #[cfg(feature = "async")]
    fn solve_sub_quadratic_typed<'a, V>(
        &'a self,
        model: &'a QuadraticTetradModel,
        master_solution: &'a [f64],
        policy: SolveValueConversionPolicy,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<QuadraticSubResultV<V>>> + Send + 'a>,
    >
    where
        V: SolveValue,
    {
        Box::pin(async move {
            self.solve_sub_quadratic(model, master_solution)
                .await?
                .try_into_typed(policy)
        })
    }

    /// 求解二次子问题（同步）/ Solve quadratic sub-problem (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_sub_quadratic(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
    ) -> Result<QuadraticSubResult>;

    /// 求解二次子问题（typed，同步）/
    /// Solve quadratic sub-problem with typed output (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_sub_quadratic_typed<V>(
        &self,
        model: &QuadraticTetradModel,
        master_solution: &[f64],
        policy: SolveValueConversionPolicy,
    ) -> Result<QuadraticSubResultV<V>>
    where
        V: SolveValue,
    {
        self.solve_sub_quadratic(model, master_solution)?
            .try_into_typed(policy)
    }

    /// 使用 MetaModel 执行二次 Benders 分解（async）/
    /// Execute quadratic Benders decomposition from MetaModel (async)
    #[cfg(feature = "async")]
    fn solve_meta_quadratic<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        self.solve_meta_quadratic_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行二次 Benders 分解（简化参数对象，async）/
    /// Execute quadratic Benders decomposition from MetaModel with simplified options object (async)
    #[cfg(feature = "async")]
    fn solve_meta_quadratic_with_options<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        let master_mechanism_model = match master_meta_model
            .try_to_mechanism_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_mechanism_model = match sub_meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let master_mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &master_mechanism_model,
            options.value_conversion_policy,
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &sub_mechanism_model,
            options.value_conversion_policy,
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let master_model = match master_mechanism_model
            .try_into_quadratic_tetrad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let sub_model = match sub_mechanism_model
            .try_into_quadratic_tetrad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        Box::pin(async move {
            self.solve_quadratic_with_options(&master_model, &sub_model, options)
                .await
        })
    }

    /// 使用 MetaModel 执行二次 Benders 分解并返回 typed 结果（async）/
    /// Execute quadratic Benders decomposition from MetaModel and return typed result (async)
    #[cfg(feature = "async")]
    fn solve_meta_quadratic_typed<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<FeasibleSolutionV<V>>> + Send + 'a>,
    >
    where
        V: SolveValue,
    {
        self.solve_meta_quadratic_typed_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行二次 Benders 分解并返回 typed 结果（参数对象，async）/
    /// Execute quadratic Benders decomposition from MetaModel and return typed result with options (async)
    #[cfg(feature = "async")]
    fn solve_meta_quadratic_typed_with_options<'a, V>(
        &'a self,
        master_meta_model: &'a MetaModel<V>,
        sub_meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<FeasibleSolutionV<V>>> + Send + 'a>,
    >
    where
        V: SolveValue,
    {
        let policy = options.value_conversion_policy;
        let solve_future =
            self.solve_meta_quadratic_with_options(master_meta_model, sub_meta_model, options);
        Box::pin(async move { solve_future.await?.try_into_typed(policy) })
    }

    /// 使用 MetaModel 执行二次 Benders 分解（同步）/
    /// Execute quadratic Benders decomposition from MetaModel (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_quadratic<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
    ) -> Result<FeasibleSolution>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        self.solve_meta_quadratic_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行二次 Benders 分解（简化参数对象，同步）/
    /// Execute quadratic Benders decomposition from MetaModel with simplified options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_quadratic_with_options<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        let master_mechanism_model = master_meta_model
            .try_to_mechanism_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            )?;
        let sub_mechanism_model = sub_meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        let master_mechanism_model = ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &master_mechanism_model,
            options.value_conversion_policy,
        )?;
        let sub_mechanism_model = ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &sub_mechanism_model,
            options.value_conversion_policy,
        )?;
        let master_model = master_mechanism_model
            .try_into_quadratic_tetrad_model_with_status_callback(
                options.model_building_status_callback.as_ref(),
            )?;
        let sub_model = sub_mechanism_model.try_into_quadratic_tetrad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        self.solve_quadratic_with_options(&master_model, &sub_model, options)
    }

    /// 使用 MetaModel 执行二次 Benders 分解并返回 typed 结果（同步）/
    /// Execute quadratic Benders decomposition from MetaModel and return typed result (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_quadratic_typed<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
    ) -> Result<FeasibleSolutionV<V>>
    where
        V: SolveValue,
    {
        self.solve_meta_quadratic_typed_with_options(
            master_meta_model,
            sub_meta_model,
            FrameworkSolveOptions::default(),
        )
    }

    /// 使用 MetaModel 执行二次 Benders 分解并返回 typed 结果（参数对象，同步）/
    /// Execute quadratic Benders decomposition from MetaModel and return typed result with options (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_meta_quadratic_typed_with_options<V>(
        &self,
        master_meta_model: &MetaModel<V>,
        sub_meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolutionV<V>>
    where
        V: SolveValue,
    {
        let policy = options.value_conversion_policy;
        self.solve_meta_quadratic_with_options(master_meta_model, sub_meta_model, options)?
            .try_into_typed(policy)
    }

    /// 使用参数对象执行二次 Benders 分解（async）/
    /// Execute quadratic Benders decomposition with options object (async)
    #[cfg(feature = "async")]
    fn solve_quadratic_with_options<'a>(
        &'a self,
        master_model: &'a QuadraticTetradModel,
        sub_model: &'a QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    {
        Box::pin(async move {
            self.solve_quadratic(
                master_model,
                sub_model,
                options.max_iterations,
                options.tolerance,
                options.max_stall_iterations,
                options.objective_stall_iterations,
            )
            .await
        })
    }

    /// 使用参数对象执行二次 Benders 分解（同步）/
    /// Execute quadratic Benders decomposition with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_quadratic_with_options(
        &self,
        master_model: &QuadraticTetradModel,
        sub_model: &QuadraticTetradModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve_quadratic(
            master_model,
            sub_model,
            options.max_iterations,
            options.tolerance,
            options.max_stall_iterations,
            options.objective_stall_iterations,
        )
    }

    /// 执行二次 Benders 分解迭代 / Execute quadratic Benders decomposition iteration
    #[cfg(feature = "async")]
    async fn solve_quadratic(
        &self,
        master_model: &QuadraticTetradModel,
        sub_model: &QuadraticTetradModel,
        max_iterations: usize,
        tolerance: f64,
        max_stall_iterations: Option<usize>,
        objective_stall_iterations: Option<usize>,
    ) -> Result<FeasibleSolution> {
        let mut linear_cuts = Vec::new();
        let mut quadratic_cuts = Vec::new();
        let mut best_solution: Option<FeasibleSolution> = None;
        let mut best_solution_iterations: Option<usize> = None;
        let mut prev_obj = f64::NEG_INFINITY;
        let max_stall_iterations = max_stall_iterations.filter(|window| *window > 0);
        let objective_stall_iterations = objective_stall_iterations.unwrap_or(1).max(1);
        let mut no_cut_iterations = 0usize;
        let mut no_obj_improvement_iterations = 0usize;
        let mut executed_iterations = 0usize;
        let mut stop_reason: Option<BendersStopReason> = None;
        let mut iteration_snapshots: Vec<BendersIterationSnapshot> = Vec::new();

        for iter in 0..max_iterations {
            executed_iterations = iter + 1;
            let master_result = self
                .solve_master_quadratic(master_model, &linear_cuts, &quadratic_cuts)
                .await?;
            let master_solution = master_solution_from_output(&master_result)?;
            let master_obj = master_objective_from_output(&master_result);

            if (master_obj - prev_obj).abs() < tolerance {
                no_obj_improvement_iterations += 1;
            } else {
                no_obj_improvement_iterations = 0;
            }
            prev_obj = master_obj;

            let cuts_before = linear_cuts.len() + quadratic_cuts.len();
            let sub_result = self
                .solve_sub_quadratic(sub_model, &master_solution)
                .await?;
            match sub_result {
                QuadraticSubResult::Feasible(feasible) => {
                    best_solution = Some(feasible.linear.result.clone());
                    best_solution_iterations = Some(iter + 1);
                    if let Some(new_linear_cuts) = feasible.linear.cuts {
                        linear_cuts.extend(new_linear_cuts);
                    }
                    if let Some(new_quadratic_cuts) = feasible.quadratic_cuts {
                        quadratic_cuts.extend(new_quadratic_cuts);
                    }
                }
                QuadraticSubResult::Infeasible(infeasible) => {
                    if let Some(new_linear_cuts) = infeasible.linear.cuts {
                        linear_cuts.extend(new_linear_cuts);
                    }
                    if let Some(new_quadratic_cuts) = infeasible.quadratic_cuts {
                        quadratic_cuts.extend(new_quadratic_cuts);
                    }
                }
            }
            let total_cuts_now = linear_cuts.len() + quadratic_cuts.len();
            let cuts_added = total_cuts_now.saturating_sub(cuts_before);

            if total_cuts_now > cuts_before {
                no_cut_iterations = 0;
            } else {
                no_cut_iterations += 1;
            }
            iteration_snapshots.push(BendersIterationSnapshot {
                iteration: iter + 1,
                master_obj,
                master_gap: master_result.mip_gap,
                cuts_added,
                total_cuts: total_cuts_now,
                no_cut_iterations,
                no_obj_improvement_iterations,
            });
            if let Some(window) = max_stall_iterations {
                if no_cut_iterations >= window {
                    stop_reason = Some(BendersStopReason::CutStall);
                    log::info!(
                        "Quadratic Benders decomposition stopped at iteration {} due to cut stagnation window {}",
                        iter + 1,
                        window
                    );
                    break;
                }
            }

            if no_obj_improvement_iterations >= objective_stall_iterations {
                stop_reason = Some(BendersStopReason::ObjectiveStall);
                log::info!(
                    "Quadratic Benders decomposition stopped at iteration {} due to objective stall window {}",
                    iter + 1,
                    objective_stall_iterations
                );
                break;
            }
        }

        if stop_reason.is_none() && executed_iterations >= max_iterations && max_iterations > 0 {
            stop_reason = Some(BendersStopReason::IterationLimit);
        }

        match best_solution {
            Some(solution) => {
                let runtime_metrics = build_benders_runtime_metrics(
                    executed_iterations,
                    best_solution_iterations,
                    linear_cuts.len() + quadratic_cuts.len(),
                    no_cut_iterations,
                    no_obj_improvement_iterations,
                    stop_reason,
                    iteration_snapshots,
                );
                Ok(match best_solution_iterations {
                    Some(iterations) => solution
                        .with_benders_iterations(iterations)
                        .with_benders_runtime_metrics(runtime_metrics),
                    None => solution.with_benders_runtime_metrics(runtime_metrics),
                })
            }
            None => Err(CoreError::Solver(SolverError::SolveFailed(
                "Quadratic Benders decomposition did not converge".into(),
            ))),
        }
    }

    /// 执行二次 Benders 分解迭代（同步）/ Execute quadratic Benders decomposition iteration (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_quadratic(
        &self,
        master_model: &QuadraticTetradModel,
        sub_model: &QuadraticTetradModel,
        max_iterations: usize,
        tolerance: f64,
        max_stall_iterations: Option<usize>,
        objective_stall_iterations: Option<usize>,
    ) -> Result<FeasibleSolution> {
        let mut linear_cuts = Vec::new();
        let mut quadratic_cuts = Vec::new();
        let mut best_solution: Option<FeasibleSolution> = None;
        let mut best_solution_iterations: Option<usize> = None;
        let mut prev_obj = f64::NEG_INFINITY;
        let max_stall_iterations = max_stall_iterations.filter(|window| *window > 0);
        let objective_stall_iterations = objective_stall_iterations.unwrap_or(1).max(1);
        let mut no_cut_iterations = 0usize;
        let mut no_obj_improvement_iterations = 0usize;
        let mut executed_iterations = 0usize;
        let mut stop_reason: Option<BendersStopReason> = None;
        let mut iteration_snapshots: Vec<BendersIterationSnapshot> = Vec::new();

        for iter in 0..max_iterations {
            executed_iterations = iter + 1;
            let master_result =
                self.solve_master_quadratic(master_model, &linear_cuts, &quadratic_cuts)?;
            let master_solution = master_solution_from_output(&master_result)?;
            let master_obj = master_objective_from_output(&master_result);

            if (master_obj - prev_obj).abs() < tolerance {
                no_obj_improvement_iterations += 1;
            } else {
                no_obj_improvement_iterations = 0;
            }
            prev_obj = master_obj;

            let cuts_before = linear_cuts.len() + quadratic_cuts.len();
            let sub_result = self.solve_sub_quadratic(sub_model, &master_solution)?;
            match sub_result {
                QuadraticSubResult::Feasible(feasible) => {
                    best_solution = Some(feasible.linear.result.clone());
                    best_solution_iterations = Some(iter + 1);
                    if let Some(new_linear_cuts) = feasible.linear.cuts {
                        linear_cuts.extend(new_linear_cuts);
                    }
                    if let Some(new_quadratic_cuts) = feasible.quadratic_cuts {
                        quadratic_cuts.extend(new_quadratic_cuts);
                    }
                }
                QuadraticSubResult::Infeasible(infeasible) => {
                    if let Some(new_linear_cuts) = infeasible.linear.cuts {
                        linear_cuts.extend(new_linear_cuts);
                    }
                    if let Some(new_quadratic_cuts) = infeasible.quadratic_cuts {
                        quadratic_cuts.extend(new_quadratic_cuts);
                    }
                }
            }
            let total_cuts_now = linear_cuts.len() + quadratic_cuts.len();
            let cuts_added = total_cuts_now.saturating_sub(cuts_before);

            if total_cuts_now > cuts_before {
                no_cut_iterations = 0;
            } else {
                no_cut_iterations += 1;
            }
            iteration_snapshots.push(BendersIterationSnapshot {
                iteration: iter + 1,
                master_obj,
                master_gap: master_result.mip_gap,
                cuts_added,
                total_cuts: total_cuts_now,
                no_cut_iterations,
                no_obj_improvement_iterations,
            });
            if let Some(window) = max_stall_iterations {
                if no_cut_iterations >= window {
                    stop_reason = Some(BendersStopReason::CutStall);
                    log::info!(
                        "Quadratic Benders decomposition stopped at iteration {} due to cut stagnation window {}",
                        iter + 1,
                        window
                    );
                    break;
                }
            }

            if no_obj_improvement_iterations >= objective_stall_iterations {
                stop_reason = Some(BendersStopReason::ObjectiveStall);
                log::info!(
                    "Quadratic Benders decomposition stopped at iteration {} due to objective stall window {}",
                    iter + 1,
                    objective_stall_iterations
                );
                break;
            }
        }

        if stop_reason.is_none() && executed_iterations >= max_iterations && max_iterations > 0 {
            stop_reason = Some(BendersStopReason::IterationLimit);
        }

        match best_solution {
            Some(solution) => {
                let runtime_metrics = build_benders_runtime_metrics(
                    executed_iterations,
                    best_solution_iterations,
                    linear_cuts.len() + quadratic_cuts.len(),
                    no_cut_iterations,
                    no_obj_improvement_iterations,
                    stop_reason,
                    iteration_snapshots,
                );
                Ok(match best_solution_iterations {
                    Some(iterations) => solution
                        .with_benders_iterations(iterations)
                        .with_benders_runtime_metrics(runtime_metrics),
                    None => solution.with_benders_runtime_metrics(runtime_metrics),
                })
            }
            None => Err(CoreError::Solver(SolverError::SolveFailed(
                "Quadratic Benders decomposition did not converge".into(),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::model::intermediate::{
        BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
    };
    use ospf_rust_core::model::{ConstraintRelation, MetaModel};
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::{ModelBuildingStage, ModelBuildingStatusCallback};
    use ospf_rust_core::variable::ContinuousVariableItem;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[cfg(not(feature = "async"))]
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_linear_cut() {
        let cut = LinearCut::le("test_cut", vec![(0, 1.0), (1, 2.0)], 10.0);
        assert_eq!(cut.name, "test_cut");
        assert_eq!(cut.rhs, 10.0);
        assert_eq!(cut.sense, CutSense::LessOrEqual);
    }

    #[test]
    fn test_quadratic_cut() {
        let cut = QuadraticCut::new(
            "quad_cut",
            vec![(0, 1.0)],
            vec![((0, 1), 2.0)],
            5.0,
            CutSense::LessOrEqual,
        );
        assert_eq!(cut.linear.name, "quad_cut");
        assert_eq!(cut.quadratic_coefficients.len(), 1);
    }

    struct MockQuadraticBendersSolver {
        call_index: AtomicUsize,
    }

    impl MockQuadraticBendersSolver {
        fn new() -> Self {
            Self {
                call_index: AtomicUsize::new(0),
            }
        }
    }

    fn build_meta_model(name: &str) -> MetaModel<f64> {
        let mut model = MetaModel::<f64>::new(name);
        let x = ContinuousVariableItem::auto(&format!("{}_x", name));
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                &format!("{}_c", name),
            )
            .unwrap();
        model
    }

    #[cfg(not(feature = "async"))]
    fn build_non_finite_meta_model(name: &str) -> MetaModel<f64> {
        let mut model = MetaModel::<f64>::new(name);
        let x = ContinuousVariableItem::auto(&format!("{}_x", name));
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, f64::NAN)],
                ConstraintRelation::LessEqual,
                1.0,
                &format!("{}_non_finite_c", name),
            )
            .unwrap();
        model
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl LinearBendersDecompositionSolver for MockQuadraticBendersSolver {
        fn name(&self) -> &str {
            "mock_quadratic_benders"
        }

        #[cfg(feature = "async")]
        async fn solve_master(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(0.0, vec![0.0]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_master(
            &self,
            _model: &LinearTriadModel,
            _cuts: &[LinearCut],
        ) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(0.0, vec![0.0]))
        }

        #[cfg(feature = "async")]
        async fn solve_sub(
            &self,
            _model: &LinearTriadModel,
            _master_solution: &[f64],
        ) -> Result<LinearSubResult> {
            Ok(LinearSubResult::Feasible(LinearFeasibleResult::new(
                FeasibleSolution::new(0.0, vec![0.0]),
                LinearDualSolution::default(),
            )))
        }

        #[cfg(not(feature = "async"))]
        fn solve_sub(
            &self,
            _model: &LinearTriadModel,
            _master_solution: &[f64],
        ) -> Result<LinearSubResult> {
            Ok(LinearSubResult::Feasible(LinearFeasibleResult::new(
                FeasibleSolution::new(0.0, vec![0.0]),
                LinearDualSolution::default(),
            )))
        }
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl QuadraticBendersDecompositionSolver for MockQuadraticBendersSolver {
        #[cfg(feature = "async")]
        async fn solve_master_quadratic(
            &self,
            _model: &QuadraticTetradModel,
            _linear_cuts: &[LinearCut],
            _quadratic_cuts: &[QuadraticCut],
        ) -> Result<SolverOutput> {
            let index = self.call_index.fetch_add(1, Ordering::SeqCst);
            let objective = match index {
                0 => 10.0,
                1 => 5.0,
                _ => 5.0,
            };
            Ok(SolverOutput::optimal(objective, vec![objective]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_master_quadratic(
            &self,
            _model: &QuadraticTetradModel,
            _linear_cuts: &[LinearCut],
            _quadratic_cuts: &[QuadraticCut],
        ) -> Result<SolverOutput> {
            let index = self.call_index.fetch_add(1, Ordering::SeqCst);
            let objective = match index {
                0 => 10.0,
                1 => 5.0,
                _ => 5.0,
            };
            Ok(SolverOutput::optimal(objective, vec![objective]))
        }

        #[cfg(feature = "async")]
        async fn solve_sub_quadratic(
            &self,
            _model: &QuadraticTetradModel,
            master_solution: &[f64],
        ) -> Result<QuadraticSubResult> {
            let objective = master_solution.first().copied().unwrap_or(0.0);
            Ok(QuadraticSubResult::Feasible(QuadraticFeasibleResult::new(
                LinearFeasibleResult::new(
                    FeasibleSolution::new(objective, master_solution.to_vec()),
                    LinearDualSolution::default(),
                ),
            )))
        }

        #[cfg(not(feature = "async"))]
        fn solve_sub_quadratic(
            &self,
            _model: &QuadraticTetradModel,
            master_solution: &[f64],
        ) -> Result<QuadraticSubResult> {
            let objective = master_solution.first().copied().unwrap_or(0.0);
            Ok(QuadraticSubResult::Feasible(QuadraticFeasibleResult::new(
                LinearFeasibleResult::new(
                    FeasibleSolution::new(objective, master_solution.to_vec()),
                    LinearDualSolution::default(),
                ),
            )))
        }
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_quadratic_benders_trait_default_solve_converges() {
        let solver = MockQuadraticBendersSolver::new();
        let master_model =
            QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("master"));
        let sub_model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("sub"));

        let result = solver
            .solve_quadratic(&master_model, &sub_model, 10, 1e-9, None, None)
            .await
            .expect("quadratic benders solve should converge");

        assert!((result.obj - 5.0).abs() <= 1e-9);
        assert_eq!(result.solution, vec![5.0]);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_quadratic_benders_trait_default_solve_converges() {
        let solver = MockQuadraticBendersSolver::new();
        let master_model =
            QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("master"));
        let sub_model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("sub"));

        let result = solver
            .solve_quadratic(&master_model, &sub_model, 10, 1e-9, None, None)
            .expect("quadratic benders solve should converge");

        assert!((result.obj - 5.0).abs() <= 1e-9);
        assert_eq!(result.solution, vec![5.0]);
        assert!(result.benders_iterations.is_some());
        assert!(result.benders_runtime_metrics.is_some());
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_meta_model_shortcut_builds_then_delegates() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta");
        let stages = Arc::new(Mutex::new(Vec::new()));
        let stages_for_callback = stages.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            stages_for_callback.lock().unwrap().push(status.stage);
            Ok(())
        });

        let options = FrameworkSolveOptions::new()
            .with_building_callback(Some(callback))
            .with_iterations(5, 1e-9);
        let result = solver
            .solve_meta_with_options(&master_meta_model, &sub_meta_model, options)
            .expect("linear benders meta-model shortcut should succeed");

        assert!((result.obj - 0.0).abs() <= 1e-9);
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
    fn test_linear_benders_meta_model_with_options_builds_then_delegates() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta_options");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_options");
        let stages = Arc::new(Mutex::new(Vec::new()));
        let stages_for_callback = stages.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            stages_for_callback.lock().unwrap().push(status.stage);
            Ok(())
        });

        let options = FrameworkSolveOptions::new()
            .with_building_callback(Some(callback))
            .with_iterations(5, 1e-9);
        let result = solver
            .solve_meta_with_options(&master_meta_model, &sub_meta_model, options)
            .expect("linear benders meta-model options shortcut should succeed");

        assert!((result.obj - 0.0).abs() <= 1e-9);
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
    fn test_linear_sub_result_try_into_typed_works() {
        let raw = LinearSubResult::Feasible(LinearFeasibleResult::new(
            FeasibleSolution::new(2.0, vec![1.0, 3.0]),
            LinearDualSolution::new(vec![0.5], vec![-0.25]),
        ));
        let typed = raw
            .try_into_typed::<f64>(ospf_rust_core::solver::SolveValueConversionPolicy::Strict)
            .expect("typed linear sub result conversion should succeed");
        match typed {
            LinearSubResultV::Feasible(feasible) => {
                assert!((feasible.result.obj - 2.0).abs() <= 1e-9);
                assert_eq!(feasible.result.solution.len(), 2);
                assert_eq!(feasible.dual_solution.constraints.len(), 1);
            }
            LinearSubResultV::Infeasible(_) => panic!("expected feasible typed sub result"),
        }
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_solve_sub_typed_shortcut_works() {
        let solver = MockQuadraticBendersSolver::new();
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("linear_sub_typed"));
        let typed = solver
            .solve_sub_typed::<f64>(
                &model,
                &[0.0],
                ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
            )
            .expect("typed linear sub shortcut should succeed");
        match typed {
            LinearSubResultV::Feasible(feasible) => {
                assert!((feasible.result.obj - 0.0).abs() <= 1e-9);
            }
            LinearSubResultV::Infeasible(_) => panic!("expected feasible typed linear sub result"),
        }
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_objective_stall_window_stops_early() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_objective_stall_master");
        let sub_meta_model = build_meta_model("linear_benders_objective_stall_sub");
        let options = FrameworkSolveOptions::new()
            .with_iterations(10, 1e-9)
            .with_objective_stall_iterations(2);

        let result = solver
            .solve_meta_with_options(&master_meta_model, &sub_meta_model, options)
            .expect("linear benders objective stall should return best feasible solution");

        assert_eq!(result.benders_iterations, Some(3));
        let runtime_metrics = result
            .benders_runtime_metrics
            .as_ref()
            .expect("runtime metrics should be set");
        assert_eq!(runtime_metrics.executed_iterations, 3);
        assert_eq!(
            runtime_metrics.stop_reason,
            Some(BendersStopReason::ObjectiveStall)
        );
        assert_eq!(runtime_metrics.iteration_snapshots.len(), 3);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_cut_stall_window_stops_early() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_cut_stall_master");
        let sub_meta_model = build_meta_model("linear_benders_cut_stall_sub");
        let options = FrameworkSolveOptions::new()
            .with_iterations(10, 1e-9)
            .with_stall_iterations(2)
            .with_objective_stall_iterations(10);

        let result = solver
            .solve_meta_with_options(&master_meta_model, &sub_meta_model, options)
            .expect("linear benders cut stall should return best feasible solution");

        assert_eq!(result.benders_iterations, Some(2));
        let runtime_metrics = result
            .benders_runtime_metrics
            .as_ref()
            .expect("runtime metrics should be set");
        assert_eq!(runtime_metrics.executed_iterations, 2);
        assert_eq!(
            runtime_metrics.stop_reason,
            Some(BendersStopReason::CutStall)
        );
        assert_eq!(runtime_metrics.iteration_snapshots.len(), 2);
    }

    #[cfg(feature = "async")]
    fn assert_send<T: Send>(_: &T) {}

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_linear_benders_meta_model_shortcut_returns_send_future() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta_async_send");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_async_send");

        let future = solver.solve_meta(&master_meta_model, &sub_meta_model);
        assert_send(&future);

        let result = future
            .await
            .expect("linear benders meta-model shortcut should succeed");
        assert!((result.obj - 0.0).abs() <= 1e-9);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_linear_benders_meta_model_with_options_returns_send_future() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta_async_send_options");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_async_send_options");
        let options = FrameworkSolveOptions::new().with_iterations(5, 1e-9);

        let future = solver.solve_meta_with_options(&master_meta_model, &sub_meta_model, options);
        assert_send(&future);

        let result = future
            .await
            .expect("linear benders meta-model options shortcut should succeed");
        assert!((result.obj - 0.0).abs() <= 1e-9);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_quadratic_benders_meta_model_shortcut_returns_send_future() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("quadratic_benders_master_meta");
        let sub_meta_model = build_meta_model("quadratic_benders_sub_meta");

        let future = solver.solve_meta_quadratic(&master_meta_model, &sub_meta_model);
        assert_send(&future);

        let result = future
            .await
            .expect("quadratic benders meta-model shortcut should succeed");
        assert!((result.obj - 5.0).abs() <= 1e-9);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_quadratic_benders_meta_model_with_options_returns_send_future() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("quadratic_benders_master_meta_options_send");
        let sub_meta_model = build_meta_model("quadratic_benders_sub_meta_options_send");
        let options = FrameworkSolveOptions::new().with_iterations(5, 1e-9);

        let future =
            solver.solve_meta_quadratic_with_options(&master_meta_model, &sub_meta_model, options);
        assert_send(&future);

        let result = future
            .await
            .expect("quadratic benders meta-model options shortcut should succeed");
        assert!((result.obj - 5.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_meta_model_simplified_alias_builds_then_delegates() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta_alias");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_alias");

        let result = solver
            .solve_meta(&master_meta_model, &sub_meta_model)
            .expect("linear benders simplified alias should succeed");
        assert!((result.obj - 0.0).abs() <= 1e-9);
        assert!(result.benders_iterations.is_some());
        assert!(result.benders_runtime_metrics.is_some());
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_meta_model_shortcut_rejects_non_finite_value() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model =
            build_non_finite_meta_model("linear_benders_master_meta_non_finite");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_non_finite");
        let options = FrameworkSolveOptions::new().with_value_conversion_policy(
            ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
        );

        let error = solver
            .solve_meta_with_options(&master_meta_model, &sub_meta_model, options)
            .expect_err(
                "strict mode should reject non-finite conversion in linear benders meta path",
            );
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::NonFinite(_))
        ));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_linear_benders_meta_typed_shortcut_builds_then_delegates() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("linear_benders_master_meta_typed");
        let sub_meta_model = build_meta_model("linear_benders_sub_meta_typed");

        let result = solver
            .solve_meta_typed_with_options(
                &master_meta_model,
                &sub_meta_model,
                FrameworkSolveOptions::new().with_iterations(5, 1e-9),
            )
            .expect("linear benders typed meta shortcut should succeed");
        assert!((result.obj - 0.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_quadratic_benders_meta_model_shortcut_rejects_non_finite_value() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model =
            build_non_finite_meta_model("quadratic_benders_master_meta_non_finite");
        let sub_meta_model = build_meta_model("quadratic_benders_sub_meta_non_finite");
        let options = FrameworkSolveOptions::new().with_value_conversion_policy(
            ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
        );

        let error = solver
            .solve_meta_quadratic_with_options(&master_meta_model, &sub_meta_model, options)
            .expect_err(
                "strict mode should reject non-finite conversion in quadratic benders meta path",
            );
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::NonFinite(_))
        ));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_quadratic_sub_result_try_into_typed_works() {
        let raw =
            QuadraticSubResult::Feasible(QuadraticFeasibleResult::new(LinearFeasibleResult::new(
                FeasibleSolution::new(4.0, vec![2.0]),
                LinearDualSolution::new(vec![1.0], vec![0.0]),
            )));
        let typed = raw
            .try_into_typed::<f64>(ospf_rust_core::solver::SolveValueConversionPolicy::Strict)
            .expect("typed quadratic sub result conversion should succeed");
        match typed {
            QuadraticSubResultV::Feasible(feasible) => {
                assert!((feasible.linear.result.obj - 4.0).abs() <= 1e-9);
                assert_eq!(feasible.linear.result.solution.len(), 1);
            }
            QuadraticSubResultV::Infeasible(_) => panic!("expected feasible typed sub result"),
        }
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_quadratic_benders_solve_sub_typed_shortcut_works() {
        let solver = MockQuadraticBendersSolver::new();
        let model =
            QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("quadratic_sub_typed"));
        let typed = solver
            .solve_sub_quadratic_typed::<f64>(
                &model,
                &[3.0],
                ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
            )
            .expect("typed quadratic sub shortcut should succeed");
        match typed {
            QuadraticSubResultV::Feasible(feasible) => {
                assert!((feasible.linear.result.obj - 3.0).abs() <= 1e-9);
            }
            QuadraticSubResultV::Infeasible(_) => {
                panic!("expected feasible typed quadratic sub result")
            }
        }
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_quadratic_benders_meta_typed_shortcut_builds_then_delegates() {
        let solver = MockQuadraticBendersSolver::new();
        let master_meta_model = build_meta_model("quadratic_benders_master_meta_typed");
        let sub_meta_model = build_meta_model("quadratic_benders_sub_meta_typed");

        let result = solver
            .solve_meta_quadratic_typed_with_options(
                &master_meta_model,
                &sub_meta_model,
                FrameworkSolveOptions::new().with_iterations(5, 1e-9),
            )
            .expect("quadratic benders typed meta shortcut should succeed");
        assert!((result.obj - 5.0).abs() <= 1e-9);
    }
}
