//! 单项式模块
//! Monomial module
//!
//! 本模块定义三种单项式类型：
//! This module defines three monomial types:
//!
//! - [`LinearMonomial`] - 线性单项式：c * S
//! - [`QuadraticMonomial`] - 二次单项式：c * S1 * S2 或 c * S1
//! - [`CanonicalMonomial`] - 标准单项式：c * S1^n1 * S2^n2 * ...

pub mod canonical;
pub mod linear;
pub mod quadratic;

pub use canonical::*;
pub use linear::*;
pub use quadratic::*;
