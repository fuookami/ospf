//! Reciprocal - 倒数运算特征
//! Reciprocal - Reciprocal operation trait

use bigdecimal::BigDecimal;
use num_bigint::{BigInt, BigUint};
use num_rational::{BigRational, Rational32, Rational64};
use num_traits::One;

/// Reciprocal - 倒数运算特征
/// Reciprocal - Reciprocal operation trait
///
/// 定义了取倒数的操作，即 1/x。
/// Defines the reciprocal operation, i.e., 1/x.
pub trait Reciprocal {
    /// 倒数运算的输出类型
    /// Output type of the reciprocal operation
    type Output;

    /// 计算倒数
    /// Calculate reciprocal
    ///
    /// 返回 1/self。
    /// Returns 1/self.
    fn reciprocal(self) -> Self::Output;
}

// ============================================================================
// ReciprocalRef - 引用倒数 / Reference Reciprocal
// ============================================================================

/// ReciprocalRef - 支持引用倒数的类型
/// ReciprocalRef - Types that support reference reciprocal
///
/// 表示 `&T -> T` 倒数操作。
/// Represents the `&T -> T` reciprocal operation.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::operator::ReciprocalRef;
///
/// fn reciprocal_ref<T: ReciprocalRef>(a: &T) -> T {
///     T::reciprocal_ref(a)
/// }
///
/// let result = reciprocal_ref(&2.0f64);
/// assert!((result - 0.5).abs() < 1e-10);
/// ```
pub trait ReciprocalRef: Sized {
    /// 引用倒数 / Reciprocal by reference
    fn reciprocal_ref(a: &Self) -> Self;
}

/// 为满足约束的类型自动实现 ReciprocalRef
/// Auto-implement ReciprocalRef for types satisfying constraints
impl<T> ReciprocalRef for T
where
    for<'a> &'a T: Reciprocal<Output = T>,
{
    fn reciprocal_ref(a: &Self) -> Self {
        Reciprocal::reciprocal(a)
    }
}

// ============================================================================
// 宏：为类型实现 Reciprocal
// Macro: Implement Reciprocal for types
// ============================================================================

/// 为浮点类型实现 Reciprocal（输出类型相同）
/// Implement Reciprocal for floating point types (output type is the same)
macro_rules! impl_reciprocal_float {
    ($type:ty) => {
        impl Reciprocal for $type {
            type Output = $type;

            fn reciprocal(self) -> Self::Output {
                <$type as One>::one() / self
            }
        }

        impl Reciprocal for &$type {
            type Output = $type;

            fn reciprocal(self) -> Self::Output {
                <$type as One>::one() / *self
            }
        }
    };
}

/// 为整数类型实现 Reciprocal（输出类型为 f64）
/// Implement Reciprocal for integer types (output type is f64)
macro_rules! impl_reciprocal_int {
    ($type:ty) => {
        impl Reciprocal for $type {
            type Output = f64;

            fn reciprocal(self) -> Self::Output {
                f64::one() / self as f64
            }
        }

        impl Reciprocal for &$type {
            type Output = f64;

            fn reciprocal(self) -> Self::Output {
                f64::one() / *self as f64
            }
        }
    };
}

/// 为多个浮点类型实现 Reciprocal
/// Implement Reciprocal for multiple floating point types
macro_rules! impl_reciprocal_for_float_types {
    ($($type:ty),+ $(,)?) => {
        $(
            impl_reciprocal_float!($type);
        )+
    };
}

/// 为多个整数类型实现 Reciprocal
/// Implement Reciprocal for multiple integer types
macro_rules! impl_reciprocal_for_int_types {
    ($($type:ty),+ $(,)?) => {
        $(
            impl_reciprocal_int!($type);
        )+
    };
}

// ============================================================================
// 浮点类型 Reciprocal 实现 / Floating point Reciprocal implementations
// ============================================================================

impl_reciprocal_for_float_types!(f64, f32);

// ============================================================================
// 整数类型 Reciprocal 实现 / Integer Reciprocal implementations
// 整数的倒数返回 f64
// ============================================================================

