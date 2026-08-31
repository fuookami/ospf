//! # 素数算法 / Prime Number Algorithm
//!
//! 提供素数判断、素数列表生成等功能。
//! Provides prime number checking, prime list generation, and related functions.

use std::sync::{Mutex, OnceLock};
use std::vec::Vec;

/// 素数缓存结构
/// Prime number cache structure
///
/// 使用埃拉托斯特尼筛法缓存素数，支持动态扩展。
/// Uses Sieve of Eratosthenes to cache primes, supports dynamic extension.
pub struct PrimeCache {
    current: usize,
    is_prime: Vec<bool>,
    primes: Vec<usize>,
}

impl PrimeCache {
    /// 创建新的素数缓存
    /// Create a new prime cache
    fn new() -> Self {
        let mut cache = Self {
            current: 0,
            is_prime: Vec::new(),
            primes: Vec::new(),
        };
        cache.sieve(1000);
        cache
    }

    /// 扩展筛法范围
    /// Extend the sieve range
    fn extend_sieve(&mut self, new_limit: usize) {
        if new_limit <= self.current {
            return;
        }

        let old_limit = self.current;
        let new_size = new_limit + 1;
        let sqrt_limit = (new_limit as f64).sqrt() as usize;

        // 扩展 is_prime 数组
        // Extend is_prime array
        self.is_prime.resize(new_size, true);

        if old_limit == 0 {
            // new_limit >= 0 恒为真，因为 new_limit 是 usize
            // new_limit >= 0 is always true since new_limit is usize
            self.is_prime[0] = false;
            if new_limit >= 1 {
                self.is_prime[1] = false;
            }
        }

        self.current = new_limit;

        // 用已有的素数标记新范围内的合数
        // Mark composites in new range using existing primes
        for &p in &self.primes {
            if p > sqrt_limit {
                break;
            }

            let start = if p * p > old_limit + 1 {
                p * p
            } else {
                ((old_limit + 1 + p - 1) / p) * p
            };

            for j in (start..=new_limit).step_by(p) {
                self.is_prime[j] = false;
            }
        }

        // 处理新范围内的素数
        // Process primes in new range
        let start = if old_limit < 2 { 2 } else { old_limit + 1 };
        for i in start..=new_limit {
            if self.is_prime[i] {
                self.primes.push(i);

                if i <= sqrt_limit {
                    let start_multiple = i * i.max(start);
                    for j in (start_multiple..=new_limit).step_by(i) {
                        self.is_prime[j] = false;
                    }
                }
            }
        }
    }

    /// 执行筛法
    /// Perform the sieve
    fn sieve(&mut self, limit: usize) {
        if limit <= self.current {
            return;
        }

        self.is_prime = vec![true; limit + 1];
        // limit >= 0 恒为真，因为 limit 是 usize
        // limit >= 0 is always true since limit is usize
        self.is_prime[0] = false;
        if limit >= 1 {
            self.is_prime[1] = false;
        }

        let sqrt_limit = (limit as f64).sqrt() as usize;

        for i in 2..=sqrt_limit {
            if self.is_prime[i] {
                for j in ((i * i)..=limit).step_by(i) {
                    self.is_prime[j] = false;
                }
            }
        }

        self.primes.clear();
        for i in 2..=limit {
            if self.is_prime[i] {
                self.primes.push(i);
            }
        }

        self.current = limit;
    }

    /// 获取小于等于 limit 的所有素数
    /// Get all primes up to limit
    pub fn get_primes(&mut self, limit: usize) -> Vec<usize> {
        if limit > self.current {
            self.extend_sieve(limit);
        }
        self.primes
            .iter()
            .filter(|&&p| p <= limit)
            .copied()
            .collect()
    }

    /// 判断一个数是否为素数
    /// Check if a number is prime
    pub fn is_prime(&mut self, num: usize) -> bool {
        if num <= 1 {
            return false;
        }

        if num > self.current {
            if num <= 1_000_000 {
                self.extend_sieve(num);
                return self.is_prime[num];
            } else {
                return self.is_prime_quick_check(num);
            }
        }

        self.is_prime[num]
    }

