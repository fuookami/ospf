//! 加法引用操作 traits
//! Additive reference operation traits
//!
//! 这些 traits 为类型提供加法相关的引用操作约束，
//! These traits provide additive reference operation constraints for types,
//! 允许在不消耗所有权的情况下进行运算。
//! allowing operations without consuming ownership.

use std::ops::{Add, Neg, Sub};

// ============================================================================
// AddRef - 引用相加 / Reference Addition
// ============================================================================

/// AddRef - 支持引用相加的类型
/// AddRef - Types that support reference addition
///
/// 表示 `&T + &T -> T` 操作。
/// Represents the `&T + &T -> T` operation.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::operator::AddRef;
///
/// fn add_refs<T: AddRef>(a: &T, b: &T) -> T {
///     T::add_ref(a, b)
/// }
///
/// let result = add_refs(&1i32, &2i32);
/// assert_eq!(result, 3);
/// ```
pub trait AddRef {
    /// 引用相加 / Add by reference
    fn add_ref(a: &Self, b: &Self) -> Self;
}

// ============================================================================
// SubRef - 引用相减 / Reference Subtraction
// ============================================================================

/// SubRef - 支持引用相减的类型
/// SubRef - Types that support reference subtraction
///
/// 表示 `&T - &T -> T` 操作。
/// Represents the `&T - &T -> T` operation.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::operator::SubRef;
///
/// fn sub_refs<T: SubRef>(a: &T, b: &T) -> T {
///     T::sub_ref(a, b)
/// }
///
/// let result = sub_refs(&5i32, &3i32);
/// assert_eq!(result, 2);
/// ```
pub trait SubRef: Sized {
    /// 引用相减 / Subtract by reference
    fn sub_ref(a: &Self, b: &Self) -> Self;
}

// ============================================================================
// NegRef - 引用取负 / Reference Negation
// ============================================================================

/// NegRef - 支持引用取负的类型
/// NegRef - Types that support reference negation
///
/// 表示 `-&T -> T` 操作。
/// Represents the `-&T -> T` operation.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::operator::NegRef;
///
/// fn neg_ref<T: NegRef>(a: &T) -> T {
///     T::neg_ref(a)
/// }
///
/// let result = neg_ref(&5i32);
/// assert_eq!(result, -5);
/// ```
pub trait NegRef: Sized {
    /// 引用取负 / Negate by reference
    fn neg_ref(a: &Self) -> Self;
}

// ============================================================================
// 自动实现 - Auto Implementations
// ============================================================================

/// 为满足约束的类型自动实现 AddRef
/// Auto-implement AddRef for types satisfying constraints
impl<T> AddRef for T
where
    for<'a> &'a T: Add<Output = T>,
{
    fn add_ref(a: &Self, b: &Self) -> Self {
        a + b
    }
}

/// 为满足约束的类型自动实现 SubRef
/// Auto-implement SubRef for types satisfying constraints
impl<T> SubRef for T
where
    for<'a> &'a T: Sub<Output = T>,
{
    fn sub_ref(a: &Self, b: &Self) -> Self {
        a - b
    }
}

/// 为满足约束的类型自动实现 NegRef
/// Auto-implement NegRef for types satisfying constraints
impl<T> NegRef for T
where
    for<'a> &'a T: Neg<Output = T>,
{
    fn neg_ref(a: &Self) -> Self {
        -a
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_ref() {
        let a = 5i32;
        let b = 3i32;

        // 测试引用相加 / Test reference addition
        let result = i32::add_ref(&a, &b);
        assert_eq!(result, 8);
    }

    #[test]
    fn test_sub_ref() {
        let a = 5i32;
        let b = 3i32;

        // 测试引用相减 / Test reference subtraction
        let result = i32::sub_ref(&a, &b);
        assert_eq!(result, 2);
    }

    #[test]
    fn test_neg_ref() {
        let a = 5i32;

        // 测试引用取负 / Test reference negation
        let result = i32::neg_ref(&a);
        assert_eq!(result, -5);
    }

    #[test]
    fn test_f64_add_ref() {
        let a = 5.5f64;
        let b = 3.5f64;

        let result = f64::add_ref(&a, &b);
        assert!((result - 9.0).abs() < 1e-10);
    }
}
