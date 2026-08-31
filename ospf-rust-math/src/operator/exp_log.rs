//! 指数与对数运算 traits
//! Exponential and logarithm operation traits

use bigdecimal::{BigDecimal, FromPrimitive};
use crate::ordinary::big_decimal_pow;

/// 自然指数运算。
/// Natural exponential operation.
pub trait Exp {
    /// 指数运算结果类型 / Exponential operation output type
    type Output;

    /// 计算 `e^self`。
    /// Calculate `e^self`.
    fn exp(self) -> Self::Output;
}

/// 带精度的自然指数运算。
/// Precision-aware natural exponential operation.
pub trait ExpWithPrecision<P = i64> {
    /// 指数运算结果类型 / Exponential operation output type
    type Output;

    /// 使用指定精度计算 `e^self`。
    /// Calculate `e^self` with the given precision.
    fn exp_with_precision(self, precision: P) -> Self::Output;
}

/// 对数运算。
/// Logarithm operation.
pub trait Log<B = Self> {
    /// 对数运算结果类型 / Logarithm operation output type
    type Output;

    /// 计算以 `base` 为底的对数。
    /// Calculate logarithm with the given base.
    fn log(self, base: B) -> Option<Self::Output>;

    /// 计算常用对数。
    /// Calculate common logarithm.
    fn lg(self) -> Option<Self::Output>;

    /// 计算二进制对数。
    /// Calculate binary logarithm.
    fn lg2(self) -> Option<Self::Output>;

    /// 计算自然对数。
    /// Calculate natural logarithm.
    fn ln(self) -> Option<Self::Output>;
}

/// 带精度的对数运算。
/// Precision-aware logarithm operation.
pub trait LogWithPrecision<B = Self, P = i64> {
    /// 对数运算结果类型 / Logarithm operation output type
    type Output;

    /// 使用指定精度计算以 `base` 为底的对数。
    /// Calculate logarithm with the given base and precision.
    fn log_with_precision(self, base: B, precision: P) -> Option<Self::Output>;

    /// 使用指定精度计算常用对数。
    /// Calculate common logarithm with the given precision.
    fn lg_with_precision(self, precision: P) -> Option<Self::Output>;

    /// 使用指定精度计算二进制对数。
    /// Calculate binary logarithm with the given precision.
    fn lg2_with_precision(self, precision: P) -> Option<Self::Output>;

    /// 使用指定精度计算自然对数。
    /// Calculate natural logarithm with the given precision.
    fn ln_with_precision(self, precision: P) -> Option<Self::Output>;
}

macro_rules! impl_exp_log_for_float {
    ($($type:ty),* $(,)?) => {
        $(
            impl Exp for $type {
                type Output = $type;

                fn exp(self) -> Self::Output {
                    <$type>::exp(self)
                }
            }

            impl Exp for &$type {
                type Output = $type;

                fn exp(self) -> Self::Output {
                    <$type>::exp(*self)
                }
            }

            impl Log<$type> for $type {
                type Output = $type;

                fn log(self, base: $type) -> Option<Self::Output> {
                    valid_log(self, base).then(|| <$type>::log(self, base))
                }

                fn lg(self) -> Option<Self::Output> {
                    (self > 0.0).then(|| <$type>::log10(self))
                }

                fn lg2(self) -> Option<Self::Output> {
                    (self > 0.0).then(|| <$type>::log2(self))
                }

                fn ln(self) -> Option<Self::Output> {
                    (self > 0.0).then(|| <$type>::ln(self))
                }
            }

            impl Log<$type> for &$type {
                type Output = $type;

                fn log(self, base: $type) -> Option<Self::Output> {
                    valid_log(*self, base).then(|| <$type>::log(*self, base))
                }

                fn lg(self) -> Option<Self::Output> {
                    (*self > 0.0).then(|| <$type>::log10(*self))
                }

                fn lg2(self) -> Option<Self::Output> {
                    (*self > 0.0).then(|| <$type>::log2(*self))
                }

                fn ln(self) -> Option<Self::Output> {
                    (*self > 0.0).then(|| <$type>::ln(*self))
                }
            }
        )*
    };
}

impl_exp_log_for_float!(f32, f64);

impl Exp for BigDecimal {
    type Output = BigDecimal;

    fn exp(self) -> Self::Output {
        big_decimal_pow::exp(&self)
    }
}

impl Exp for &BigDecimal {
    type Output = BigDecimal;

    fn exp(self) -> Self::Output {
        big_decimal_pow::exp(self)
    }
}

