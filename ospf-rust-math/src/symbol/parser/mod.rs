//! 多项式解析器 / Polynomial Parser
//!
//! 提供多项式和不等式的字符串解析功能。
//! Provides string parsing functionality for polynomials and inequalities.
//!
//! # 设计说明 / Design Notes
//!
//! 解析器分为三个阶段：
//! The parser is divided into three stages:
//! 1. 词法分析 (Lexer): 将字符串转换为 token 序列
//!    Lexical analysis: Convert string to token sequence
//! 2. 语法分析 (Parser): 将 token 序列转换为表达式树
//!    Syntax analysis: Convert token sequence to expression tree
//! 3. 语义分析: 将表达式树转换为多项式类型
//!    Semantic analysis: Convert expression tree to polynomial types

mod error;
mod expr;
mod lexer;
mod parser;

pub use error::{ParseError, ParseResult};
pub use expr::{Expr, ExprKind};
pub use lexer::{Lexer, Token};
pub use parser::Parser;

use std::fmt::Debug;
use std::ops::{Add, Mul, Neg, Sub};

use crate::symbol::inequality::{LinearInequality, QuadraticInequality};
use crate::symbol::{Linear, Quadratic};

/// 解析线性多项式 / Parse linear polynomial
///
/// 支持格式：`2*x + 3*y - 1`, `x`, `2*x + y`
/// Supported formats: `2*x + 3*y - 1`, `x`, `2*x + y`
///
/// # 示例 / Examples
///
/// ```ignore
/// use ospf_rust_math::symbol::parser::parse_linear;
///
/// let linear = parse_linear::<f64>("2*x + 3*y - 1")?;
/// ```
pub fn parse_linear<T>(s: &str) -> ParseResult<Linear<T>>
where
    T: std::str::FromStr
        + Clone
        + num_traits::Zero
        + num_traits::One
        + Neg<Output = T>
        + Add<Output = T>
        + Sub<Output = T>
        + Debug
        + PartialEq,
{
    let mut parser = Parser::new(s);
    parser.parse_linear()
}

/// 解析二次多项式 / Parse quadratic polynomial
///
/// 支持格式：`x^2 + 2*x*y + 1`, `x*x + y`, `x² + 2*x + 1`
/// Supported formats: `x^2 + 2*x*y + 1`, `x*x + y`, `x² + 2*x + 1`
///
/// # 示例 / Examples
///
/// ```ignore
/// use ospf_rust_math::symbol::parser::parse_quadratic;
///
/// let quadratic = parse_quadratic::<f64>("x^2 + 2*x*y + 1")?;
/// ```
pub fn parse_quadratic<T>(s: &str) -> ParseResult<Quadratic<T>>
where
    T: std::str::FromStr
        + Clone
        + num_traits::Zero
        + num_traits::One
        + Neg<Output = T>
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Debug
        + PartialEq,
{
    let mut parser = Parser::new(s);
    parser.parse_quadratic()
}

/// 解析线性不等式 / Parse linear inequality
///
/// 支持格式：`2*x + 3*y <= 5`, `x + y >= 1`
/// Supported formats: `2*x + 3*y <= 5`, `x + y >= 1`
///
/// # 示例 / Examples
///
/// ```ignore
/// use ospf_rust_math::symbol::parser::parse_linear_inequality;
///
/// let ineq = parse_linear_inequality::<f64>("2*x + 3*y <= 5")?;
/// ```
pub fn parse_linear_inequality<T>(s: &str) -> ParseResult<LinearInequality<T>>
where
    T: std::str::FromStr
        + Clone
        + num_traits::Zero
        + num_traits::One
        + Neg<Output = T>
        + Add<Output = T>
        + Sub<Output = T>
        + Debug
        + PartialEq,
{
    let mut parser = Parser::new(s);
    parser.parse_linear_inequality()
}

/// 解析二次不等式 / Parse quadratic inequality
///
/// 支持格式：`x^2 + y^2 <= 10`, `x*x + y*y >= 1`
/// Supported formats: `x^2 + y^2 <= 10`, `x*x + y*y >= 1`
pub fn parse_quadratic_inequality<T>(s: &str) -> ParseResult<QuadraticInequality<T>>
where
    T: std::str::FromStr
        + Clone
        + num_traits::Zero
        + num_traits::One
        + Neg<Output = T>
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Debug
        + PartialEq,
{
    let mut parser = Parser::new(s);
    parser.parse_quadratic_inequality()
}
