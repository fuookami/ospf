#![cfg_attr(feature = "nightly", feature(unboxed_closures, fn_traits))]

//! OSPF Rust Framework
//!
//! 运筹学建模框架的高级封装和工具。
//! High-level abstractions and utilities for operations research modeling.
//!
//! # 模块 / Modules
//!
//! - [`model`] - 模型框架模块，提供 Pipeline 和 Shadow Price 管理
//! - [`solver`] - 求解器框架模块，提供组合求解器和 Benders 分解

pub mod model;
pub mod solver;

// 重导出常用类型 / Re-export common types
pub use model::{
    BasicShadowPriceMap, CGPipeline, HAPipeline, HAPipelineObj, Pipeline, PipelineList,
    ShadowPrice, ShadowPriceKey, ShadowPriceMap,
};

pub use solver::{
    ColumnGenerationSolver, CutSense, FeasibleSolution, LPResult, LinearBendersDecompositionSolver,
    LinearCut, LinearDualSolution, LinearFeasibleResult, LinearInfeasibleResult, LinearSolver,
    LinearSubResult, MetaDualSolution, ParallelCombinatorialColumnGenerationSolver,
    ParallelCombinatorialLinearSolver, ParallelCombinatorialMode,
    ParallelCombinatorialQuadraticSolver, QuadraticBendersDecompositionSolver, QuadraticCut,
    QuadraticFeasibleResult, QuadraticInfeasibleResult, QuadraticSolver, QuadraticSubResult,
    RegistrationStatus, SerialCombinatorialColumnGenerationSolver, SerialCombinatorialLinearSolver,
    SerialCombinatorialQuadraticSolver, SolveOptions, SolvingStatus,
};

#[cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]
pub use solver::{
    GurobiBendersDecompositionSolver, GurobiColumnGenerationSolver,
    GurobiLinearBendersDecompositionSolver,
};

#[cfg(feature = "scip")]
pub use solver::{
    ScipColumnGenerationSolver, ScipLinearBendersDecompositionSolver,
    ScipQuadraticBendersDecompositionSolver,
};

// 日志初始化 / Log initialization
#[cfg(feature = "async")]
pub fn init_logging() {
    let _ = log::set_logger(&crate::LOGGER);
    log::set_max_level(log::LevelFilter::Info);
}

#[cfg(feature = "async")]
static LOGGER: SimpleLogger = SimpleLogger;

#[cfg(feature = "async")]
struct SimpleLogger;

#[cfg(feature = "async")]
impl log::Log for SimpleLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        println!("[{}] {}", record.level(), record.args());
    }

    fn flush(&self) {}
}
