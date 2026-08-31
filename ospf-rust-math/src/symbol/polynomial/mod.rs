//! 多项式模块
//! Polynomial module
//!
//! 本模块定义三种多项式类型：
//! This module defines three polynomial types:
//!
//! - [`Linear`] - 线性多项式：Σ cᵢSᵢ + b
//! - [`Quadratic`] - 二次多项式：Σ cᵢⱼSᵢSⱼ + Σ dᵢSᵢ + e
//! - [`Canonical`] - 标准多项式：Σ cᵢ * ∏ Sⱼ^nⱼ + d

pub mod canonical;
pub mod linear;
pub mod quadratic;

pub use canonical::*;
pub use linear::*;
pub use quadratic::*;
