#![doc(hidden)]

//! 兼容入口：旧路径转发到新路径
//! Compatibility entry: legacy path forwards to aligned path

#[deprecated(note = "use crate::solver::output::solver_output::* instead")]
/// 旧 `solver_output` 路径的兼容转发；新代码应使用 `solver::output::solver_output`。
/// Compatibility forwarding for the legacy `solver_output` path; new code should use `solver::output::solver_output`.
pub use crate::solver::output::solver_output::*;
