//! 幂运算 traits
//! Power operation traits

use crate::ordinary::big_decimal_pow;
use bigdecimal::BigDecimal;
use num_traits::One;

/// 整数幂运算。
/// Integer power operation.
pub trait Pow {
    /// 幂运算结果类型 / Power operation output type
    type Output;

    /// 计算 `self^exponent`。
    /// Calculate `self^exponent`.
    fn pow(self, exponent: u32) -> Self::Output;

    /// 计算平方。
    /// Calculate square.
    fn sqr(self) -> Self::Output;

    /// 计算立方。
    /// Calculate cube.
    fn cub(self) -> Self::Output;
}

/// 浮点或非整数幂运算。
/// Floating-point or non-integer power operation.
pub trait PowF<I = Self> {
    /// 幂运算结果类型 / Power operation output type
    type Output;

    /// 计算 `self^exponent`。
    /// Calculate `self^exponent`.
    fn powf(self, exponent: I) -> Self::Output;

    /// 计算平方根。
    /// Calculate square root.
    fn sqrt(self) -> Self::Output;

    /// 计算立方根。
    /// Calculate cube root.
    fn cbrt(self) -> Self::Output;
}

/// 带精度的浮点或非整数幂运算。
/// Precision-aware floating-point or non-integer power operation.
pub trait PowFWithPrecision<I = Self, P = i64> {
    /// 幂运算结果类型 / Power operation output type
    type Output;

    /// 使用指定精度计算 `self^exponent`。
    /// Calculate `self^exponent` with the given precision.
    fn powf_with_precision(self, exponent: I, precision: P) -> Self::Output;

    /// 使用指定精度计算平方根。
    /// Calculate square root with the given precision.
    fn sqrt_with_precision(self, precision: P) -> Self::Output;

    /// 使用指定精度计算立方根。
    /// Calculate cube root with the given precision.
    fn cbrt_with_precision(self, precision: P) -> Self::Output;
}

macro_rules! impl_pow_for_integer {
    ($($type:ty),* $(,)?) => {
        $(
            impl Pow for $type {
                type Output = $type;

                fn pow(self, exponent: u32) -> Self::Output {
                    <$type>::pow(self, exponent)
                }

                fn sqr(self) -> Self::Output {
                    self * self
                }

                fn cub(self) -> Self::Output {
                    self * self * self
                }
            }

            impl Pow for &$type {
                type Output = $type;

                fn pow(self, exponent: u32) -> Self::Output {
                    <$type>::pow(*self, exponent)
                }

                fn sqr(self) -> Self::Output {
                    *self * *self
                }

                fn cub(self) -> Self::Output {
                    *self * *self * *self
                }
            }
        )*
    };
}

macro_rules! impl_pow_for_float {
    ($($type:ty),* $(,)?) => {
        $(
            impl Pow for $type {
                type Output = $type;

                fn pow(self, exponent: u32) -> Self::Output {
                    self.powi(exponent as i32)
                }

                fn sqr(self) -> Self::Output {
                    self * self
                }

                fn cub(self) -> Self::Output {
                    self * self * self
                }
            }

            impl Pow for &$type {
                type Output = $type;

                fn pow(self, exponent: u32) -> Self::Output {
                    (*self).powi(exponent as i32)
                }

                fn sqr(self) -> Self::Output {
                    *self * *self
                }

                fn cub(self) -> Self::Output {
                    *self * *self * *self
                }
            }

            impl PowF<$type> for $type {
                type Output = $type;

                fn powf(self, exponent: $type) -> Self::Output {
                    <$type>::powf(self, exponent)
                }

                fn sqrt(self) -> Self::Output {
                    <$type>::sqrt(self)
                }

                fn cbrt(self) -> Self::Output {
                    <$type>::cbrt(self)
                }
            }

            impl PowF<$type> for &$type {
                type Output = $type;

                fn powf(self, exponent: $type) -> Self::Output {
                    <$type>::powf(*self, exponent)
                }

                fn sqrt(self) -> Self::Output {
                    <$type>::sqrt(*self)
                }

                fn cbrt(self) -> Self::Output {
                    <$type>::cbrt(*self)
                }
            }
        )*
    };
}

impl_pow_for_integer!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);
impl_pow_for_float!(f32, f64);

impl Pow for BigDecimal {
    type Output = BigDecimal;

    fn pow(self, exponent: u32) -> Self::Output {
        self.powi(i64::from(exponent))
    }

    fn sqr(self) -> Self::Output {
        &self * &self
    }

    fn cub(self) -> Self::Output {
        &self * &self * &self
    }
}

impl Pow for &BigDecimal {
    type Output = BigDecimal;

    fn pow(self, exponent: u32) -> Self::Output {
        self.powi(i64::from(exponent))
    }

    fn sqr(self) -> Self::Output {
        self * self
    }

    fn cub(self) -> Self::Output {
        self * self * self
    }
}

impl PowF<BigDecimal> for BigDecimal {
    type Output = BigDecimal;

    fn powf(self, exponent: BigDecimal) -> Self::Output {
        big_decimal_pow::pow(&self, &exponent)
    }

    fn sqrt(self) -> Self::Output {
        let half = BigDecimal::one() / BigDecimal::from(2);
        big_decimal_pow::pow(&self, &half)
    }

    fn cbrt(self) -> Self::Output {
        let third = BigDecimal::one() / BigDecimal::from(3);
        big_decimal_pow::pow(&self, &third)
    }
}