impl_reciprocal_for_int_types!(
    i64, i32, i128, i16, i8, isize, u64, u32, u128, u16, u8, usize
);

// ============================================================================
// BigDecimal Reciprocal 实现 / BigDecimal Reciprocal implementation
// ============================================================================

impl Reciprocal for BigDecimal {
    type Output = BigDecimal;

    fn reciprocal(self) -> Self::Output {
        BigDecimal::one() / self
    }
}

impl Reciprocal for &BigDecimal {
    type Output = BigDecimal;

    fn reciprocal(self) -> Self::Output {
        BigDecimal::one() / self
    }
}

// ============================================================================
// BigInt Reciprocal 实现 / BigInt Reciprocal implementation
// BigInt 的倒数返回 BigDecimal
// ============================================================================

impl Reciprocal for BigInt {
    type Output = BigDecimal;

    fn reciprocal(self) -> Self::Output {
        BigDecimal::one() / BigDecimal::from(self)
    }
}

impl Reciprocal for &BigInt {
    type Output = BigDecimal;

    fn reciprocal(self) -> Self::Output {
        BigDecimal::one() / BigDecimal::from(self.clone())
    }
}

// ============================================================================
// BigUint Reciprocal 实现 / BigUint Reciprocal implementation
// BigUint 的倒数返回 BigDecimal
// ============================================================================

impl Reciprocal for BigUint {
    type Output = BigDecimal;

    fn reciprocal(self) -> Self::Output {
        BigDecimal::one() / BigDecimal::from(BigInt::from(self))
    }
}

impl Reciprocal for &BigUint {
    type Output = BigDecimal;

    fn reciprocal(self) -> Self::Output {
        BigDecimal::one() / BigDecimal::from(BigInt::from(self.clone()))
    }
}

// ============================================================================
// Rational Reciprocal 实现 / Rational Reciprocal implementations
// Ratio<T> 有 recip() 方法，返回 Ratio<T>
// ============================================================================

impl Reciprocal for Rational64 {
    type Output = Rational64;

    fn reciprocal(self) -> Self::Output {
        Rational64::recip(&self)
    }
}

impl Reciprocal for &Rational64 {
    type Output = Rational64;

    fn reciprocal(self) -> Self::Output {
        Rational64::recip(self)
    }
}

impl Reciprocal for Rational32 {
    type Output = Rational32;

    fn reciprocal(self) -> Self::Output {
        Rational32::recip(&self)
    }
}

impl Reciprocal for &Rational32 {
    type Output = Rational32;

    fn reciprocal(self) -> Self::Output {
        Rational32::recip(self)
    }
}

impl Reciprocal for BigRational {
    type Output = BigRational;

    fn reciprocal(self) -> Self::Output {
        BigRational::recip(&self)
    }
}

impl Reciprocal for &BigRational {
    type Output = BigRational;

    fn reciprocal(self) -> Self::Output {
        BigRational::recip(self)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bigdecimal_reciprocal() {
        let v: BigDecimal = "2.0".parse().unwrap();
        let r = v.reciprocal();
        assert_eq!(r, BigDecimal::from(1) / BigDecimal::from(2));
    }

    #[test]
    fn test_bigdecimal_ref_reciprocal() {
        let v: BigDecimal = "4.0".parse().unwrap();
        let r = (&v).reciprocal();
        assert_eq!(r, BigDecimal::from(1) / BigDecimal::from(4));
    }

    #[test]
    fn test_f64_reciprocal() {
        let v = 2.0_f64;
        let r = v.reciprocal();
        assert!((r - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_f32_reciprocal() {
        let v = 4.0_f32;
        let r = v.reciprocal();
        assert!((r - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_i64_reciprocal() {
        let v = 2_i64;
        let r = v.reciprocal();
        assert!((r - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_i32_reciprocal() {
        let v = 4_i32;
        let r = v.reciprocal();
        assert!((r - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_u64_reciprocal() {
        let v = 2_u64;
        let r = v.reciprocal();
        assert!((r - 0.5).abs() < 1e-10);
    }
}
