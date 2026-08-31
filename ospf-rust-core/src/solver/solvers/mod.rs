//! 求解器实现
//! Solver Implementations

pub mod pso;

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub mod gurobi;

#[cfg(feature = "scip")]
pub mod scip;

// 重导出 / Re-exports
pub use pso::ParticleSwarmHeuristicSolver;

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub use gurobi::GurobiSolver;

#[cfg(feature = "scip")]
pub use scip::SCIPSolver;
