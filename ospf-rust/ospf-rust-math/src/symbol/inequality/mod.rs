//! 不等式模块
//! Inequality module
//!
//! 本模块提供不等式类型，支持优化问题中的约束表示。
//! This module provides inequality types for constraint representation in optimization problems.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`Comparison`] - 比较运算符
//! - [`LinearInequality`] - 线性不等式
//! - [`QuadraticInequality`] - 二次不等式
//! - [`CanonicalInequality`] - 标准不等式

pub mod canonical;
pub mod comparison;
pub mod linear;
pub mod quadratic;

pub use canonical::*;
pub use comparison::*;
pub use linear::*;
pub use quadratic::*;
