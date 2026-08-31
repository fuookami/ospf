//! 类型转换 Trait 定义
//! Type conversion trait definitions
//!
//! 本模块提供多项式类型转换的 trait 定义，包括必然转换和尝试转换。
//! This module provides trait definitions for polynomial type conversion,
//! including infallible and fallible conversions.

use crate::operator::Exponent;
use crate::symbol::{Canonical, Linear, Quadratic};

// ============================================================================
// 类型转换 Traits / Type Conversion Traits
// ============================================================================

/// 转换为线性多项式 / Convert to linear polynomial
///
/// 将当前类型转换为线性多项式形式。
/// Converts the current type to linear polynomial form.
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 系数类型 / Coefficient type
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::operation::ToLinear;
/// use ospf_rust_math::symbol::{LinearMonomial, Linear};
/// use ospf_rust_math::symbols_test;
///
/// symbols_test!(x);
///
/// let linear_monomial = LinearMonomial::new(2.0, x);
/// let linear: Linear<f64> = linear_monomial.to_linear();
/// assert_eq!(linear.monomials.len(), 1);
/// ```
pub trait ToLinear<T>: Sized {
    /// 转换为线性多项式
    /// Convert to linear polynomial
    fn to_linear(self) -> Linear<T>;
}

/// 转换为二次多项式 / Convert to quadratic polynomial
///
/// 将当前类型转换为二次多项式形式。
/// Converts the current type to quadratic polynomial form.
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 系数类型 / Coefficient type
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::operation::ToQuadratic;
/// use ospf_rust_math::symbol::{Linear, LinearMonomial, Quadratic};
/// use ospf_rust_math::symbols_test;
///
/// symbols_test!(x);
///
/// let linear = Linear::new(vec![LinearMonomial::new(2.0, x)], 1.0);
/// let quadratic: Quadratic<f64> = linear.to_quadratic();
/// assert_eq!(quadratic.monomials.len(), 1);
/// ```
pub trait ToQuadratic<T>: Sized {
    /// 转换为二次多项式
    /// Convert to quadratic polynomial
    fn to_quadratic(self) -> Quadratic<T>;
}

/// 转换为标准多项式 / Convert to canonical polynomial
///
/// 将当前类型转换为标准多项式形式。
/// Converts the current type to canonical polynomial form.
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 系数类型 / Coefficient type
/// - `E`: 指数类型，默认为 `i32` / Exponent type, defaults to `i32`
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::operation::ToCanonical;
/// use ospf_rust_math::symbol::{Quadratic, QuadraticMonomial, Canonical};
/// use ospf_rust_math::symbols_test;
///
/// symbols_test!(x, y);
///
/// let quadratic = Quadratic::new(vec![
///     QuadraticMonomial::quadratic(1.0, x.clone(), x.clone()),
///     QuadraticMonomial::quadratic(2.0, x.clone(), y.clone()),
/// ], 0.0);
/// let canonical: Canonical<f64, i32> = quadratic.to_canonical();
/// assert_eq!(canonical.monomials.len(), 2);
/// ```
pub trait ToCanonical<T, E: Exponent = i32>: Sized {
    /// 转换为标准多项式
    /// Convert to canonical polynomial
    fn to_canonical(self) -> Canonical<T, E>;
}

// ============================================================================
// 尝试转换错误类型 / Fallible Conversion Error Types
// ============================================================================

/// 尝试转换为线性多项式的错误类型
/// Error type for trying to convert to linear polynomial
#[derive(Debug, Clone, PartialEq)]
pub enum TryToLinearError {
    /// 包含二次或更高次项 / Contains quadratic or higher order terms
    HasHigherOrderTerms,
    /// 包含多个符号的单项式 / Contains monomial with multiple symbols
    HasMultipleSymbols,
}

impl std::fmt::Display for TryToLinearError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TryToLinearError::HasHigherOrderTerms => {
                write!(f, "Cannot convert to linear: contains higher order terms")
            }
            TryToLinearError::HasMultipleSymbols => {
                write!(f, "Cannot convert to linear: monomial has multiple symbols")
            }
        }
    }
}

impl std::error::Error for TryToLinearError {}

/// 尝试转换为二次多项式的错误类型
/// Error type for trying to convert to quadratic polynomial
#[derive(Debug, Clone, PartialEq)]
pub enum TryToQuadraticError {
    /// 包含三次或更高次项 / Contains cubic or higher order terms
    HasHigherOrderTerms,
    /// 单项式次数超过 2 / Monomial degree exceeds 2
    MonomialDegreeTooHigh,
}

impl std::fmt::Display for TryToQuadraticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TryToQuadraticError::HasHigherOrderTerms => {
                write!(f, "Cannot convert to quadratic: contains higher order terms")
            }
            TryToQuadraticError::MonomialDegreeTooHigh => {
                write!(f, "Cannot convert to quadratic: monomial degree exceeds 2")
            }
        }
    }
}

