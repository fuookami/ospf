//! 批次迭代模型 / Bunch iteration models
//!
//! 批次迭代状态委托到 application::iteration 模块。
//! Bunch iteration state delegates to application::iteration module.

pub use crate::application::iteration::Iteration as BunchIteration;
pub use crate::application::iteration::IterationSnapshot as BunchIterationSnapshot;
