//! 任务应用服务 / Task application services
//!
//! 提供任务级列生成和分支定价算法入口。
//! Provides task-level column generation and branch-and-price algorithm entry points.

use crate::application::algorithm::policy::ColumnGenerationPolicy;
use crate::application::algorithm::task_column_generation::TaskColumnGenerationAlgorithm;
use crate::domain::task_compilation::context::IterativeTaskCompilationContext;
use ospf_rust_framework::solver::column_generation_solver::ColumnGenerationSolver;

/// 创建任务级列生成算法实例 / Create task-level column generation algorithm instance
pub fn create_task_column_generation<C, S>(
    context: C,
    solver: S,
    configuration: ColumnGenerationPolicy,
) -> TaskColumnGenerationAlgorithm<C, S>
where
    C: IterativeTaskCompilationContext,
    S: ColumnGenerationSolver,
{
    TaskColumnGenerationAlgorithm::new(context, solver, configuration)
}
