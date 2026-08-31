//! 后端求解器实现入口（Kotlin 对齐 + Rust feature 兼容）
//! Backend solver implementations (Kotlin-aligned + Rust feature compatibility)

pub mod pso {
    pub use crate::solver::solvers::pso::*;
}

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub mod gurobi {
    pub use crate::solver::solvers::gurobi::*;
}

#[cfg(feature = "scip")]
pub mod scip {
    pub use crate::solver::solvers::scip::*;
}

pub use pso::*;

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub use gurobi::*;

#[cfg(feature = "scip")]
pub use scip::*;
