//! 代数性质 traits
//! Algebraic property traits

// ============================================================================
// Bounded Trait - 有界性
// ============================================================================

/// Bounded - 有界性 trait
/// Bounded - Boundedness trait
///
/// 表示类型是否有上下界。
/// Represents whether a type has upper and lower bounds.
///
/// 对于有界类型，可以获取其最小值和最大值。
/// For bounded types, you can get their minimum and maximum values.
///
/// # 示例 / Examples
/// ```
/// // 有界类型：i32 有最小值和最大值
/// // Bounded type: i32 has min and max values
/// assert!(i32::is_bounded());
/// assert_eq!(i32::min_value(), i32::MIN);
/// assert_eq!(i32::max_value(), i32::MAX);
///
/// // 无界类型：BigInt 没有界限
/// // Unbounded type: BigInt has no bounds
/// // （需要单独实现）
/// ```
pub trait Bounded {
    /// 判断类型是否有界
    /// Check if the type is bounded
    ///
    /// # 返回 / Returns
    /// 如果类型有上下界返回 `true`，否则返回 `false`
    /// Returns `true` if the type has bounds, `false` otherwise
    fn is_bounded() -> bool;

    /// 获取类型的最小值（如果有界）
    /// Get the minimum value of the type (if bounded)
    ///
    /// # 返回 / Returns
    /// 如果类型有界返回 `Some(min_value)`，否则返回 `None`
    /// Returns `Some(min_value)` if bounded, `None` otherwise
    fn min_value() -> Option<Self>
    where
        Self: Sized,
    {
        None
    }

    /// 获取类型的最大值（如果有界）
    /// Get the maximum value of the type (if bounded)
    ///
    /// # 返回 / Returns
    /// 如果类型有界返回 `Some(max_value)`，否则返回 `None`
    /// Returns `Some(max_value)` if bounded, `None` otherwise
    fn max_value() -> Option<Self>
    where
        Self: Sized,
    {
        None
    }
}

// ============================================================================
// Fixed Trait - 固定性（退化为单点）
// ============================================================================

/// Fixed - 固定性 trait
/// Fixed - Fixedness trait
///
/// 表示类型是否退化为单点（即只有一个可能的值）。
/// Represents whether a type degenerates to a single point (only one possible value).
///
/// 对于区间类型，如果上下界相等且都是闭区间，则退化为单点。
/// For interval types, if upper and lower bounds are equal and both closed, it degenerates to a point.
///
/// # 示例 / Examples
/// ```
/// // 非固定类型：有多个可能值
/// // Non-fixed type: has multiple possible values
/// assert!(!i32::is_fixed());
///
/// // 固定区间：[5, 5] 只有一个值
/// // Fixed interval: [5, 5] has only one value
/// // （需要 ValueRange 实现）
/// ```
pub trait Fixed {
    /// 判断类型是否固定（退化为单点）
    /// Check if the type is fixed (degenerates to a single point)
    ///
    /// # 返回 / Returns
    /// 如果类型只有一个可能的值返回 `true`，否则返回 `false`
    /// Returns `true` if the type has only one possible value, `false` otherwise
    fn is_fixed() -> bool;

    /// 获取固定值（如果存在）
    /// Get the fixed value (if exists)
    ///
    /// # 返回 / Returns
    /// 如果类型退化为单点返回 `Some(value)`，否则返回 `None`
    /// Returns `Some(value)` if degenerates to a point, `None` otherwise
    fn fixed_value() -> Option<Self>
    where
        Self: Sized,
    {
        None
    }
}

// 宏：为类型实现 Bounded
// Macro: Implement Bounded for types
// ============================================================================

/// 为有界类型实现 Bounded trait
/// Implement Bounded trait for bounded types
macro_rules! impl_bounded {
    ($type:ty, $min:expr, $max:expr) => {
        impl Bounded for $type {
            fn is_bounded() -> bool {
                true
            }

            fn min_value() -> Option<Self> {
                Some($min)
            }

            fn max_value() -> Option<Self> {
                Some($max)
            }
        }
    };
}

/// 为无界类型实现 Bounded trait
/// Implement Bounded trait for unbounded types
macro_rules! impl_unbounded {
    ($type:ty) => {
        impl Bounded for $type {
            fn is_bounded() -> bool {
                false
            }
        }
    };
}

// ============================================================================
// 有界类型 Bounded 实现 / Bounded implementations for bounded types
// ============================================================================

// 有符号整数 / Signed integers
impl_bounded!(i64, i64::MIN, i64::MAX);
impl_bounded!(i32, i32::MIN, i32::MAX);
impl_bounded!(i128, i128::MIN, i128::MAX);
impl_bounded!(i16, i16::MIN, i16::MAX);
impl_bounded!(i8, i8::MIN, i8::MAX);
impl_bounded!(isize, isize::MIN, isize::MAX);

