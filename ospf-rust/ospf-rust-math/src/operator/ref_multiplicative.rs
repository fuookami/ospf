//! 乘法引用操作 traits
//! Multiplicative reference operation traits
//!
//! 这些 traits 为类型提供乘法相关的引用操作约束，
//! These traits provide multiplicative reference operation constraints for types,
//! 允许在不消耗所有权的情况下进行运算。
//! allowing operations without consuming ownership.

use std::ops::{Div, Mul};

// ============================================================================
// MulRef - 引用相乘 / Reference Multiplication
// ============================================================================

/// MulRef - 支持引用相乘的类型
/// MulRef - Types that support reference multiplication
///
/// 表示 `&T * &T -> T` 操作。
/// Represents the `&T * &T -> T` operation.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::operator::MulRef;
///
/// fn mul_refs<T: MulRef>(a: &T, b: &T) -> T {
///     T::mul_ref(a, b)
/// }
///
/// let result = mul_refs(&2i32, &3i32);
/// assert_eq!(result, 6);
/// ```
pub trait MulRef {
    /// 引用相乘 / Multiply by reference
    fn mul_ref(a: &Self, b: &Self) -> Self;
}

// ============================================================================
// DivRef - 引用相除 / Reference Division
// ============================================================================

/// DivRef - 支持引用相除的类型
/// DivRef - Types that support reference division
///
/// 表示 `&T / &T -> T` 操作。
/// Represents the `&T / &T -> T` operation.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::operator::DivRef;
///
/// fn div_refs<T: DivRef>(a: &T, b: &T) -> T {
///     T::div_ref(a, b)
/// }
///
/// let result = div_refs(&6.0f64, &2.0f64);
/// assert!((result - 3.0).abs() < 1e-10);
/// ```
pub trait DivRef: Sized {
    /// 引用相除 / Divide by reference
    fn div_ref(a: &Self, b: &Self) -> Self;
}

// ============================================================================
// 自动实现 - Auto Implementations
// ============================================================================

/// 为满足约束的类型自动实现 MulRef
/// Auto-implement MulRef for types satisfying constraints
impl<T> MulRef for T
where
    for<'a> &'a T: Mul<Output = T>,
{
    fn mul_ref(a: &Self, b: &Self) -> Self {
        a * b
    }
}

/// 为满足约束的类型自动实现 DivRef
/// Auto-implement DivRef for types satisfying constraints
impl<T> DivRef for T
where
    for<'a> &'a T: Div<Output = T>,
{
    fn div_ref(a: &Self, b: &Self) -> Self {
        a / b
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mul_ref() {
        let a = 5i32;
        let b = 3i32;

        // 测试引用相乘 / Test reference multiplication
        let result = i32::mul_ref(&a, &b);
        assert_eq!(result, 15);
    }

    #[test]
    fn test_div_ref() {
        let a = 6f64;
        let b = 2f64;

        // 测试引用相除 / Test reference division
        let result = f64::div_ref(&a, &b);
        assert!((result - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_f64_mul_ref() {
        let a = 2.5f64;
        let b = 4.0f64;

        let result = f64::mul_ref(&a, &b);
        assert!((result - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_i32_div_ref() {
        let a = 10i32;
        let b = 3i32;

        let result = i32::div_ref(&a, &b);
        assert_eq!(result, 3); // 整数除法 / Integer division
    }
}
