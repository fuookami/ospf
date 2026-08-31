//! 模型系统
//! Model System
//!
//! 本模块包含运筹学建模框架的所有模型相关组件。
//! This module contains all model-related components of the operations research modeling framework.
//!
//! # 子模块 / Sub-modules
//!
//! - [`flatten`] - 表达式平展系统
//! - [`mechanism`] - 机理模型系统（包含约束）
//! - [`intermediate`] - 中间模型层
//! - [`callback`] - 回调模型层

pub mod basic_model;
pub mod configuration;
pub mod meta_model;
pub mod object;
pub mod range_cache;
pub mod value_cache;

pub mod basic;
pub mod callback;
pub mod flatten;
pub mod intermediate;
pub mod mechanism;
pub mod status;

pub use basic_model::*;
pub use configuration::*;
pub use meta_model::*;
pub use object::*;
pub use range_cache::*;
pub use value_cache::*;

// 重新导出 flatten 模块
pub use flatten::*;

// 重新导出 mechanism 模块（包含约束和机理模型）
pub use mechanism::*;
pub use status::*;
