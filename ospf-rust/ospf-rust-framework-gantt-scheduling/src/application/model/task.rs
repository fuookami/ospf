//! 任务迭代模型 / Task iteration models
//!
//! 任务迭代状态委托到 application::iteration 模块。
//! Task iteration state delegates to application::iteration module.

pub use crate::application::iteration::Iteration as TaskIteration;
pub use crate::application::iteration::IterationSnapshot as TaskIterationSnapshot;
