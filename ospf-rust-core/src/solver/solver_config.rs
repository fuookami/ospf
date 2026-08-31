#![doc(hidden)]

//! 兼容入口：旧路径转发到新路径
//! Compatibility entry: legacy path forwards to aligned path

#[deprecated(note = "use crate::solver::config::solver_config::* instead")]
pub use crate::solver::config::solver_config::*;
