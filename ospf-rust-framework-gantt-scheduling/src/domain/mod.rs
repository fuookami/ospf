//! 领域层 / Domain layer
//!
//! 按 Kotlin Gantt Scheduling 子模块映射为 Rust 包。
//! Maps Kotlin Gantt Scheduling submodules into Rust modules.

pub mod bunch_compilation;
pub mod bunch_generation;
pub mod capacity_scheduling;
pub mod common;
pub mod error;
pub mod produce;
pub mod resource;
pub mod task;
pub mod task_compilation;
pub mod task_generation;
