//! Token 系统模块 / Token System Module
//!
//! 本模块提供变量在求解器中的表示和管理。
//! This module provides variable representation and management in solvers.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`IntoValue<V>`] - 值类型转换 trait / Value type conversion trait
//! - `Variable<V>` - 变量抽象 trait / Variable abstraction trait
//! - [`AnyVariable<V>`] - 任意类型变量包装器 / Any-type variable wrapper
//! - [`Token<V>`] - 变量在求解器中的表示 / Variable representation in solver
//! - [`TokenList<V>`] - Token 列表 trait / Token list trait
//! - [`TokenTable<V>`] - Token 表管理 trait / Token table management trait

#[allow(clippy::module_inception)]
pub mod token;
pub mod token_list;
pub mod token_table;

pub use token::{
    AnyVariable, AnyVariableF64, IntoValue, Token, TokenF64, TokenSnapshot,
    VariableData as TokenVariableData,
};
pub use token_list::*;
pub use token_table::*;
