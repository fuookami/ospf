//! 求解器框架模块
//! Solver Framework Module
//!
//! 本模块提供运筹学求解器的高级封装和组合求解器。
//! This module provides high-level abstractions and combinatorial solvers for operations research.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`ColumnGenerationSolver`] - 列生成求解器 trait
//! - [`ParallelCombinatorialMode`] - 并行组合模式
//! - [`ParallelCombinatorialLinearSolver`] - 并行组合线性求解器
//! - [`SerialCombinatorialLinearSolver`] - 串行组合线性求解器
//! - [`LinearBendersDecompositionSolver`] - 线性 Benders 分解求解器
//! - `Gurobi*` / `Scip*` - 插件风格扩展求解器接口实现

#![allow(unused_imports)]

#[doc(hidden)]
pub mod benders_decomposition;
#[doc(hidden)]
pub mod column_generation;
pub mod column_generation_solver;
#[cfg(any(
    feature = "scip",
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12"
))]
mod core_extensions;
pub mod framework_async;
pub mod framework_number_aliases;
pub mod framework_solve_options;
pub mod linear_benders_decomposition_solver;
#[doc(hidden)]
pub mod options;
#[doc(hidden)]
pub mod parallel_column_generation;
pub mod parallel_combinatorial_column_generation_solver;
pub mod parallel_combinatorial_linear_solver;
pub mod parallel_combinatorial_mode;
pub mod parallel_combinatorial_quadratic_solver;
#[doc(hidden)]
pub mod parallel_linear;
#[doc(hidden)]
pub mod parallel_mode;
#[doc(hidden)]
pub mod parallel_quadratic;
pub mod quadratic_benders_decomposition_solver;
#[cfg(feature = "remote-solver")]
pub mod remote;
#[doc(hidden)]
pub mod serial_column_generation;
pub mod serial_combinatorial_column_generation_solver;
pub mod serial_combinatorial_linear_solver;
pub mod serial_combinatorial_quadratic_solver;
#[doc(hidden)]
pub mod serial_linear;
#[doc(hidden)]
pub mod serial_quadratic;

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub mod gurobi_extension;

#[cfg(feature = "scip")]
pub mod scip_extension;

pub use benders_decomposition::CutSense;
pub use column_generation_solver::*;
pub use framework_number_aliases::*;
pub use framework_solve_options::*;
pub use linear_benders_decomposition_solver::*;
// 兼容别名：保留 `SolveOptions`
// Compatibility alias: keep `SolveOptions`.
#[doc(hidden)]
pub use options::*;
pub use parallel_combinatorial_column_generation_solver::*;
pub use parallel_combinatorial_linear_solver::*;
pub use parallel_combinatorial_mode::*;
pub use parallel_combinatorial_quadratic_solver::*;
pub use quadratic_benders_decomposition_solver::*;

#[cfg(feature = "remote-solver")]
pub use remote::*;
pub use serial_combinatorial_column_generation_solver::*;
pub use serial_combinatorial_linear_solver::*;
pub use serial_combinatorial_quadratic_solver::*;

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub use gurobi_extension::*;

#[cfg(feature = "scip")]
pub use scip_extension::*;
