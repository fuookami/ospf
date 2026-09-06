//! 回调模型层
//! Callback Model Layer
//!
//! 本模块提供求解器回调功能的基础接口和实现。
//! This module provides base interfaces and implementations for solver callback functionality.

pub mod abstract_callback_model;
pub mod callback_model_trait;
pub mod multi_objective_model;
pub mod solution;

pub use abstract_callback_model::*;
pub use callback_model_trait::*;
pub use multi_objective_model::*;
pub use solution::*;