// 无符号整数 / Unsigned integers
impl_bounded!(u64, u64::MIN, u64::MAX);
impl_bounded!(u32, u32::MIN, u32::MAX);
impl_bounded!(u128, u128::MIN, u128::MAX);
impl_bounded!(u16, u16::MIN, u16::MAX);
impl_bounded!(u8, u8::MIN, u8::MAX);
impl_bounded!(usize, usize::MIN, usize::MAX);

// 浮点数（有界但有特殊值）/ Floating point (bounded but with special values)
impl_bounded!(f64, f64::MIN, f64::MAX);
impl_bounded!(f32, f32::MIN, f32::MAX);

// ============================================================================
// 无界类型 Bounded 实现 / Bounded implementations for unbounded types
// ============================================================================

use bigdecimal::BigDecimal;
use num_bigint::{BigInt, BigUint};
use num_rational::{BigRational, Rational32, Rational64};

impl_unbounded!(BigDecimal);
impl_unbounded!(BigInt);
impl_unbounded!(BigUint);
impl_unbounded!(BigRational);
impl_unbounded!(Rational64);
impl_unbounded!(Rational32);

// ============================================================================
// Fixed Trait 实现 / Fixed Trait implementations
// ============================================================================

/// 为非固定类型实现 Fixed trait
/// Implement Fixed trait for non-fixed types
macro_rules! impl_not_fixed {
    ($type:ty) => {
        impl Fixed for $type {
            fn is_fixed() -> bool {
                false
            }
        }
    };
}

// 所有基本数值类型都不是固定的
// All basic numeric types are not fixed
impl_not_fixed!(i64);
impl_not_fixed!(i32);
impl_not_fixed!(i128);
impl_not_fixed!(i16);
impl_not_fixed!(i8);
impl_not_fixed!(isize);
impl_not_fixed!(u64);
impl_not_fixed!(u32);
impl_not_fixed!(u128);
impl_not_fixed!(u16);
impl_not_fixed!(u8);
impl_not_fixed!(usize);
impl_not_fixed!(f64);
impl_not_fixed!(f32);
impl_not_fixed!(BigDecimal);
impl_not_fixed!(BigInt);
impl_not_fixed!(BigUint);
impl_not_fixed!(BigRational);
impl_not_fixed!(Rational64);
impl_not_fixed!(Rational32);

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Bounded 测试 / Bounded tests
    // ========================================================================

    #[test]
    fn test_i32_bounded() {
        assert!(i32::is_bounded());
        assert_eq!(i32::min_value(), Some(i32::MIN));
        assert_eq!(i32::max_value(), Some(i32::MAX));
    }

    #[test]
    fn test_i64_bounded() {
        assert!(i64::is_bounded());
        assert_eq!(i64::min_value(), Some(i64::MIN));
        assert_eq!(i64::max_value(), Some(i64::MAX));
    }

    #[test]
    fn test_u64_bounded() {
        assert!(u64::is_bounded());
        assert_eq!(u64::min_value(), Some(u64::MIN));
        assert_eq!(u64::max_value(), Some(u64::MAX));
    }

    #[test]
    fn test_f64_bounded() {
        assert!(f64::is_bounded());
        assert_eq!(f64::min_value(), Some(f64::MIN));
        assert_eq!(f64::max_value(), Some(f64::MAX));
    }

    #[test]
    fn test_f32_bounded() {
        assert!(f32::is_bounded());
        assert_eq!(f32::min_value(), Some(f32::MIN));
        assert_eq!(f32::max_value(), Some(f32::MAX));
    }

    #[test]
    fn test_bigint_unbounded() {
        assert!(!BigInt::is_bounded());
        assert_eq!(BigInt::min_value(), None);
        assert_eq!(BigInt::max_value(), None);
    }

    #[test]
    fn test_bigdecimal_unbounded() {
        assert!(!BigDecimal::is_bounded());
        assert_eq!(BigDecimal::min_value(), None);
        assert_eq!(BigDecimal::max_value(), None);
    }

    #[test]
    fn test_biguint_unbounded() {
        assert!(!BigUint::is_bounded());
        assert_eq!(BigUint::min_value(), None);
        assert_eq!(BigUint::max_value(), None);
    }

    // ========================================================================
    // Fixed 测试 / Fixed tests
    // ========================================================================

    #[test]
    fn test_i32_not_fixed() {
        assert!(!i32::is_fixed());
        assert_eq!(i32::fixed_value(), None);
    }

    #[test]
    fn test_f64_not_fixed() {
        assert!(!f64::is_fixed());
        assert_eq!(f64::fixed_value(), None);
    }

    #[test]
    fn test_bigint_not_fixed() {
        assert!(!BigInt::is_fixed());
        assert_eq!(BigInt::fixed_value(), None);
    }
}
