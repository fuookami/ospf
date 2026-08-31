#![doc(hidden)]

//! 兼容入口：旧路径转发到新路径
//! Compatibility entry: legacy path forwards to aligned path

#[deprecated(note = "use crate::solver::config::solver_config::* instead")]
/// 旧 `solver_config` 路径的兼容转发；新代码应使用 `solver::config::solver_config`。
/// Compatibility forwarding for the legacy `solver_config` path; new code should use `solver::config::solver_config`.
pub use crate::solver::config::solver_config::*;