impl<'a> PowF<&'a BigDecimal> for &'a BigDecimal {
    type Output = BigDecimal;

    fn powf(self, exponent: &'a BigDecimal) -> Self::Output {
        big_decimal_pow::pow(self, exponent)
    }

    fn sqrt(self) -> Self::Output {
        let half = BigDecimal::one() / BigDecimal::from(2);
        big_decimal_pow::pow(self, &half)
    }

    fn cbrt(self) -> Self::Output {
        let third = BigDecimal::one() / BigDecimal::from(3);
        big_decimal_pow::pow(self, &third)
    }
}

impl PowFWithPrecision<BigDecimal, i64> for BigDecimal {
    type Output = BigDecimal;

    fn powf_with_precision(self, exponent: BigDecimal, precision: i64) -> Self::Output {
        big_decimal_pow::pow_with_precision(&self, &exponent, precision)
    }

    fn sqrt_with_precision(self, precision: i64) -> Self::Output {
        let half = BigDecimal::one() / BigDecimal::from(2);
        big_decimal_pow::pow_with_precision(&self, &half, precision)
    }

    fn cbrt_with_precision(self, precision: i64) -> Self::Output {
        let third = BigDecimal::one() / BigDecimal::from(3);
        big_decimal_pow::pow_with_precision(&self, &third, precision)
    }
}

impl<'a> PowFWithPrecision<&'a BigDecimal, i64> for &'a BigDecimal {
    type Output = BigDecimal;

    fn powf_with_precision(self, exponent: &'a BigDecimal, precision: i64) -> Self::Output {
        big_decimal_pow::pow_with_precision(self, exponent, precision)
    }

    fn sqrt_with_precision(self, precision: i64) -> Self::Output {
        let half = BigDecimal::one() / BigDecimal::from(2);
        big_decimal_pow::pow_with_precision(self, &half, precision)
    }

    fn cbrt_with_precision(self, precision: i64) -> Self::Output {
        let third = BigDecimal::one() / BigDecimal::from(3);
        big_decimal_pow::pow_with_precision(self, &third, precision)
    }
}

/// 使用整数指数计算幂。
/// Calculate power with an integer exponent.
pub fn pow<T: Pow>(base: T, exponent: u32) -> T::Output {
    Pow::pow(base, exponent)
}

/// 计算平方。
/// Calculate square.
pub fn sqr<T: Pow>(base: T) -> T::Output {
    Pow::sqr(base)
}

/// 计算立方。
/// Calculate cube.
pub fn cub<T: Pow>(base: T) -> T::Output {
    Pow::cub(base)
}

/// 使用非整数指数计算幂。
/// Calculate power with a non-integer exponent.
pub fn powf<T, I>(base: T, exponent: I) -> <T as PowF<I>>::Output
where
    T: PowF<I>,
{
    PowF::powf(base, exponent)
}

/// 使用非整数指数和指定精度计算幂。
/// Calculate power with a non-integer exponent and the given precision.
pub fn powf_with_precision<T, I, P>(
    base: T,
    exponent: I,
    precision: P,
) -> <T as PowFWithPrecision<I, P>>::Output
where
    T: PowFWithPrecision<I, P>,
{
    PowFWithPrecision::powf_with_precision(base, exponent, precision)
}

/// 计算平方根。
/// Calculate square root.
pub fn sqrt<T, I>(base: T) -> <T as PowF<I>>::Output
where
    T: PowF<I>,
{
    PowF::sqrt(base)
}

/// 使用指定精度计算平方根。
/// Calculate square root with the given precision.
pub fn sqrt_with_precision<T, I, P>(base: T, precision: P) -> <T as PowFWithPrecision<I, P>>::Output
where
    T: PowFWithPrecision<I, P>,
{
    PowFWithPrecision::sqrt_with_precision(base, precision)
}

/// 计算立方根。
/// Calculate cube root.
pub fn cbrt<T, I>(base: T) -> <T as PowF<I>>::Output
where
    T: PowF<I>,
{
    PowF::cbrt(base)
}

/// 使用指定精度计算立方根。
/// Calculate cube root with the given precision.
pub fn cbrt_with_precision<T, I, P>(base: T, precision: P) -> <T as PowFWithPrecision<I, P>>::Output
where
    T: PowFWithPrecision<I, P>,
{
    PowFWithPrecision::cbrt_with_precision(base, precision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn integer_power_matches_rust_pow() {
        assert_eq!(Pow::pow(3_i32, 4), 81);
        assert_eq!(sqr(5_i64), 25);
        assert_eq!(cub(2_u32), 8);
    }

    #[test]
    fn float_power_supports_integer_and_fractional_exponents() {
        assert_eq!(Pow::pow(2.0_f64, 3), 8.0);
        assert_eq!(powf(9.0_f64, 0.5), 3.0);
        assert_eq!(sqrt(9.0_f64), 3.0);
        assert_eq!(cbrt(27.0_f64), 3.0);
    }

    #[test]
    fn bigdecimal_power_uses_high_precision_helpers() {
        let two = BigDecimal::from(2);
        let half = BigDecimal::from_str("0.5").unwrap();

        assert_eq!(sqr(&two), BigDecimal::from(4));
        let sqrt_two = powf(&two, &half);
        let expected = BigDecimal::from_str("1.4142").unwrap();
        assert!((sqrt_two - expected).abs() < BigDecimal::from_str("0.01").unwrap());
        assert!(sqrt_with_precision::<_, &BigDecimal, _>(&two, 32) > BigDecimal::from(1));
    }
}
