//! Abs - 绝对值运算特征
//! Abs - Absolute value operation trait

use bigdecimal::BigDecimal;
use num_bigint::{BigInt, BigUint};
use num_rational::{BigRational, Rational32, Rational64};
use num_traits::Signed;

/// Abs - 绝对值运算特征
/// Abs - Absolute value operation trait
///
/// 定义了取绝对值的操作。
/// Defines the absolute value operation.
pub trait Abs {
    /// 绝对值运算的输出类型
    /// Output type of the absolute value operation
    type Output;

    /// 计算绝对值
    /// Calculate absolute value
    fn abs(self) -> Self::Output;
}

// ============================================================================
// 宏：为类型实现 Abs
// Macro: Implement Abs for types
// ============================================================================

/// 为有符号类型（实现 Signed trait）实现 Abs
/// Implement Abs for signed types (implementing Signed trait)
macro_rules! impl_abs_signed {
    ($type:ty) => {
        impl Abs for $type {
            type Output = $type;

            fn abs(self) -> Self::Output {
                <$type as Signed>::abs(&self)
            }
        }

        impl Abs for &$type {
            type Output = $type;

            fn abs(self) -> Self::Output {
                <$type as Signed>::abs(self)
            }
        }
    };
}

/// 为无符号类型实现 Abs（绝对值是自身）
/// Implement Abs for unsigned types (abs is identity)
macro_rules! impl_abs_unsigned {
    ($type:ty) => {
        impl Abs for $type {
            type Output = $type;

            fn abs(self) -> Self::Output {
                self
            }
        }

        impl Abs for &$type {
            type Output = $type;

            fn abs(self) -> Self::Output {
                *self
            }
        }
    };
}

/// 为多个有符号类型实现 Abs
/// Implement Abs for multiple signed types
macro_rules! impl_abs_for_signed_types {
    ($($type:ty),+ $(,)?) => {
        $(
            impl_abs_signed!($type);
        )+
    };
}

/// 为多个无符号类型实现 Abs
/// Implement Abs for multiple unsigned types
macro_rules! impl_abs_for_unsigned_types {
    ($($type:ty),+ $(,)?) => {
        $(
            impl_abs_unsigned!($type);
        )+
    };
}

// ============================================================================
// 有符号类型 Abs 实现 / Signed types Abs implementations
// 使用 num_traits::Signed trait
// ============================================================================

impl_abs_for_signed_types!(f64, f32, i64, i32, i128, i16, i8, isize);

// ============================================================================
// 无符号类型 Abs 实现 / Unsigned types Abs implementations
// 无符号整数的绝对值是自身
// ============================================================================

impl_abs_for_unsigned_types!(u64, u32, u128, u16, u8, usize);

// ============================================================================
// BigDecimal Abs 实现 / BigDecimal Abs implementation
// BigDecimal 实现了 Signed trait，但需要特殊处理引用
// ============================================================================

impl Abs for BigDecimal {
    type Output = BigDecimal;

    fn abs(self) -> Self::Output {
        BigDecimal::abs(&self)
    }
}

impl Abs for &BigDecimal {
    type Output = BigDecimal;

    fn abs(self) -> Self::Output {
        BigDecimal::abs(self)
    }
}

// ============================================================================
// BigInt Abs 实现 / BigInt Abs implementation
// BigInt 实现了 Signed trait
// ============================================================================

impl_abs_signed!(BigInt);

// ============================================================================
// BigUint Abs 实现 / BigUint Abs implementation
// BigUint 是无符号类型，绝对值是自身
// ============================================================================

impl Abs for BigUint {
    type Output = BigUint;

    fn abs(self) -> Self::Output {
        self
    }
}

impl Abs for &BigUint {
    type Output = BigUint;

    fn abs(self) -> Self::Output {
        self.clone()
    }
}

// ============================================================================
// Rational Abs 实现 / Rational Abs implementations
// Ratio<T> 实现了 Signed trait（当 T: Clone + Integer + Signed）
// ============================================================================

impl_abs_signed!(Rational64);
impl_abs_signed!(Rational32);

impl Abs for BigRational {
    type Output = BigRational;

    fn abs(self) -> Self::Output {
        <BigRational as Signed>::abs(&self)
    }
}

impl Abs for &BigRational {
    type Output = BigRational;

    fn abs(self) -> Self::Output {
        <BigRational as Signed>::abs(self)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_abs() {
        let v = -3.14_f64;
        let r = v.abs();
        assert!((r - 3.14).abs() < 1e-10);
    }

    #[test]
    fn test_f64_abs_positive() {
        let v = 3.14_f64;
        let r = v.abs();
        assert!((r - 3.14).abs() < 1e-10);
    }

    #[test]
    fn test_f32_abs() {
        let v = -2.5_f32;
        let r = v.abs();
        assert!((r - 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_i64_abs() {
        let v = -42_i64;
        let r = v.abs();
        assert_eq!(r, 42);
    }

    #[test]
    fn test_i64_abs_positive() {
        let v = 42_i64;
        let r = v.abs();
        assert_eq!(r, 42);
    }

    #[test]
    fn test_i32_abs() {
        let v = -100_i32;
        let r = v.abs();
        assert_eq!(r, 100);
    }

    #[test]
    fn test_u64_abs() {
        let v = 42_u64;
        let r = v.abs();
        assert_eq!(r, 42);
    }

    #[test]
    fn test_u32_abs() {
        let v = 100_u32;
        let r = v.abs();
        assert_eq!(r, 100);
    }

    #[test]
    fn test_bigdecimal_abs() {
        let v: BigDecimal = "-3.14".parse().unwrap();
        let r = v.abs();
        let expected: BigDecimal = "3.14".parse().unwrap();
        assert_eq!(r, expected);
    }

    #[test]
    fn test_bigdecimal_abs_positive() {
        let v: BigDecimal = "3.14".parse().unwrap();
        let expected: BigDecimal = "3.14".parse().unwrap();
        let r = v.abs();
        assert_eq!(r, expected);
    }

    #[test]
    fn test_ref_abs() {
        let v = -5.0_f64;
        let r = Abs::abs(&v);
        assert!((r - 5.0).abs() < 1e-10);
    }
}