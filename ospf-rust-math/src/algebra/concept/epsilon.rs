//! 代数性质 traits
//! Algebraic property traits

use bigdecimal::BigDecimal;
use num_bigint::{BigInt, BigUint};
use num_rational::{BigRational, Rational32, Rational64};
use num_traits::Zero;

// ============================================================================
// Epsilon Trait - 类型默认精度
// ============================================================================

/// Epsilon - 类型默认精度 trait
/// Epsilon - Default epsilon trait for types
///
/// 为数值类型定义默认精度容差。
/// Defines default epsilon tolerance for numeric types.
///
/// 该 trait 继承 `num_traits::Zero`，复用零值的定义。
/// This trait inherits `num_traits::Zero`, reusing the zero definition.
pub trait Epsilon: Zero + Sized {
    /// 返回该类型的默认精度容差
    /// Returns the default epsilon tolerance for this type
    fn epsilon() -> Self;
}

// ============================================================================
// 宏：为类型实现 Epsilon
// Macro: Implement Epsilon for types
// ============================================================================

/// 为单个类型实现 Epsilon
/// Implement Epsilon for a single type
macro_rules! impl_epsilon {
    ($type:ty, $value:expr) => {
        impl Epsilon for $type {
            fn epsilon() -> Self {
                $value
            }
        }
    };
}

/// 为多个类型实现相同的 Epsilon 值
/// Implement the same Epsilon value for multiple types
macro_rules! impl_epsilon_for_types {
    ($value:expr, $($type:ty),+ $(,)?) => {
        $(
            impl_epsilon!($type, $value);
        )+
    };
}

// ============================================================================
// 浮点数 Epsilon 实现 / Floating point Epsilon implementations
// ============================================================================

impl_epsilon!(f64, 1e-10);
impl_epsilon!(f32, 1e-6);

// ============================================================================
// 有符号整数 Epsilon 实现 / Signed integer Epsilon implementations
// 整数类型精确比较，精度为 0
// ============================================================================

impl_epsilon_for_types!(0, i64, i32, i128, i16, i8, isize);

// ============================================================================
// 无符号整数 Epsilon 实现 / Unsigned integer Epsilon implementations
// ============================================================================

impl_epsilon_for_types!(0, u64, u32, u128, u16, u8, usize);

// ============================================================================
// BigDecimal Epsilon 实现 / BigDecimal Epsilon implementation
// ============================================================================

impl Epsilon for BigDecimal {
    fn epsilon() -> Self {
        // 创建 1e-20 的 epsilon
        // Create epsilon with value 1e-20
        // BigDecimal::new(1, 20) 创建 1 * 10^(-20) = 1e-20
        BigDecimal::new(BigInt::from(1), 20)
    }
}

// ============================================================================
// BigInt Epsilon 实现 / BigInt Epsilon implementation
// 整数类型精确比较，精度为 0
// ============================================================================

impl Epsilon for BigInt {
    fn epsilon() -> Self {
        BigInt::zero()
    }
}

// ============================================================================
// BigUint Epsilon 实现 / BigUint Epsilon implementation
// ============================================================================

impl Epsilon for BigUint {
    fn epsilon() -> Self {
        BigUint::zero()
    }
}

// ============================================================================
// Rational Epsilon 实现 / Rational Epsilon implementations
// 有理数类型精确比较，精度为 0
// ============================================================================

impl Epsilon for Rational64 {
    fn epsilon() -> Self {
        Rational64::zero()
    }
}

impl Epsilon for Rational32 {
    fn epsilon() -> Self {
        Rational32::zero()
    }
}

impl Epsilon for BigRational {
    fn epsilon() -> Self {
        BigRational::zero()
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_epsilon() {
        assert!((f64::epsilon() - 1e-10).abs() < 1e-15);
        assert!(f64::zero().is_zero());
    }

    #[test]
    fn test_f32_epsilon() {
        assert!((f32::epsilon() - 1e-6).abs() < 1e-10);
        assert!(f32::zero().is_zero());
    }

    #[test]
    fn test_i64_epsilon() {
        assert_eq!(i64::epsilon(), 0);
        assert!(i64::zero().is_zero());
    }

    #[test]
    fn test_bigdecimal_epsilon() {
        let eps = BigDecimal::epsilon();
        assert!(eps > BigDecimal::zero());
        assert!(eps < BigDecimal::from(1));
    }
}