impl std::error::Error for TryToQuadraticError {}

/// 尝试转换为标准多项式的错误类型
/// Error type for trying to convert to canonical polynomial
#[derive(Debug, Clone, PartialEq)]
pub enum TryToCanonicalError {
    /// 不支持的转换 / Unsupported conversion
    Unsupported,
}

impl std::fmt::Display for TryToCanonicalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cannot convert to canonical: unsupported conversion")
    }
}

impl std::error::Error for TryToCanonicalError {}

// ============================================================================
// 尝试转换 Traits / Fallible Conversion Traits
// ============================================================================

/// 尝试转换为线性多项式 / Try to convert to linear polynomial
///
/// 尝试将当前类型转换为线性多项式形式。
/// 如果当前类型包含二次或更高次项，则转换失败。
/// Attempts to convert the current type to linear polynomial form.
/// Fails if the current type contains quadratic or higher order terms.
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 系数类型 / Coefficient type
///
/// # 返回值 / Returns
///
/// - `Ok(Linear<T>)`: 转换成功 / Conversion succeeded
/// - `Err(TryToLinearError)`: 转换失败 / Conversion failed
pub trait TryToLinear<T>: Sized {
    /// 尝试转换为线性多项式
    /// Try to convert to linear polynomial
    fn try_to_linear(self) -> Result<Linear<T>, TryToLinearError>;
}

/// 尝试转换为二次多项式 / Try to convert to quadratic polynomial
///
/// 尝试将当前类型转换为二次多项式形式。
/// 如果当前类型包含三次或更高次项，则转换失败。
/// Attempts to convert the current type to quadratic polynomial form.
/// Fails if the current type contains cubic or higher order terms.
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 系数类型 / Coefficient type
///
/// # 返回值 / Returns
///
/// - `Ok(Quadratic<T>)`: 转换成功 / Conversion succeeded
/// - `Err(TryToQuadraticError)`: 转换失败 / Conversion failed
pub trait TryToQuadratic<T>: Sized {
    /// 尝试转换为二次多项式
    /// Try to convert to quadratic polynomial
    fn try_to_quadratic(self) -> Result<Quadratic<T>, TryToQuadraticError>;
}

/// 尝试转换为标准多项式 / Try to convert to canonical polynomial
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 系数类型 / Coefficient type
/// - `E`: 指数类型，默认为 `i32` / Exponent type, defaults to `i32`
pub trait TryToCanonical<T, E: Exponent = i32>: Sized {
    /// 尝试转换为标准多项式
    /// Try to convert to canonical polynomial
    fn try_to_canonical(self) -> Result<Canonical<T, E>, TryToCanonicalError>;
}

// ============================================================================
// 宏：为实现了 To* 的类型提供 TryTo* 实现
// Macro: provide TryTo* implementations for types implementing To*
// ============================================================================

/// 为实现了 ToLinear 的类型实现 TryToLinear（总是成功）
/// Implement TryToLinear for types implementing ToLinear (always succeeds)
#[macro_export]
macro_rules! impl_try_to_linear_from_to_linear {
    ($ty:ty) => {
        impl<T> $crate::symbol::operation::TryToLinear<T> for $ty
        where
            $ty: $crate::symbol::operation::ToLinear<T>,
        {
            fn try_to_linear(self) -> Result<$crate::symbol::Linear<T>, $crate::symbol::operation::TryToLinearError> {
                Ok(self.to_linear())
            }
        }
    };
}

/// 为实现了 ToQuadratic 的类型实现 TryToQuadratic（总是成功）
/// Implement TryToQuadratic for types implementing ToQuadratic (always succeeds)
#[macro_export]
macro_rules! impl_try_to_quadratic_from_to_quadratic {
    ($ty:ty) => {
        impl<T> $crate::symbol::operation::TryToQuadratic<T> for $ty
        where
            $ty: $crate::symbol::operation::ToQuadratic<T>,
        {
            fn try_to_quadratic(self) -> Result<$crate::symbol::Quadratic<T>, $crate::symbol::operation::TryToQuadraticError> {
                Ok(self.to_quadratic())
            }
        }
    };
}

/// 为实现了 ToCanonical 的类型实现 TryToCanonical（总是成功）
/// Implement TryToCanonical for types implementing ToCanonical (always succeeds)
#[macro_export]
macro_rules! impl_try_to_canonical_from_to_canonical {
    ($ty:ty) => {
        impl<T, E: $crate::operator::Exponent> $crate::symbol::operation::TryToCanonical<T, E> for $ty
        where
            $ty: $crate::symbol::operation::ToCanonical<T, E>,
        {
            fn try_to_canonical(self) -> Result<$crate::symbol::Canonical<T, E>, $crate::symbol::operation::TryToCanonicalError> {
                Ok(self.to_canonical())
            }
        }
    };
}
