//! # 最小公倍数算法 / Least Common Multiple Algorithm
//!
//! 提供计算最小公倍数（LCM）的功能。
//! Provides functionality to calculate the Least Common Multiple (LCM).

use super::factorization::factorize;
use super::gcd::{gcd_i64, gcd_mod, gcd_u64};

/// 计算两个数的最小公倍数
/// Calculate LCM of two numbers
///
/// # 参数 / Parameters
///
/// * `x` - 第一个非负整数 / First non-negative integer
/// * `y` - 第二个非负整数 / Second non-negative integer
///
/// # 返回值 / Returns
///
/// 最小公倍数 / Least common multiple
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::lcm;
///
/// assert_eq!(lcm(12, 8), 24);
/// assert_eq!(lcm(17, 13), 221);
/// ```
pub fn lcm(x: usize, y: usize) -> usize {
    if x == 0 || y == 0 {
        return 0;
    }

    let px = x;
    let py = y;
    let g = gcd_mod(px, py);
    (px / g) * py
}

/// 计算多个数的最小公倍数
/// Calculate LCM of multiple numbers
///
/// # 参数 / Parameters
///
/// * `numbers` - 数的迭代器 / Iterator of numbers
///
/// # 返回值 / Returns
///
/// 最小公倍数 / Least common multiple
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::lcm_many;
///
/// assert_eq!(lcm_many(vec![2, 3, 4]), 12);
/// assert_eq!(lcm_many(vec![4, 6, 8]), 24);
/// ```
pub fn lcm_many<I>(numbers: I) -> usize
where
    I: IntoIterator<Item = usize>,
{
    let mut iter = numbers.into_iter();
    let first = match iter.next() {
        Some(n) => n,
        None => return 1, // 空集合返回 1
    };

    iter.fold(first, |acc, x| lcm(acc, x))
}

/// 使用因式分解计算多个数的最小公倍数
/// Calculate LCM of multiple numbers using factorization
///
/// # 参数 / Parameters
///
/// * `numbers` - 数的迭代器 / Iterator of numbers
///
/// # 返回值 / Returns
///
/// 最小公倍数 / Least common multiple
pub fn lcm_by_factorization<I>(numbers: I) -> usize
where
    I: IntoIterator<Item = usize>,
{
    use std::collections::HashMap;

    let numbers: Vec<usize> = numbers.into_iter().collect();
    if numbers.is_empty() {
        return 1;
    }

    // 收集所有数的因式分解
    // Collect factorization of all numbers
    let all_factors: Vec<Vec<(usize, usize)>> = numbers.iter().map(|&n| factorize(n)).collect();

    // 检查是否有 0（因式分解为空表示输入为 0 或 1）
    // Check if any number is 0 (empty factorization means input is 0 or 1)
    if numbers.iter().any(|&n| n == 0) {
        return 0;
    }

    // 合并因子，取每个因子的最大指数
    // Merge factors, taking max exponent for each
    let mut merged: HashMap<usize, usize> = HashMap::new();
    for factors in all_factors {
        for (prime, exp) in factors {
            let entry = merged.entry(prime).or_insert(0);
            *entry = (*entry).max(exp);
        }
    }

    // 计算结果
    // Calculate result
    merged
        .iter()
        .fold(1, |acc, (&prime, &exp)| acc * prime.pow(exp as u32))
}

/// 计算两个 i64 数的最小公倍数
/// Calculate LCM of two i64 numbers
///
/// # 参数 / Parameters
///
/// * `x` - 第一个整数 / First integer
/// * `y` - 第二个整数 / Second integer
///
/// # 返回值 / Returns
///
/// 最小公倍数 / Least common multiple
pub fn lcm_i64(x: i64, y: i64) -> i64 {
    if x == 0 || y == 0 {
        return 0;
    }

    let px = x.abs();
    let py = y.abs();
    let g = gcd_i64(px, py);
    (px / g) * py
}

/// 计算两个 u64 数的最小公倍数
/// Calculate LCM of two u64 numbers
///
/// # 参数 / Parameters
///
/// * `x` - 第一个非负整数 / First non-negative integer
/// * `y` - 第二个非负整数 / Second non-negative integer
///
/// # 返回值 / Returns
///
/// 最小公倍数 / Least common multiple
pub fn lcm_u64(x: u64, y: u64) -> u64 {
    if x == 0 || y == 0 {
        return 0;
    }

    let px = x;
    let py = y;
    let g = gcd_u64(px, py);
    (px / g) * py
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lcm() {
        assert_eq!(lcm(0, 5), 0);
        assert_eq!(lcm(5, 0), 0);
        assert_eq!(lcm(12, 8), 24);
        assert_eq!(lcm(17, 13), 221);
        assert_eq!(lcm(4, 6), 12);
        assert_eq!(lcm(100, 50), 100);
    }

    #[test]
    fn test_lcm_many() {
        assert_eq!(lcm_many(vec![2, 3, 4]), 12);
        assert_eq!(lcm_many(vec![4, 6, 8]), 24);
        assert_eq!(lcm_many(vec![3, 5, 7]), 105);
        assert_eq!(lcm_many(vec![]), 1);
        assert_eq!(lcm_many(vec![7]), 7);
    }

    #[test]
    fn test_lcm_by_factorization() {
        assert_eq!(lcm_by_factorization(vec![2, 3, 4]), 12);
        assert_eq!(lcm_by_factorization(vec![4, 6, 8]), 24);
        assert_eq!(lcm_by_factorization(vec![3, 5, 7]), 105);
        assert_eq!(lcm_by_factorization(vec![]), 1);
    }

    #[test]
    fn test_lcm_i64() {
        assert_eq!(lcm_i64(12, 8), 24);
        assert_eq!(lcm_i64(-12, 8), 24);
        assert_eq!(lcm_i64(12, -8), 24);
        assert_eq!(lcm_i64(-12, -8), 24);
    }

    #[test]
    fn test_lcm_u64() {
        assert_eq!(lcm_u64(12, 8), 24);
        assert_eq!(lcm_u64(17, 13), 221);
    }

    #[test]
    fn test_lcm_consistency() {
        // 验证 lcm * gcd = a * b
        // Verify lcm * gcd = a * b
        for a in [12, 17, 100, 48] {
            for b in [8, 13, 50, 18] {
                assert_eq!(lcm(a, b) * gcd_mod(a, b), a * b);
            }
        }
    }
}
