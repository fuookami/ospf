//! 可配置选择型超启发式搜索 / Configurable selection hyper-heuristic search
//!
//! 高层仅调度持久化会话，不替代低层原生生命周期。
//! The upper layer schedules persistent sessions without replacing native algorithm lifecycles.

pub mod config;
pub mod components;
pub mod learning;
pub mod memory;
pub mod operators;
pub mod operator_registry;
mod parallel;
pub mod persistence;
pub mod problem;
pub mod registry;
pub mod selection;
pub mod scheduling;
pub mod schedule;
pub mod solver;

pub use config::*;
pub use components::*;
pub use operator_registry::*;
pub use selection::*;
pub use registry::{AlgorithmContext, SearchRegistry};
pub use schedule::{ScheduleCheckpoint, ScheduleEvent, ScheduleLog};
pub use solver::{HyperHeuristicRun, HyperHeuristicSolver, RunOptions, SearchResult};

#[cfg(test)]
mod tests;

use crate::solver::heuristic::result::HeuristicError;

pub(crate) fn invalid(message: impl Into<String>) -> HeuristicError {
    HeuristicError::InvalidArgument(message.into())
}

pub(crate) fn boundary<T>(operation: impl FnOnce() -> crate::solver::heuristic::result::Result<T>) -> crate::solver::heuristic::result::Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation))
        .unwrap_or_else(|_| Err(HeuristicError::Model("search callback panicked / 搜索回调异常".into())))
}
