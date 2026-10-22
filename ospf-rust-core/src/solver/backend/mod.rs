//! 后端求解器实现入口（Kotlin 对齐 + Rust feature 兼容）
//! Backend solver implementations (Kotlin-aligned + Rust feature compatibility)

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub mod gurobi {
    //! Gurobi 原生后端入口 / Gurobi native backend entry point.

    pub use crate::solver::solvers::gurobi::*;
}

#[cfg(feature = "scip")]
pub mod scip {
    //! SCIP 原生后端入口 / SCIP native backend entry point.

    pub use crate::solver::solvers::scip::*;
}

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub use gurobi::*;

#[cfg(feature = "scip")]
pub use scip::*;
