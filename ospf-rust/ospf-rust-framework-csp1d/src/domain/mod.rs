//! 领域层 / Domain layer
//!
//! 按 Kotlin CSP1D 子模块映射为 Rust 包。
//! Maps Kotlin CSP1D submodules into Rust modules.

pub mod cutting_plan_generation;
pub mod error;
pub mod length_assignment;
pub mod material;
pub mod produce;
pub mod wasting_minimization;
pub mod r#yield;