impl ExpWithPrecision<i64> for BigDecimal {
    type Output = BigDecimal;

    fn exp_with_precision(self, precision: i64) -> Self::Output {
        big_decimal_pow::exp_with_precision(&self, precision)
    }
}

impl ExpWithPrecision<i64> for &BigDecimal {
    type Output = BigDecimal;

    fn exp_with_precision(self, precision: i64) -> Self::Output {
        big_decimal_pow::exp_with_precision(self, precision)
    }
}

impl Log<BigDecimal> for BigDecimal {
    type Output = BigDecimal;

    fn log(self, base: BigDecimal) -> Option<Self::Output> {
        valid_bigdecimal_log(&self, &base)
            .then(|| big_decimal_pow::ln(&self) / big_decimal_pow::ln(&base))
    }

    fn lg(self) -> Option<Self::Output> {
        self.log(BigDecimal::from(10))
    }

    fn lg2(self) -> Option<Self::Output> {
        self.log(BigDecimal::from(2))
    }

    fn ln(self) -> Option<Self::Output> {
        (self > BigDecimal::from(0)).then(|| big_decimal_pow::ln(&self))
    }
}

impl<'a> Log<&'a BigDecimal> for &'a BigDecimal {
    type Output = BigDecimal;

    fn log(self, base: &'a BigDecimal) -> Option<Self::Output> {
        valid_bigdecimal_log(self, base)
            .then(|| big_decimal_pow::ln(self) / big_decimal_pow::ln(base))
    }

    fn lg(self) -> Option<Self::Output> {
        let base = BigDecimal::from(10);
        valid_bigdecimal_log(self, &base)
            .then(|| big_decimal_pow::ln(self) / big_decimal_pow::ln(&base))
    }

    fn lg2(self) -> Option<Self::Output> {
        let base = BigDecimal::from(2);
        valid_bigdecimal_log(self, &base)
            .then(|| big_decimal_pow::ln(self) / big_decimal_pow::ln(&base))
    }

    fn ln(self) -> Option<Self::Output> {
        (self > &BigDecimal::from(0)).then(|| big_decimal_pow::ln(self))
    }
}

impl LogWithPrecision<BigDecimal, i64> for BigDecimal {
    type Output = BigDecimal;

    fn log_with_precision(self, base: BigDecimal, precision: i64) -> Option<Self::Output> {
        valid_bigdecimal_log(&self, &base).then(|| {
            big_decimal_pow::ln_with_precision(&self, precision)
                / big_decimal_pow::ln_with_precision(&base, precision)
        })
    }

    fn lg_with_precision(self, precision: i64) -> Option<Self::Output> {
        self.log_with_precision(BigDecimal::from(10), precision)
    }

    fn lg2_with_precision(self, precision: i64) -> Option<Self::Output> {
        self.log_with_precision(BigDecimal::from(2), precision)
    }

    fn ln_with_precision(self, precision: i64) -> Option<Self::Output> {
        (self > BigDecimal::from(0)).then(|| big_decimal_pow::ln_with_precision(&self, precision))
    }
}

impl<'a> LogWithPrecision<&'a BigDecimal, i64> for &'a BigDecimal {
    type Output = BigDecimal;

    fn log_with_precision(self, base: &'a BigDecimal, precision: i64) -> Option<Self::Output> {
        valid_bigdecimal_log(self, base).then(|| {
            big_decimal_pow::ln_with_precision(self, precision)
                / big_decimal_pow::ln_with_precision(base, precision)
        })
    }

    fn lg_with_precision(self, precision: i64) -> Option<Self::Output> {
        let base = BigDecimal::from(10);
        valid_bigdecimal_log(self, &base).then(|| {
            big_decimal_pow::ln_with_precision(self, precision)
                / big_decimal_pow::ln_with_precision(&base, precision)
        })
    }

    fn lg2_with_precision(self, precision: i64) -> Option<Self::Output> {
        let base = BigDecimal::from(2);
        valid_bigdecimal_log(self, &base).then(|| {
            big_decimal_pow::ln_with_precision(self, precision)
                / big_decimal_pow::ln_with_precision(&base, precision)
        })
    }

    fn ln_with_precision(self, precision: i64) -> Option<Self::Output> {
        (self > &BigDecimal::from(0)).then(|| big_decimal_pow::ln_with_precision(self, precision))
    }
}

