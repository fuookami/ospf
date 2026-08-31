#![cfg_attr(feature = "nightly", feature(unboxed_closures, fn_traits))]

//! OSPF Rust Core - 运筹建模框架核心模块
//! OSPF Rust Core - Operations Research Modeling Framework Core Module
//!
//! 本模块实现一个运筹学建模框架，支持线性规划 (LP)、混合整数规划 (MIP)、
//! 二次规划 (QP) 等优化问题的建模与求解。
//!
//! This module implements an operations research modeling framework supporting
//! linear programming (LP), mixed integer programming (MIP), quadratic programming (QP),
//! and other optimization problems.
//!
//! # 核心模块 / Core Modules
//!
//! - [`variable`] - 变量系统（Variable System）
//! - [`token`] - Token 系统（Token System）
//! - [`symbol`] - 中间符号系统（Intermediate Symbol System）
//! - [`model`] - 模型系统（Model System）
//!   - [`model::flatten`] - 表达式平展系统（Expression Flatten System）
//!   - [`model::mechanism`] - 机理模型系统，包含约束（Mechanism Model System, includes constraints）
//!   - [`model::intermediate`] - 中间模型层（Intermediate Model Layer）
//!   - [`model::callback`] - 回调模型层（Callback Model Layer）
//! - [`solver`] - 求解器接口（Solver Interface）

pub mod error;
pub mod model;
pub mod solver;
pub mod symbol;
pub mod token;
pub mod variable;

pub use error::*;
pub use model::*;
pub use solver::*;
pub use symbol::*;
pub use token::*;

// 重新导出常用类型
// Re-export commonly used types
pub use variable::*;
