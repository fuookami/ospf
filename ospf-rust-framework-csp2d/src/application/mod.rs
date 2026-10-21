//! CSP2D 应用层 / CSP2D application layer
//!
//! 映射 Kotlin `csp2d-application`：问题定义、MILP 路径与列生成路径、结果富化。
//! Maps the Kotlin `csp2d-application` submodule: the problem definition, the MILP path and the column
//! generation path, plus result enrichment.

pub mod master_solver;
pub mod model;
pub mod service;

pub use master_solver::{
    Csp2dHeuristicMasterSolver, Csp2dMasterSolveResult, Csp2dMasterSolver,
    Csp2dMasterTerminalStatus, require_deliverable,
};
pub use model::{Csp2dConfiguration, Csp2dProblem, demand_of, placeholder_product_id};
pub use service::{Csp2dSolution, Csp2dSolvers, Csp2dTerminationReason, require_valid};
