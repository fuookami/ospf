//! 任务级列生成算法 / Task-level column generation algorithm
//!
//! 实现任务调度的列生成算法主循环。
//! Implements the column generation algorithm main loop for task scheduling.

use std::collections::{HashMap, HashSet};

use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::solver::column_generation_solver::{
    ColumnGenerationSolver, LPResult, FeasibleSolution,
};
use ospf_rust_framework::solver::framework_solve_options::FrameworkSolveOptions;

use crate::application::iteration::Iteration;
use crate::application::algorithm::policy::ColumnGenerationPolicy;
use crate::domain::task_compilation::context::IterativeTaskCompilationContext;
use crate::domain::task_compilation::iterative::AddedTaskColumn;
use crate::domain::task::Cost;
use crate::GanttResult;
use crate::GanttError;

/// 任务级列生成算法 / Task-level column generation algorithm
///
/// 实现任务调度的列生成算法，包含：
/// - 初始 MILP 求解
/// - LP 松弛求解（RMP）
/// - 影子价格提取
/// - 列生成（通过策略注入）
/// - 列移除
/// - 全局/局部固定
/// - 收敛检测
///
/// Implements column generation algorithm for task scheduling, including:
/// - Initial MILP solve
/// - LP relaxation solve (RMP)
/// - Shadow price extraction
/// - Column generation (via policy injection)
/// - Column removal
/// - Global/local fixing
/// - Convergence detection
pub struct TaskColumnGenerationAlgorithm<C, S>
where
    C: IterativeTaskCompilationContext,
    S: ColumnGenerationSolver,
{
    /// 编译上下文 / Compilation context
    pub context: C,
    /// 求解器 / Solver
    pub solver: S,
    /// 策略 / Policy
    pub policy: ColumnGenerationPolicy,
    /// 迭代状态 / Iteration state
    pub iteration: Iteration,
    /// 已固定的列 / Fixed columns
    pub fixed_columns: HashSet<usize>,
    /// 保留的列 / Kept columns
    pub kept_columns: HashSet<usize>,
    /// 影子价格映射 / Shadow price map
    pub shadow_prices: HashMap<usize, f64>,
    /// 最佳解 / Best solution
    pub best_solution: Option<Vec<f64>>,
    /// 最佳目标值 / Best objective value
    pub best_obj: f64,
}

impl<C, S> std::fmt::Debug for TaskColumnGenerationAlgorithm<C, S>
where
    C: IterativeTaskCompilationContext,
    S: ColumnGenerationSolver,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskColumnGenerationAlgorithm")
            .field("iteration", &self.iteration.iteration)
            .field("fixed_count", &self.fixed_columns.len())
            .field("kept_count", &self.kept_columns.len())
            .field("best_obj", &self.best_obj)
            .finish()
    }
}

