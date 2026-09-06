//! 缓存键定义
//! Cache Key Definitions

use std::sync::Arc;

// ============================================================================
// CacheKey - 缓存键
// ============================================================================

/// 缓存键 / Cache Key
///
/// 用于标识平展结果的缓存键。
/// Cache key for identifying flattened results.
///
/// # 设计说明 / Design Notes
///
/// - **单项式/多项式**: 使用 `Box<Inner>` 设计，缓存 Key 使用堆地址
/// - **中间符号**: 使用 `Arc<Inner>` 设计（需要共享），缓存 Key 使用 Arc 内部地址
///
/// - **Monomial/Polynomial**: Uses `Box<Inner>` design, cache key uses heap address
/// - **Intermediate Symbol**: Uses `Arc<Inner>` design (needs sharing), cache key uses Arc inner address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Key 值（内存地址）/ Key value (memory address)
    pub value: u64,
}

impl CacheKey {
    /// 从 Arc 指针地址创建（用于中间符号）/ Create from Arc pointer address (for intermediate symbols)
    pub fn from_arc<T>(ptr: &Arc<T>) -> Self {
        Self {
            // 使用 Arc 内部指针的地址作为 key
            // Use the address of the inner pointer in Arc as key
            value: Arc::as_ptr(ptr) as u64,
        }
    }

    /// 从 Box 内部地址创建（用于单项式/多项式）/ Create from Box inner address (for monomials/polynomials)
    pub fn from_box<T>(inner: &T) -> Self {
        Self {
            // 使用 Box 内部数据的地址作为 key
            // Use the address of the inner Box data as key
            value: inner as *const T as u64,
        }
    }

    /// 从原始地址创建 / Create from raw address
    pub fn from_raw(addr: u64) -> Self {
        Self { value: addr }
    }
}

// ============================================================================
// Cacheable - 可缓存 trait
// ============================================================================

/// 可缓存的 trait / Cacheable Trait
///
/// 拥有稳定内存地址的对象可实现此 trait，用于缓存。
/// Objects with stable memory addresses can implement this trait for caching.
pub trait Cacheable {
    /// 获取缓存 Key / Get cache key
    fn cache_key(&self) -> CacheKey;
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_from_arc() {
        let arc = Arc::new(42);
        let key1 = CacheKey::from_arc(&arc);
        let key2 = CacheKey::from_arc(&arc);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_key_from_box() {
        let value = Box::new(42);
        let key = CacheKey::from_box(&*value);
        assert!(key.value > 0);
    }

    #[test]
    fn test_cache_key_from_raw() {
        let key = CacheKey::from_raw(12345);
        assert_eq!(key.value, 12345);
    }
}
