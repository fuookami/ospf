//! 数学运算符 traits
//! Mathematical operator traits
//!
//! 本模块提供各类数学运算的 trait 定义，包括基础运算、指数对数、幂运算、
//! 三角函数、引用运算和容差比较等。
//! This module provides trait definitions for various mathematical operations,
//! including basic operations, exponentials/logarithms, power operations,
//! trigonometric functions, reference operations, and tolerance comparisons.
//!
//! # 基础运算 / Basic Operations
//!
//! - [`Abs`] / [`AbsRef`] - 绝对值 / Absolute value
//! - [`Reciprocal`] / [`ReciprocalRef`] - 倒数 / Reciprocal
//! - [`Contains`] - 包含判断 / Contains check
//!
//! # 指数与对数 / Exponential and Logarithm
//!
//! - [`Exp`] / [`ExpWithPrecision`] - 自然指数 / Natural exponential
//! - [`Log`] / [`LogWithPrecision`] - 对数 / Logarithm
//! - [`Exponent`] - 指数标记 trait / Exponent marker trait
//!
//! # 幂运算 / Power Operations
//!
//! - [`Pow`] - 整数幂 / Integer power
//! - [`PowF`] / [`PowFWithPrecision`] - 浮点幂 / Floating-point power
//!
//! # 三角函数 / Trigonometric Functions
//!
//! - [`Trigonometry`] - 三角与双曲函数 / Trigonometric and hyperbolic functions
//!
//! # 引用运算 / Reference Arithmetic
//!
//! - [`AddRef`] / [`SubRef`] / [`NegRef`] - 加减取负引用运算 / Add/sub/neg reference ops
//! - [`MulRef`] / [`DivRef`] - 乘除引用运算 / Mul/div reference ops
//! - [`ZeroRef`] / [`OneRef`] / [`NegOneRef`] / [`Two`] - 常量引用 / Constant references
//!
//! # 容差比较 / Tolerance Comparison
//!
//! - [`Tolerance`] / [`TolerancedEq`] / [`TolerancedOrd`] - 容差相等与排序 / Toleranced equality and ordering

pub mod abs;
pub mod contains;

pub use abs::{Abs, AbsRef};
pub mod exp_log;
pub mod exponent;
pub mod one_zero_ref;
pub mod power;
pub mod reciprocal;
pub mod ref_additive;
pub mod ref_multiplicative;
pub mod tolerance;
pub mod trigonometry;

pub use contains::Contains;
pub use exp_log::{
    Exp, ExpWithPrecision, Log, LogWithPrecision, exp, exp_with_precision, lg, lg_with_precision,
    lg2, lg2_with_precision, ln, ln_with_precision, log, log_with_precision,
};
pub use exponent::Exponent;
pub use one_zero_ref::{NegOneRef, OneRef, Two, ZeroRef};
pub use power::{
    Pow, PowF, PowFWithPrecision, cbrt, cbrt_with_precision, cub, pow, powf, powf_with_precision,
    sqr, sqrt, sqrt_with_precision,
};
pub use reciprocal::{Reciprocal, ReciprocalRef};
pub use ref_additive::{AddRef, NegRef, SubRef};
pub use ref_multiplicative::{DivRef, MulRef};
pub use tolerance::{Tolerance, TolerancedEq, TolerancedOrd};
pub use trigonometry::{
    Trigonometry, acos, acosh, acot, acoth, acsc, acsch, asec, asech, asin, asinh, atan, atanh,
    cos, cosh, cot, coth, csc, csch, sec, sech, sin, sinh, tan, tanh,
};
