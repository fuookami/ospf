//! 常用建模入口预导入模块
//! Prelude module for common modeling entry points

pub use crate::error::{CoreError, ModelError, Result, SolverError, VariableError};
pub use crate::model::basic::{ConstraintPriority, ConstraintPriorityStats};
pub use crate::model::intermediate::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, DumpOptions, LPExportableModel,
    LinearElasticBuilder, LinearTriadModel, LinearTriadModelView, ModelFileFormat,
    QuadraticElasticBuilder, QuadraticTetradModel, QuadraticTetradModelView, SparseMatrix,
    SparseVector, dump_batch, dump_lp_batch,
};
pub use crate::model::{
    ConstraintGroup, ConstraintRelation, LinearConstraint, LinearConstraintInput,
    LinearExpressionBuilder, LinearInequality, MetaModel, ModelBuildingStage, ModelBuildingStatus,
    ModelBuildingStatusCallback, Objective, ObjectiveCategory, QuadraticConstraint,
    QuadraticInequality, SubObjective, SymbolicLinearInequality, SymbolicQuadraticInequality,
};
pub use crate::solver::iis::{IISConfig, LinearIISModel};
#[cfg(feature = "async")]
pub use crate::solver::{
    AsyncSolveOptions, AsyncSolver, SolveJoinHandle, solve_async_with_callback,
    solve_async_with_options, spawn_solve, spawn_solve_with_callback, spawn_solve_with_options,
};
pub use crate::solver::{
    ConfigurableSolver, FeasibleSolverOutput, LinearSolver, MultiSolutionOutput, QuadraticSolver,
    SolveOptions, SolveOptionsBuilder, SolveValue, SolveValueConversionPolicy, Solver,
    SolverConfig, SolverExt, SolverInfo, SolverOutput, SolverOutputWithIIS, SolverStatus,
    SolvingStatus, SolvingStatusCallback, TypedMultiSolutionOutput,
};
pub use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
pub use crate::symbol::function::*;
pub use crate::token::{
    AnyVariable, ConcurrentTokenList, ConcurrentTokenTable, Token, TokenList, TokenTable,
};
pub use crate::variable::*;
