//! 求解器实现
//! Solver Implementations
//!
//! 本目录只保留后端求解器适配；启发式算法统一由 [`crate::solver::heuristic`] 承载。
//! This directory keeps backend solver adapters only; metaheuristics live in [`crate::solver::heuristic`].

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub mod gurobi;

#[cfg(feature = "scip")]
pub mod scip;

// 重导出 / Re-exports
#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub use gurobi::GurobiSolver;

#[cfg(feature = "scip")]
pub use scip::{SCIPSolver, ScipSolver};
