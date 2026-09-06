//! 符号运算操作模块
//! Symbolic operation module
//!
//! 本模块提供多项式类型转换和运算功能。
//! This module provides polynomial type conversion and operation capabilities.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`ToLinear`] - 转换为线性多项式
//! - [`ToQuadratic`] - 转换为二次多项式
//! - [`ToCanonical`] - 转换为标准多项式
//! - [`Evaluate`] - 多项式求值
//! - [`EvaluateOrdered`] - 有序求值
//! - [`Differentiate`] - 符号微分
//! - [`SecondOrderDifferentiate`] - 二阶微分
//! - [`ToMatrixForm`] - 转换为矩阵形式
//! - [`LinearMatrixForm`] - 线性多项式矩阵形式
//! - [`QuadraticMatrixForm`] - 二次多项式矩阵形式
//! - [`ToLaTeX`] - LaTeX 输出
//! - [`LatexOptions`] - LaTeX 输出选项
//! - [`CompileEval`] - 编译求值
//! - [`CompileGradient`] - 编译梯度

// ============================================================================
// 模块导出 / Module Exports
// ============================================================================

pub mod combine;
pub mod compile;
mod convert;
mod differentiate;
mod evaluate;
mod factorization;
mod integrate;
mod latex;
mod matrix_form;
mod normalize;
mod value_provider;

pub use compile::{CompileEval, CompileGradient};
pub use convert::*;
pub use differentiate::*;
pub use evaluate::*;
pub use factorization::*;
pub use integrate::*;
pub use latex::*;
pub use matrix_form::*;
pub use normalize::*;
pub use value_provider::*;