    /// 快速素数检查（用于大数）
    /// Quick prime check (for large numbers)
    fn is_prime_quick_check(&self, n: usize) -> bool {
        if n <= 1 {
            return false;
        }
        if n <= 3 {
            return true;
        }
        if n % 2 == 0 || n % 3 == 0 {
            return false;
        }

        // 检查缓存中的素数
        // Check against cached primes
        for &p in &self.primes {
            if p * p > n {
                break;
            }
            if n % p == 0 {
                return false;
            }
        }

        // 使用 6k±1 方法继续检查
        // Continue checking using 6k±1 method
        let mut i = 5;
        while i * i <= n {
            if n % i == 0 || n % (i + 2) == 0 {
                return false;
            }
            i += 6;
        }

        true
    }
}

/// 全局素数缓存
/// Global prime cache
static PRIME_CACHE: OnceLock<Mutex<PrimeCache>> = OnceLock::new();

/// 获取素数缓存的引用
/// Get reference to prime cache
fn get_cache() -> &'static Mutex<PrimeCache> {
    PRIME_CACHE.get_or_init(|| Mutex::new(PrimeCache::new()))
}

/// 判断一个数是否为素数
/// Check if a number is prime
///
/// # 参数 / Parameters
///
/// * `num` - 要检查的数 / Number to check
///
/// # 返回值 / Returns
///
/// 如果是素数返回 `true`，否则返回 `false`
/// Returns `true` if prime, `false` otherwise
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::is_prime;
///
/// assert!(is_prime(7));
/// assert!(!is_prime(4));
/// ```
pub fn is_prime(num: usize) -> bool {
    get_cache().lock().expect("prime cache mutex poisoned / 素数缓存互斥锁已中毒").is_prime(num)
}

/// 获取小于等于 limit 的所有素数
/// Get all primes up to limit
///
/// # 参数 / Parameters
///
/// * `limit` - 上限值 / Upper limit
///
/// # 返回值 / Returns
///
/// 素数列表 / List of primes
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::ordinary::get_primes;
///
/// let primes = get_primes(10);
/// assert_eq!(primes, vec![2, 3, 5, 7]);
/// ```
pub fn get_primes(limit: usize) -> Vec<usize> {
    get_cache().lock().expect("prime cache mutex poisoned / 素数缓存互斥锁已中毒").get_primes(limit)
}

/// 判断一个 u64 数是否为素数
/// Check if a u64 number is prime
pub fn is_prime_u64(num: u64) -> bool {
    if num <= usize::MAX as u64 {
        is_prime(num as usize)
    } else {
        // 对于大于 usize 的数，使用简单方法
        // For numbers larger than usize, use simple method
        if num <= 1 {
            return false;
        }
        if num <= 3 {
            return true;
        }
        if num % 2 == 0 || num % 3 == 0 {
            return false;
        }

        let mut i: u64 = 5;
        while i * i <= num {
            if num % i == 0 || num % (i + 2) == 0 {
                return false;
            }
            i += 6;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_prime() {
        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(!is_prime(4));
        assert!(is_prime(5));
        assert!(!is_prime(6));
        assert!(is_prime(7));
        assert!(!is_prime(8));
        assert!(!is_prime(9));
        assert!(!is_prime(10));
        assert!(is_prime(11));
        assert!(is_prime(13));
        assert!(is_prime(17));
        assert!(is_prime(19));
        assert!(is_prime(23));
        assert!(is_prime(97));
        assert!(!is_prime(100));
    }

    #[test]
    fn test_get_primes() {
        let primes = get_primes(10);
        assert_eq!(primes, vec![2, 3, 5, 7]);

        let primes = get_primes(20);
        assert_eq!(primes, vec![2, 3, 5, 7, 11, 13, 17, 19]);
    }

    #[test]
    fn test_large_prime() {
        assert!(is_prime(104729)); // 第 10000 个素数
        assert!(!is_prime(104730));
    }
}
