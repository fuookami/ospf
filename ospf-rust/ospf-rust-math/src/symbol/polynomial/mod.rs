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

/// Kotlin 风格线性多项式命名兼容别名。
/// Kotlin-style compatibility alias for linear polynomial naming.
pub type LinearPolynomial<T> = Linear<T>;

/// Kotlin 风格二次多项式命名兼容别名。
/// Kotlin-style compatibility alias for quadratic polynomial naming.
pub type QuadraticPolynomial<T> = Quadratic<T>;

/// Kotlin 风格标准多项式命名兼容别名。
/// Kotlin-style compatibility alias for canonical polynomial naming.
pub type CanonicalPolynomial<T, E = i32> = Canonical<T, E>;