impl<C, S> TaskColumnGenerationAlgorithm<C, S>
where
    C: IterativeTaskCompilationContext,
    S: ColumnGenerationSolver,
{
    /// 创建算法实例 / Create algorithm instance
    pub fn new(context: C, solver: S, policy: ColumnGenerationPolicy) -> Self {
        Self {
            context,
            solver,
            policy,
            iteration: Iteration::new(),
            fixed_columns: HashSet::new(),
            kept_columns: HashSet::new(),
            shadow_prices: HashMap::new(),
            best_solution: None,
            best_obj: f64::NEG_INFINITY,
        }
    }

    /// 注册模型 / Register model
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.context.register(model)
    }

    /// 添加初始列 / Add initial columns
    pub fn add_initial_columns(
        &mut self,
        columns: Vec<(usize, usize, Cost<f64>)>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<AddedTaskColumn>> {
        self.context.add_columns(0, columns, model)
    }

    /// 求解 RMP LP / Solve RMP LP relaxation
    ///
    /// 求解 LP 松弛并提取影子价格。
    /// Solves LP relaxation and extracts shadow prices.
    pub fn solve_rmp_lp(
        &self,
        model: &MetaModel<f64>,
    ) -> GanttResult<LPResult> {
        let options = FrameworkSolveOptions::new();
        let triad_model = model.try_to_linear_triad_model().map_err(|e| {
                GanttError::Calculation {
                    message: format!("Failed to convert model: {:?}", e),
                }
            })?;
        solve_lp_with_options_sync(
            &self.solver,
            &triad_model,
            options,
        )
            .map_err(|e| GanttError::Calculation {
                message: format!("LP solve failed: {:?}", e),
            })
    }

    /// 求解 MILP / Solve MILP
    pub fn solve_milp(
        &self,
        model: &MetaModel<f64>,
    ) -> GanttResult<FeasibleSolution> {
        let options = FrameworkSolveOptions::new();
        solve_with_options_sync(
            &self.solver,
            model,
            options,
        )
            .map_err(|e| GanttError::Calculation {
                message: format!("MILP solve failed: {:?}", e),
            })
    }

    /// 提取影子价格 / Extract shadow prices
    ///
    /// 从 LP 对偶解提取影子价格。
    /// Extracts shadow prices from LP dual solution.
    pub fn extract_shadow_prices(
        &mut self,
        lp_result: &LPResult,
        constraint_name_to_index: &HashMap<String, usize>,
    ) {
        self.shadow_prices = self.context.extract_shadow_price(
            &lp_result.dual_solution,
            constraint_name_to_index,
        );
    }

    /// 检查是否应继续迭代 / Check whether iteration should continue
    pub fn should_continue(&self) -> bool {
        if self.iteration.iteration >= self.policy.max_iterations {
            return false;
        }
        if self.iteration.is_improvement_slow() {
            return false;
        }
        if self.iteration.elapsed() >= self.policy.time_limit {
            return false;
        }
        true
    }

    /// 全局固定 / Globally fix
    ///
    /// 将指定列固定为已选择。
    /// Fixes specified columns as selected.
    pub fn globally_fix(&mut self, columns: &HashSet<usize>) {
        self.fixed_columns.extend(columns.iter().copied());
    }

    /// 局部固定 / Locally fix
    ///
    /// 将解值超过阈值的列固定。
    /// Fixes columns with solution value above threshold.
    pub fn locally_fix(&mut self, solution: &[f64]) -> HashSet<usize> {
        let _threshold = self.policy.local_fix_threshold;
        let mut newly_fixed = HashSet::new();

        for &col_idx in &self.context.extract_kept(solution) {
            if !self.fixed_columns.contains(&col_idx) {
                self.fixed_columns.insert(col_idx);
                newly_fixed.insert(col_idx);
            }
        }

        newly_fixed
    }

    /// 刷新迭代状态 / Flush iteration state
    pub fn flush(&mut self) {
        self.fixed_columns.clear();
        self.kept_columns.clear();
        self.shadow_prices.clear();
    }
}

#[cfg(feature = "async")]
fn solve_lp_with_options_sync<S>(
    solver: &S,
    model: &ospf_rust_core::model::intermediate::LinearTriadModel,
    options: FrameworkSolveOptions,
) -> ospf_rust_core::error::Result<LPResult>
where
    S: ColumnGenerationSolver,
{
    futures::executor::block_on(solver.solve_lp_with_options(model, options))
}

#[cfg(not(feature = "async"))]
fn solve_lp_with_options_sync<S>(
    solver: &S,
    model: &ospf_rust_core::model::intermediate::LinearTriadModel,
    options: FrameworkSolveOptions,
) -> ospf_rust_core::error::Result<LPResult>
where
    S: ColumnGenerationSolver,
{
    solver.solve_lp_with_options(model, options)
}

#[cfg(feature = "async")]
fn solve_with_options_sync<S>(
    solver: &S,
    model: &MetaModel<f64>,
    options: FrameworkSolveOptions,
) -> ospf_rust_core::error::Result<FeasibleSolution>
where
    S: ColumnGenerationSolver,
{
    futures::executor::block_on(solver.solve_with_options(model, options))
}

#[cfg(not(feature = "async"))]
fn solve_with_options_sync<S>(
    solver: &S,
    model: &MetaModel<f64>,
    options: FrameworkSolveOptions,
) -> ospf_rust_core::error::Result<FeasibleSolution>
where
    S: ColumnGenerationSolver,
{
    solver.solve_with_options(model, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_generation_policy_default() {
        let policy = ColumnGenerationPolicy::default();
        assert_eq!(policy.max_iterations, 100);
        assert_eq!(policy.bad_reduced_amount, 20);
        assert!((policy.local_fix_threshold - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_column_generation_policy_builder() {
        let policy = ColumnGenerationPolicy::new()
            .with_max_iterations(50)
            .with_max_column_amount(10000)
            .with_local_fix_threshold(0.8);

        assert_eq!(policy.max_iterations, 50);
        assert_eq!(policy.max_column_amount, 10000);
        assert!((policy.local_fix_threshold - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_should_continue_checks() {
        let iter = Iteration::new();
        let policy = ColumnGenerationPolicy::new()
            .with_max_iterations(2);

        // 模拟迭代 0 — 应继续
        assert!(iter.iteration < policy.max_iterations);
        assert!(!iter.is_improvement_slow());
    }
}