/// 计算自然指数。
/// Calculate natural exponential.
pub fn exp<T: Exp>(value: T) -> T::Output {
    Exp::exp(value)
}

/// 使用指定精度计算自然指数。
/// Calculate natural exponential with the given precision.
pub fn exp_with_precision<T, P>(value: T, precision: P) -> <T as ExpWithPrecision<P>>::Output
where
    T: ExpWithPrecision<P>,
{
    ExpWithPrecision::exp_with_precision(value, precision)
}

/// 计算以 `base` 为底的对数。
/// Calculate logarithm with the given base.
pub fn log<T, B>(base: B, value: T) -> Option<<T as Log<B>>::Output>
where
    T: Log<B>,
{
    Log::log(value, base)
}

/// 使用指定精度计算以 `base` 为底的对数。
/// Calculate logarithm with the given base and precision.
pub fn log_with_precision<T, B, P>(
    base: B,
    value: T,
    precision: P,
) -> Option<<T as LogWithPrecision<B, P>>::Output>
where
    T: LogWithPrecision<B, P>,
{
    LogWithPrecision::log_with_precision(value, base, precision)
}

/// 计算常用对数。
/// Calculate common logarithm.
pub fn lg<T, B>(value: T) -> Option<<T as Log<B>>::Output>
where
    T: Log<B>,
{
    Log::lg(value)
}

/// 使用指定精度计算常用对数。
/// Calculate common logarithm with the given precision.
pub fn lg_with_precision<T, B, P>(
    value: T,
    precision: P,
) -> Option<<T as LogWithPrecision<B, P>>::Output>
where
    T: LogWithPrecision<B, P>,
{
    LogWithPrecision::lg_with_precision(value, precision)
}

/// 计算二进制对数。
/// Calculate binary logarithm.
pub fn lg2<T, B>(value: T) -> Option<<T as Log<B>>::Output>
where
    T: Log<B>,
{
    Log::lg2(value)
}

/// 使用指定精度计算二进制对数。
/// Calculate binary logarithm with the given precision.
pub fn lg2_with_precision<T, B, P>(
    value: T,
    precision: P,
) -> Option<<T as LogWithPrecision<B, P>>::Output>
where
    T: LogWithPrecision<B, P>,
{
    LogWithPrecision::lg2_with_precision(value, precision)
}

/// 计算自然对数。
/// Calculate natural logarithm.
pub fn ln<T, B>(value: T) -> Option<<T as Log<B>>::Output>
where
    T: Log<B>,
{
    Log::ln(value)
}

/// 使用指定精度计算自然对数。
/// Calculate natural logarithm with the given precision.
pub fn ln_with_precision<T, B, P>(
    value: T,
    precision: P,
) -> Option<<T as LogWithPrecision<B, P>>::Output>
where
    T: LogWithPrecision<B, P>,
{
    LogWithPrecision::ln_with_precision(value, precision)
}

fn valid_log<T>(value: T, base: T) -> bool
where
    T: PartialOrd + FromPrimitive + Copy,
{
    let zero = T::from_u8(0).expect("zero should be representable / 零应该可表示");
    let one = T::from_u8(1).expect("one should be representable / 一应该可表示");
    value > zero && base > zero && base != one
}

fn valid_bigdecimal_log(value: &BigDecimal, base: &BigDecimal) -> bool {
    value > &BigDecimal::from(0) && base > &BigDecimal::from(0) && base != &BigDecimal::from(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn float_exp_and_log_match_standard_functions() {
        assert!((exp(1.0_f64) - std::f64::consts::E).abs() < 1e-10);
        assert_eq!(log(2.0_f64, 8.0_f64), Some(3.0));
        assert_eq!(lg(100.0_f64), Some(2.0));
        assert_eq!(lg2(8.0_f64), Some(3.0));
        assert!(ln(-1.0_f64).is_none());
    }

    #[test]
    fn bigdecimal_exp_and_log_use_precision_helpers() {
        let e = BigDecimal::from_str("2.718281828459045").unwrap();
        let one = BigDecimal::from(1);

        let exp_one = exp(&one);
        assert!((exp_one - &e).abs() < BigDecimal::from_str("0.01").unwrap());
        let ln_e = ln(&e).unwrap();
        assert!((ln_e - &one).abs() < BigDecimal::from_str("0.01").unwrap());
        let precise_ln_e = ln_with_precision::<_, &BigDecimal, _>(&e, 32).unwrap();
        assert!((precise_ln_e - one).abs() < BigDecimal::from_str("0.01").unwrap());
    }
}
