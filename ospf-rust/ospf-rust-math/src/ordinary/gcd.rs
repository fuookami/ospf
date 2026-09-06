//! # 最大公约数算法 / Greatest Common Divisor Algorithm
//!
//! 提供计算最大公约数（GCD）的功能。
//! Provides functionality to calculate the Greatest Common Divisor (GCD).

/// 计算两个数的最大公约数（欧几里得算法）
/// Calculate GCD of two numbers (Euclidean algorithm)
///
/// # 参数 / Parameters
///
/// * `x` - 第一个非负整数 / First non-negative integer
/// * `y` - 第二个非负整数 / Second non-negative integer
///
/// # 返回值 / Returns
///
/// 最大公约数 / Greatest common divisor
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::gcd;
///
/// assert_eq!(gcd(12, 8), 4);
/// assert_eq!(gcd(17, 13), 1);
/// ```
pub fn gcd(x: usize, y: usize) -> usize {
    if x == 0 {
        return y;
    }
    if y == 0 {
        return x;
    }

    let mut a = x;
    let mut b = y;
    while b != 0 {
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        b -= a;
    }
    a
}

/// 使用取模优化的 GCD 算法
/// GCD algorithm optimized with modulo
///
/// # 参数 / Parameters
///
/// * `x` - 第一个非负整数 / First non-negative integer
/// * `y` - 第二个非负整数 / Second non-negative integer
///
/// # 返回值 / Returns
///
/// 最大公约数 / Greatest common divisor
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::gcd_mod;
///
/// assert_eq!(gcd_mod(12, 8), 4);
/// assert_eq!(gcd_mod(17, 13), 1);
/// ```
pub fn gcd_mod(x: usize, y: usize) -> usize {
    if x == 0 {
        return y;
    }
    if y == 0 {
        return x;
    }

    let mut a = x;
    let mut b = y;
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

/// 计算多个数的最大公约数
/// Calculate GCD of multiple numbers
///
/// # 参数 / Parameters
///
/// * `numbers` - 数的迭代器 / Iterator of numbers
///
/// # 返回值 / Returns
///
/// 最大公约数 / Greatest common divisor
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::gcd_many;
///
/// assert_eq!(gcd_many(vec![12, 8, 4]), 4);
/// assert_eq!(gcd_many(vec![17, 13, 11]), 1);
/// ```
pub fn gcd_many<I>(numbers: I) -> usize
where
    I: IntoIterator<Item = usize>,
{
    let mut iter = numbers.into_iter();
    let first = match iter.next() {
        Some(n) => n,
        None => return 1, // 空集合返回 1
    };

    iter.fold(first, |acc, x| gcd_mod(acc, x))
}

/// 计算两个 i64 数的最大公约数
/// Calculate GCD of two i64 numbers
///
/// # 参数 / Parameters
///
/// * `x` - 第一个整数 / First integer
/// * `y` - 第二个整数 / Second integer
///
/// # 返回值 / Returns
///
/// 最大公约数 / Greatest common divisor
pub fn gcd_i64(x: i64, y: i64) -> i64 {
    let x = x.abs();
    let y = y.abs();
    gcd_mod(x as usize, y as usize) as i64
}

/// 计算两个 u64 数的最大公约数
/// Calculate GCD of two u64 numbers
///
/// # 参数 / Parameters
///
/// * `x` - 第一个非负整数 / First non-negative integer
/// * `y` - 第二个非负整数 / Second non-negative integer
///
/// # 返回值 / Returns
///
/// 最大公约数 / Greatest common divisor
pub fn gcd_u64(x: u64, y: u64) -> u64 {
    if x == 0 {
        return y;
    }
    if y == 0 {
        return x;
    }

    let mut a = x;
    let mut b = y;
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

/// 扩展欧几里得算法
/// Extended Euclidean Algorithm
///
/// 计算 gcd(a, b) 以及满足 ax + by = gcd(a, b) 的系数 x, y。
/// Calculates gcd(a, b) and coefficients x, y such that ax + by = gcd(a, b).
///
/// # 参数 / Parameters
///
/// * `a` - 第一个整数 / First integer
/// * `b` - 第二个整数 / Second integer
///
/// # 返回值 / Returns
///
/// (gcd, x, y) 元组，其中 ax + by = gcd
/// (gcd, x, y) tuple where ax + by = gcd
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::extended_gcd;
///
/// let (g, x, y) = extended_gcd(12, 8);
/// assert_eq!(g, 4);
/// assert_eq!(12 * x + 8 * y, 4);
/// ```
pub fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if b == 0 {
        (a.abs(), if a >= 0 { 1 } else { -1 }, 0)
    } else {
        let (g, x, y) = extended_gcd(b, a % b);
        (g, y, x - (a / b) * y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(5, 0), 5);
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(17, 13), 1);
        assert_eq!(gcd(100, 50), 50);
        assert_eq!(gcd(48, 18), 6);
    }

    #[test]
    fn test_gcd_mod() {
        assert_eq!(gcd_mod(0, 5), 5);
        assert_eq!(gcd_mod(5, 0), 5);
        assert_eq!(gcd_mod(12, 8), 4);
        assert_eq!(gcd_mod(17, 13), 1);
        assert_eq!(gcd_mod(100, 50), 50);
        assert_eq!(gcd_mod(48, 18), 6);
    }

    #[test]
    fn test_gcd_many() {
        assert_eq!(gcd_many(vec![12, 8, 4]), 4);
        assert_eq!(gcd_many(vec![17, 13, 11]), 1);
        assert_eq!(gcd_many(vec![100, 50, 25]), 25);
        assert_eq!(gcd_many(vec![]), 1);
        assert_eq!(gcd_many(vec![7]), 7);
    }

    #[test]
    fn test_gcd_i64() {
        assert_eq!(gcd_i64(-12, 8), 4);
        assert_eq!(gcd_i64(12, -8), 4);
        assert_eq!(gcd_i64(-12, -8), 4);
    }

    #[test]
    fn test_extended_gcd() {
        let (g, x, y) = extended_gcd(12, 8);
        assert_eq!(g, 4);
        assert_eq!(12 * x + 8 * y, 4);

        let (g, x, y) = extended_gcd(17, 13);
        assert_eq!(g, 1);
        assert_eq!(17 * x + 13 * y, 1);

        let (g, x, y) = extended_gcd(48, 18);
        assert_eq!(g, 6);
        assert_eq!(48 * x + 18 * y, 6);
    }
}
