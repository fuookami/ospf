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

pub mod benders_decomposition;
pub mod column_generation;
#[cfg(any(
    feature = "scip",
    feature = "gurobi10",
    feature = "gurobi11",
    feature = "gurobi12"
))]
mod core_extensions;
pub mod options;
pub mod parallel_column_generation;
pub mod parallel_linear;
pub mod parallel_mode;
pub mod parallel_quadratic;
pub mod serial_column_generation;
pub mod serial_linear;
pub mod serial_quadratic;

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub mod gurobi_extension;

#[cfg(feature = "scip")]
pub mod scip_extension;

pub use benders_decomposition::*;
pub use column_generation::*;
pub use options::*;
pub use parallel_column_generation::*;
pub use parallel_linear::*;
pub use parallel_mode::*;
pub use parallel_quadratic::*;
pub use serial_column_generation::*;
pub use serial_linear::*;
pub use serial_quadratic::*;

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub use gurobi_extension::*;

#[cfg(feature = "scip")]
pub use scip_extension::*;
