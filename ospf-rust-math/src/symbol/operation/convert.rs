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

/// Kotlin 风格线性多项式转换命名兼容 Trait。
/// Kotlin-style compatibility trait for linear polynomial conversion naming.
pub trait ToLinearPolynomial<T>: ToLinear<T> {}

impl<T, P> ToLinearPolynomial<T> for P where P: ToLinear<T> {}

/// Kotlin 风格二次多项式转换命名兼容 Trait。
/// Kotlin-style compatibility trait for quadratic polynomial conversion naming.
pub trait ToQuadraticPolynomial<T>: ToQuadratic<T> {}

impl<T, P> ToQuadraticPolynomial<T> for P where P: ToQuadratic<T> {}

/// Kotlin 风格标准多项式转换命名兼容 Trait。
/// Kotlin-style compatibility trait for canonical polynomial conversion naming.
pub trait ToCanonicalPolynomial<T, E: Exponent = i32>: ToCanonical<T, E> {}

impl<T, E, P> ToCanonicalPolynomial<T, E> for P
where
    E: Exponent,
    P: ToCanonical<T, E>,
{
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
                write!(
                    f,
                    "Cannot convert to quadratic: contains higher order terms"
                )
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

/// Kotlin 风格线性多项式尝试转换命名兼容 Trait。
/// Kotlin-style compatibility trait for fallible linear polynomial conversion naming.
pub trait TryToLinearPolynomial<T>: TryToLinear<T> {}

impl<T, P> TryToLinearPolynomial<T> for P where P: TryToLinear<T> {}

/// Kotlin 风格二次多项式尝试转换命名兼容 Trait。
/// Kotlin-style compatibility trait for fallible quadratic polynomial conversion naming.
pub trait TryToQuadraticPolynomial<T>: TryToQuadratic<T> {}

impl<T, P> TryToQuadraticPolynomial<T> for P where P: TryToQuadratic<T> {}

/// Kotlin 风格标准多项式尝试转换命名兼容 Trait。
/// Kotlin-style compatibility trait for fallible canonical polynomial conversion naming.
pub trait TryToCanonicalPolynomial<T, E: Exponent = i32>: TryToCanonical<T, E> {}

impl<T, E, P> TryToCanonicalPolynomial<T, E> for P
where
    E: Exponent,
    P: TryToCanonical<T, E>,
{
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
            fn try_to_linear(
                self,
            ) -> Result<$crate::symbol::Linear<T>, $crate::symbol::operation::TryToLinearError>
            {
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
        impl<T, E: $crate::operator::Exponent> $crate::symbol::operation::TryToCanonical<T, E>
            for $ty
        where
            $ty: $crate::symbol::operation::ToCanonical<T, E>,
        {
            fn try_to_canonical(
                self,
            ) -> Result<
                $crate::symbol::Canonical<T, E>,
                $crate::symbol::operation::TryToCanonicalError,
            > {
                Ok(self.to_canonical())
            }
        }
    };
}

// ============================================================================
// 测试 / Tests
// ============================================================================

/// 本模块声明转换 trait，实现分散在 `polynomial/{linear,quadratic,canonical}.rs`。
/// 这里通过真实实现校验转换契约与错误语义。
///
/// This module declares the conversion traits; implementations live in
/// `polynomial/{linear,quadratic,canonical}.rs`. The conversion contract and the error
/// semantics are verified here through those real implementations.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::test_utils::SimpleSymbol;
    use crate::symbol::{Linear, LinearMonomial, OwnedSymbol, Quadratic, QuadraticMonomial};

    /// 构造一个具名测试符号 / Build a named test symbol.
    fn sym(id: usize, name: &str) -> OwnedSymbol {
        OwnedSymbol::new(SimpleSymbol::with_id(id, name))
    }

    #[test]
    fn try_to_linear_error_messages_are_specific() {
        // 两个变体必须有可区分的、面向人的消息，便于定位转换失败原因。
        // The two variants must carry distinguishable, human-readable messages so the
        // cause of a failed conversion is identifiable.
        let higher = TryToLinearError::HasHigherOrderTerms.to_string();
        let multiple = TryToLinearError::HasMultipleSymbols.to_string();

        assert!(higher.contains("higher order"), "实际: {higher}");
        assert!(multiple.contains("multiple symbols"), "实际: {multiple}");
        assert_ne!(higher, multiple, "两个变体的消息必须不同");
    }

    #[test]
    fn try_to_quadratic_error_messages_are_specific() {
        // 同上，二次转换的两类错误必须可区分。
        // Likewise, the two quadratic-conversion errors must be distinguishable.
        let higher = TryToQuadraticError::HasHigherOrderTerms.to_string();
        let degree = TryToQuadraticError::MonomialDegreeTooHigh.to_string();

        assert!(higher.contains("higher order"), "实际: {higher}");
        assert!(degree.contains("degree exceeds 2"), "实际: {degree}");
        assert_ne!(higher, degree);
    }

    #[test]
    fn try_to_canonical_error_message_is_descriptive() {
        // 规范化转换目前只有一种失败原因，消息必须说明是"不支持"。
        // Canonical conversion currently has a single failure cause; the message must say
        // the conversion is unsupported.
        let message = TryToCanonicalError::Unsupported.to_string();
        assert!(message.contains("unsupported"), "实际: {message}");
    }

    #[test]
    fn conversion_errors_are_usable_as_std_errors() {
        // 必须能作为 std::error::Error 使用，才能接入 `?` 与错误链。
        // They must be usable as std::error::Error so they compose with `?` and error chains.
        let linear: &dyn std::error::Error = &TryToLinearError::HasHigherOrderTerms;
        let quadratic: &dyn std::error::Error = &TryToQuadraticError::MonomialDegreeTooHigh;
        let canonical: &dyn std::error::Error = &TryToCanonicalError::Unsupported;

        assert!(!linear.to_string().is_empty());
        assert!(!quadratic.to_string().is_empty());
        assert!(!canonical.to_string().is_empty());
        assert!(linear.source().is_none(), "当前没有下层错误来源");
    }

    #[test]
    fn conversion_errors_compare_by_variant() {
        // 错误类型派生 PartialEq，同变体相等、异变体不等。
        // The error types derive PartialEq: same variant equal, different variant unequal.
        assert_eq!(
            TryToLinearError::HasHigherOrderTerms,
            TryToLinearError::HasHigherOrderTerms
        );
        assert_ne!(
            TryToLinearError::HasHigherOrderTerms,
            TryToLinearError::HasMultipleSymbols
        );
        assert_ne!(
            TryToQuadraticError::HasHigherOrderTerms,
            TryToQuadraticError::MonomialDegreeTooHigh
        );
    }

    #[test]
    fn purely_linear_quadratic_converts_to_linear() {
        // 只含线性项的 Quadratic 必须能转成 Linear，且系数与常数项原样保留。
        // A Quadratic holding only linear terms must convert to Linear, preserving
        // coefficients and the constant term.
        let x = sym(1, "x");
        let y = sym(2, "y");
        let quadratic = Quadratic::new(
            vec![
                QuadraticMonomial::linear(3.0_f64, x.clone()),
                QuadraticMonomial::linear(-2.0_f64, y.clone()),
            ],
            7.0,
        );

        let linear: Linear<f64> = quadratic
            .try_to_linear()
            .expect("纯线性项必须可以转换 / purely linear terms must convert");

        assert_eq!(linear.constant, 7.0, "常数项必须保留");
        assert_eq!(linear.monomials.len(), 2);
        let terms: Vec<(String, f64)> = linear
            .monomials
            .iter()
            .map(|m| (m.symbol.to_string(), m.coefficient))
            .collect();
        assert!(terms.contains(&("x".to_string(), 3.0)), "实际 {terms:?}");
        assert!(terms.contains(&("y".to_string(), -2.0)), "实际 {terms:?}");
    }

    #[test]
    fn genuinely_quadratic_cannot_convert_to_linear() {
        // 含 x² 或 xy 的二次式必须拒绝转线性，并给出 HasHigherOrderTerms。
        // A form containing x² or xy must refuse linear conversion with HasHigherOrderTerms.
        let x = sym(1, "x");
        let y = sym(2, "y");

        let square = Quadratic::new(
            vec![QuadraticMonomial::quadratic(1.0_f64, x.clone(), x.clone())],
            0.0,
        );
        assert_eq!(
            square.try_to_linear().unwrap_err(),
            TryToLinearError::HasHigherOrderTerms,
            "x² 不得转成线性"
        );

        let mixed = Quadratic::new(
            vec![QuadraticMonomial::quadratic(1.0_f64, x, y)],
            0.0,
        );
        assert_eq!(
            mixed.try_to_linear().unwrap_err(),
            TryToLinearError::HasHigherOrderTerms,
            "xy 不得转成线性"
        );
    }

    #[test]
    fn reference_conversion_matches_owned_conversion() {
        // &Quadratic 与 Quadratic 的转换结果必须一致（两者各自实现了 trait）。
        // The &Quadratic and Quadratic conversions must agree; each has its own impl.
        let x = sym(1, "x");
        let quadratic = Quadratic::new(
            vec![QuadraticMonomial::linear(4.0_f64, x.clone())],
            1.5,
        );

        let owned: Linear<f64> = quadratic.clone().try_to_linear().expect("owned 转换成功");
        let borrowed: Linear<f64> = (&quadratic).try_to_linear().expect("borrowed 转换成功");

        assert_eq!(owned.constant, borrowed.constant);
        assert_eq!(owned.monomials.len(), borrowed.monomials.len());
        let owned_terms: Vec<(String, f64)> = owned
            .monomials
            .iter()
            .map(|m| (m.symbol.to_string(), m.coefficient))
            .collect();
        let borrowed_terms: Vec<(String, f64)> = borrowed
            .monomials
            .iter()
            .map(|m| (m.symbol.to_string(), m.coefficient))
            .collect();
        assert_eq!(owned_terms, borrowed_terms);
    }

    #[test]
    fn linear_monomial_to_canonical_keeps_the_symbol() {
        // 单项式转换到 Canonical 后必须保留符号身份（类型级转换的核心不变量）。
        // Converting a monomial to Canonical must preserve symbol identity — the core
        // invariant of the type-level conversions.
        use crate::symbol::operation::ToCanonical;

        let x = sym(1, "x");
        let monomial = LinearMonomial::new(3.0_f64, x.clone());
        let canonical: crate::symbol::Canonical<f64, i32> = monomial.to_canonical();

        assert_eq!(canonical.constant, 0.0, "单项式转换不引入常数项");
        assert_eq!(canonical.monomials.len(), 1);
        let canonical_monomial = &canonical.monomials[0];
        assert_eq!(canonical_monomial.coefficient, 3.0);
        let symbols: Vec<String> = canonical_monomial
            .powers
            .keys()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(symbols, vec!["x".to_string()], "符号身份必须保留");
    }
}
