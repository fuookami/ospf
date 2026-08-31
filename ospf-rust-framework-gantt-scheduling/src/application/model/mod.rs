//! 应用层模型 / Application models
//!
//! 承接迭代快照、任务迭代和批次迭代等应用编排状态。
//! Hosts application orchestration states such as iteration snapshots,
//! task iterations, and bunch iterations.

pub mod bunch;
pub mod task;

pub use crate::application::iteration::{Iteration, IterationSnapshot};
