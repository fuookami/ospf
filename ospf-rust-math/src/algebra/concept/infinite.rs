//! Infinite - 无穷大支持 trait
//! Infinite - Infinity support trait

use bigdecimal::BigDecimal;
use num_bigint::{BigInt, BigUint};
use num_rational::{BigRational, Rational32, Rational64};

// ============================================================================
// Infinite Trait - 无穷大支持
// ============================================================================

/// Infinite - 无穷大支持 trait
/// Infinite - Infinity support trait
///
/// 标记类型是否有原生无穷大支持。
/// Marks whether a type has native infinity support.
///
/// 对于没有原生无穷大的类型（如 `i64`），可以使用 `ValueWrapper<T>` 来添加无穷大概念。
/// For types without native infinity (e.g., `i64`), use `ValueWrapper<T>` to add infinity concept.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::Infinite;
///
/// // f64 有原生无穷大
/// // f64 has native infinity
/// assert!(f64::infinity().is_some());
/// assert!(f64::negative_infinity().is_some());
///
/// // i64 没有原生无穷大
/// // i64 has no native infinity
/// assert!(i64::infinity().is_none());
/// ```
pub trait Infinite {
    /// 返回正无穷，如果类型原生支持
    /// Returns positive infinity if natively supported
    ///
    /// # 返回 / Returns
    /// 如果类型原生支持无穷大返回 `Some(infinity)`，否则返回 `None`
    /// Returns `Some(infinity)` if natively supported, `None` otherwise
    fn infinity() -> Option<Self>
    where
        Self: Sized,
    {
        None
    }

    /// 返回负无穷，如果类型原生支持
    /// Returns negative infinity if natively supported
    ///
    /// # 返回 / Returns
    /// 如果类型原生支持负无穷大返回 `Some(neg_infinity)`，否则返回 `None`
    /// Returns `Some(neg_infinity)` if natively supported, `None` otherwise
    fn negative_infinity() -> Option<Self>
    where
        Self: Sized,
    {
        None
    }

    /// 判断值是否为无穷大（正或负）
    /// Check if value is infinity (positive or negative)
    ///
    /// # 返回 / Returns
    /// 如果值为正无穷或负无穷返回 `true`，否则返回 `false`
    /// Returns `true` if value is positive or negative infinity, `false` otherwise
    fn is_infinity(&self) -> bool {
        false
    }

    /// 判断值是否为正无穷
    /// Check if value is positive infinity
    ///
    /// # 返回 / Returns
    /// 如果值为正无穷返回 `true`，否则返回 `false`
    /// Returns `true` if value is positive infinity, `false` otherwise
    fn is_positive_infinity(&self) -> bool {
        false
    }

    /// 判断值是否为负无穷
    /// Check if value is negative infinity
    ///
    /// # 返回 / Returns
    /// 如果值为负无穷返回 `true`，否则返回 `false`
    /// Returns `true` if value is negative infinity, `false` otherwise
    fn is_negative_infinity(&self) -> bool {
        false
    }
}

// ============================================================================
// Infinite Trait 实现 / Infinite Trait implementations
// ============================================================================

/// 为浮点类型实现 Infinite（有原生无穷大）
/// Implement Infinite for floating point types (with native infinity)
macro_rules! impl_infinite_float {
    ($type:ty) => {
        impl Infinite for $type {
            fn infinity() -> Option<Self> {
                Some(<$type>::INFINITY)
            }

            fn negative_infinity() -> Option<Self> {
                Some(<$type>::NEG_INFINITY)
            }

            fn is_infinity(&self) -> bool {
                self.is_infinite()
            }

            fn is_positive_infinity(&self) -> bool {
                *self == <$type>::INFINITY
            }

            fn is_negative_infinity(&self) -> bool {
                *self == <$type>::NEG_INFINITY
            }
        }
    };
}

/// 为没有原生无穷大的类型实现 Infinite（默认实现）
/// Implement Infinite for types without native infinity (default implementation)
macro_rules! impl_infinite_none {
    ($type:ty) => {
        impl Infinite for $type {}
    };
}

// 浮点数有原生无穷大 / Floating point has native infinity
impl_infinite_float!(f64);
impl_infinite_float!(f32);

// 整数没有原生无穷大 / Integers have no native infinity
impl_infinite_none!(i64);
impl_infinite_none!(i32);
impl_infinite_none!(i128);
impl_infinite_none!(i16);
impl_infinite_none!(i8);
impl_infinite_none!(isize);
impl_infinite_none!(u64);
impl_infinite_none!(u32);
impl_infinite_none!(u128);
impl_infinite_none!(u16);
impl_infinite_none!(u8);
impl_infinite_none!(usize);

// BigDecimal, BigInt, BigUint, Rational 没有原生无穷大
// BigDecimal, BigInt, BigUint, Rational have no native infinity
impl_infinite_none!(BigDecimal);
impl_infinite_none!(BigInt);
impl_infinite_none!(BigUint);
impl_infinite_none!(BigRational);
impl_infinite_none!(Rational64);
impl_infinite_none!(Rational32);

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_infinite() {
        assert!(f64::infinity().is_some());
        assert!(f64::negative_infinity().is_some());

        let pos_inf = f64::infinity().unwrap();
        let neg_inf = f64::negative_infinity().unwrap();

        assert!(pos_inf.is_positive_infinity());
        assert!(neg_inf.is_negative_infinity());
        assert!(pos_inf.is_infinity());
        assert!(neg_inf.is_infinity());
    }

    #[test]
    fn test_f32_infinite() {
        assert!(f32::infinity().is_some());
        assert!(f32::negative_infinity().is_some());
    }

    #[test]
    fn test_i64_no_infinite() {
        assert!(i64::infinity().is_none());
        assert!(i64::negative_infinity().is_none());
    }

    #[test]
    fn test_i32_no_infinite() {
        assert!(i32::infinity().is_none());
        assert!(i32::negative_infinity().is_none());
    }

    #[test]
    fn test_bigdecimal_no_infinite() {
        assert!(BigDecimal::infinity().is_none());
        assert!(BigDecimal::negative_infinity().is_none());
    }

    #[test]
    fn test_bigint_no_infinite() {
        assert!(BigInt::infinity().is_none());
        assert!(BigInt::negative_infinity().is_none());
    }
}
