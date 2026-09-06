//! 固定性 trait
//! Fixedness trait

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
/// use ospf_rust_math::Fixed;
///
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

use bigdecimal::BigDecimal;
use num_bigint::{BigInt, BigUint};
use num_rational::{BigRational, Rational32, Rational64};

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
