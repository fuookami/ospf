//! # 因式分解算法 / Integer Factorization Algorithm
//!
//! 提供整数因式分解功能。
//! Provides integer factorization functionality.

use super::prime::get_primes;
use std::vec::Vec;

/// 对整数进行因式分解
/// Factorize an integer
///
/// 返回素因子及其指数的列表。
/// Returns a list of prime factors with their exponents.
///
/// # 参数 / Parameters
///
/// * `num` - 要分解的正整数 / Positive integer to factorize
///
/// # 返回值 / Returns
///
/// 素因子和指数的向量 / Vector of prime factors and their exponents
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::factorize;
///
/// let factors = factorize(12);
/// assert_eq!(factors, vec![(2, 2), (3, 1)]); // 12 = 2^2 * 3^1
///
/// let factors = factorize(100);
/// assert_eq!(factors, vec![(2, 2), (5, 2)]); // 100 = 2^2 * 5^2
/// ```
pub fn factorize(num: usize) -> Vec<(usize, usize)> {
    if num <= 1 {
        return Vec::new();
    }

    let mut n = num;
    let mut factors = Vec::new();

    // 获取可能需要的素数
    // Get primes we might need
    let primes = get_primes((num as f64).sqrt() as usize + 1);

    for prime in primes {
        if prime * prime > num {
            break;
        }

        let mut exponent = 0;
        while n % prime == 0 {
            exponent += 1;
            n /= prime;
        }

        if exponent > 0 {
            factors.push((prime, exponent));
        }
    }

    // 如果剩余的 n > 1，它是一个素因子
    // If remaining n > 1, it's a prime factor
    if n > 1 {
        factors.push((n, 1));
    }

    factors
}

/// 对 i64 进行因式分解
/// Factorize an i64 integer
///
/// # 参数 / Parameters
///
/// * `num` - 要分解的整数 / Integer to factorize
///
/// # 返回值 / Returns
///
/// 素因子和指数的向量 / Vector of prime factors and their exponents
pub fn factorize_i64(num: i64) -> Vec<(i64, i64)> {
    let num = num.abs();
    if num <= 1 {
        return Vec::new();
    }

    let mut n = num;
    let mut factors = Vec::new();

    // 先处理 2
    // Handle 2 first
    let mut exponent = 0;
    while n % 2 == 0 {
        exponent += 1;
        n /= 2;
    }
    if exponent > 0 {
        factors.push((2, exponent));
    }

    // 处理奇数因子
    // Handle odd factors
    let mut i: i64 = 3;
    while i * i <= n {
        let mut exponent = 0;
        while n % i == 0 {
            exponent += 1;
            n /= i;
        }
        if exponent > 0 {
            factors.push((i, exponent));
        }
        i += 2;
    }

    if n > 1 {
        factors.push((n, 1));
    }

    factors
}

/// 对 u64 进行因式分解
/// Factorize a u64 integer
///
/// # 参数 / Parameters
///
/// * `num` - 要分解的非负整数 / Non-negative integer to factorize
///
/// # 返回值 / Returns
///
/// 素因子和指数的向量 / Vector of prime factors and their exponents
pub fn factorize_u64(num: u64) -> Vec<(u64, u64)> {
    if num <= 1 {
        return Vec::new();
    }

    let mut n = num;
    let mut factors = Vec::new();

    // 先处理 2
    // Handle 2 first
    let mut exponent = 0;
    while n % 2 == 0 {
        exponent += 1;
        n /= 2;
    }
    if exponent > 0 {
        factors.push((2, exponent));
    }

    // 处理奇数因子
    // Handle odd factors
    let mut i: u64 = 3;
    while i * i <= n {
        let mut exponent = 0;
        while n % i == 0 {
            exponent += 1;
            n /= i;
        }
        if exponent > 0 {
            factors.push((i, exponent));
        }
        i += 2;
    }

    if n > 1 {
        factors.push((n, 1));
    }

    factors
}

/// 从因式分解结果重构原数
/// Reconstruct the original number from factorization
///
/// # 参数 / Parameters
///
/// * `factors` - 素因子和指数的切片 / Slice of prime factors and exponents
///
/// # 返回值 / Returns
///
/// 重构的数值 / Reconstructed number
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::{factorize, defactorize};
///
/// let factors = factorize(12);
/// assert_eq!(defactorize(&factors), 12);
/// ```
pub fn defactorize(factors: &[(usize, usize)]) -> usize {
    factors
        .iter()
        .fold(1, |acc, &(prime, exp)| acc * prime.pow(exp as u32))
}

