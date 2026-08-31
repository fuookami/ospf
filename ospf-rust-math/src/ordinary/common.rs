//! 常规数学小工具
//! Common mathematical utilities

use num_traits::{Float, One};
use std::ops::Mul;

/// 将值限制在闭区间内。
/// Clamp a value into a closed interval.
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// 返回两个值的有序二元组。
/// Return an ordered pair of two values.
pub fn minmax<T: PartialOrd>(lhs: T, rhs: T) -> (T, T) {
    if lhs <= rhs { (lhs, rhs) } else { (rhs, lhs) }
}

/// 计算指定底数的对数。
/// Calculate logarithm with a specified base.
pub fn log<T: Float>(value: T, base: T) -> T {
    value.log(base)
}

/// 使用快速幂计算非负整数次幂。
/// Calculate a non-negative integer power by exponentiation by squaring.
pub fn powi<T>(mut base: T, mut exponent: u64) -> T
where
    T: One + Clone + Mul<Output = T>,
{
    let mut result = T::one();
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = result * base.clone();
        }
        exponent >>= 1;
        if exponent > 0 {
            base = base.clone() * base;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{clamp, log, minmax, powi};

    #[test]
    fn clamp_limits_values_to_range() {
        assert_eq!(clamp(5, 0, 10), 5);
        assert_eq!(clamp(-1, 0, 10), 0);
        assert_eq!(clamp(11, 0, 10), 10);
    }

    #[test]
    fn minmax_orders_values() {
        assert_eq!(minmax(3, 1), (1, 3));
        assert_eq!(minmax(1, 3), (1, 3));
    }

    #[test]
    fn log_uses_given_base() {
        let value: f64 = log(8.0, 2.0);
        assert!((value - 3.0).abs() < 1e-10);
    }

    #[test]
    fn powi_uses_integer_fast_power() {
        assert_eq!(powi(2_i32, 0), 1);
        assert_eq!(powi(2_i32, 10), 1024);
        assert_eq!(powi(3_i64, 5), 243);
    }
}