/// 获取所有因子（包括 1 和自身）
/// Get all divisors (including 1 and itself)
///
/// # 参数 / Parameters
///
/// * `num` - 正整数 / Positive integer
///
/// # 返回值 / Returns
///
/// 所有因子的向量（已排序） / Vector of all divisors (sorted)
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::divisors;
///
/// let divs = divisors(12);
/// assert_eq!(divs, vec![1, 2, 3, 4, 6, 12]);
/// ```
pub fn divisors(num: usize) -> Vec<usize> {
    if num == 0 {
        return Vec::new();
    }
    if num == 1 {
        return vec![1];
    }

    let factors = factorize(num);
    let mut result = vec![1];

    for (prime, exp) in factors {
        let current_len = result.len();
        let mut multiplier = 1;
        for _ in 0..exp {
            multiplier *= prime;
            for i in 0..current_len {
                result.push(result[i] * multiplier);
            }
        }
    }

    result.sort();
    result
}

/// 计算因子个数
/// Count the number of divisors
///
/// # 参数 / Parameters
///
/// * `num` - 正整数 / Positive integer
///
/// # 返回值 / Returns
///
/// 因子个数 / Number of divisors
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::divisor_count;
///
/// assert_eq!(divisor_count(12), 6); // 1, 2, 3, 4, 6, 12
/// assert_eq!(divisor_count(7), 2); // 1, 7
/// ```
pub fn divisor_count(num: usize) -> usize {
    if num == 0 {
        return 0;
    }
    if num == 1 {
        return 1;
    }

    let factors = factorize(num);
    factors.iter().map(|&(_, exp)| exp + 1).product()
}

/// 计算欧拉函数 φ(n)
/// Calculate Euler's totient function φ(n)
///
/// φ(n) 表示小于 n 且与 n 互质的正整数个数。
/// φ(n) represents the count of positive integers less than n that are coprime to n.
///
/// # 参数 / Parameters
///
/// * `num` - 正整数 / Positive integer
///
/// # 返回值 / Returns
///
/// 欧拉函数值 / Euler's totient function value
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::euler_totient;
///
/// assert_eq!(euler_totient(1), 1);
/// assert_eq!(euler_totient(12), 4); // 1, 5, 7, 11
/// assert_eq!(euler_totient(7), 6); // 素数 p 有 φ(p) = p - 1
/// ```
pub fn euler_totient(num: usize) -> usize {
    if num == 0 {
        return 0;
    }
    if num == 1 {
        return 1;
    }

    let factors = factorize(num);
    factors
        .iter()
        .fold(num, |acc, &(prime, _)| acc / prime * (prime - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factorize() {
        assert!(factorize(0).is_empty());
        assert!(factorize(1).is_empty());
        assert_eq!(factorize(2), vec![(2, 1)]);
        assert_eq!(factorize(3), vec![(3, 1)]);
        assert_eq!(factorize(4), vec![(2, 2)]);
        assert_eq!(factorize(12), vec![(2, 2), (3, 1)]);
        assert_eq!(factorize(100), vec![(2, 2), (5, 2)]);
        assert_eq!(factorize(360), vec![(2, 3), (3, 2), (5, 1)]);
    }

    #[test]
    fn test_factorize_i64() {
        assert!(factorize_i64(0).is_empty());
        assert!(factorize_i64(1).is_empty());
        assert_eq!(factorize_i64(12), vec![(2, 2), (3, 1)]);
        assert_eq!(factorize_i64(-12), vec![(2, 2), (3, 1)]);
    }

    #[test]
    fn test_factorize_u64() {
        assert!(factorize_u64(0).is_empty());
        assert!(factorize_u64(1).is_empty());
        assert_eq!(factorize_u64(12), vec![(2, 2), (3, 1)]);
        assert_eq!(factorize_u64(100), vec![(2, 2), (5, 2)]);
    }

    #[test]
    fn test_defactorize() {
        assert_eq!(defactorize(&factorize(12)), 12);
        assert_eq!(defactorize(&factorize(100)), 100);
        assert_eq!(defactorize(&factorize(360)), 360);
    }

    #[test]
    fn test_divisors() {
        assert_eq!(divisors(1), vec![1]);
        assert_eq!(divisors(12), vec![1, 2, 3, 4, 6, 12]);
        assert_eq!(divisors(7), vec![1, 7]);
        assert_eq!(divisors(100), vec![1, 2, 4, 5, 10, 20, 25, 50, 100]);
    }

    #[test]
    fn test_divisor_count() {
        assert_eq!(divisor_count(1), 1);
        assert_eq!(divisor_count(12), 6);
        assert_eq!(divisor_count(7), 2);
        assert_eq!(divisor_count(100), 9);
    }

    #[test]
    fn test_euler_totient() {
        assert_eq!(euler_totient(1), 1);
        assert_eq!(euler_totient(12), 4);
        assert_eq!(euler_totient(7), 6);
        assert_eq!(euler_totient(100), 40);
    }
}
