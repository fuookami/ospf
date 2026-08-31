//! Contains - 包含关系运算符
//! Contains - Containment operator

// ============================================================================
// Contains Trait - 包含关系
// ============================================================================

/// Contains - 包含关系运算符
/// Contains - Containment operator
///
/// 判断一个值是否在当前范围内。
/// Check if a value is within the current range.
///
/// # 类型参数 / Type Parameters
/// - `T`: 要检查的值类型
/// - `T`: The value type to check
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::operator::Contains;
///
/// // 对于标准 Range 类型
/// // For standard Range types
/// let range = 1..=10;
/// assert!(range.contains(&5));
/// assert!(!range.contains(&15));
/// ```
pub trait Contains<T> {
    /// 判断值是否在范围内
    /// Check if value is within the range
    ///
    /// # 参数 / Parameters
    /// - `value`: 要检查的值
    /// - `value`: The value to check
    ///
    /// # 返回 / Returns
    /// 如果值在范围内返回 `true`，否则返回 `false`
    /// Returns `true` if value is within range, `false` otherwise
    fn contains(&self, value: &T) -> bool;
}

// ============================================================================
// 为标准库类型实现 Contains
// Implement Contains for standard library types
// ============================================================================

// 为 RangeInclusive<T> 实现 Contains
// Implement Contains for RangeInclusive<T>
impl<T: PartialOrd> Contains<T> for std::ops::RangeInclusive<T> {
    fn contains(&self, value: &T) -> bool {
        std::ops::RangeInclusive::contains(self, value)
    }
}

// 为 Range<T> 实现 Contains
// Implement Contains for Range<T>
impl<T: PartialOrd> Contains<T> for std::ops::Range<T> {
    fn contains(&self, value: &T) -> bool {
        std::ops::Range::contains(self, value)
    }
}

// 为 RangeFrom<T> 实现 Contains
// Implement Contains for RangeFrom<T>
impl<T: PartialOrd> Contains<T> for std::ops::RangeFrom<T> {
    fn contains(&self, value: &T) -> bool {
        std::ops::RangeFrom::contains(self, value)
    }
}

// 为 RangeTo<T> 实现 Contains
// Implement Contains for RangeTo<T>
impl<T: PartialOrd> Contains<T> for std::ops::RangeTo<T> {
    fn contains(&self, value: &T) -> bool {
        std::ops::RangeTo::contains(self, value)
    }
}

// 为 RangeToInclusive<T> 实现 Contains
// Implement Contains for RangeToInclusive<T>
impl<T: PartialOrd> Contains<T> for std::ops::RangeToInclusive<T> {
    fn contains(&self, value: &T) -> bool {
        std::ops::RangeToInclusive::contains(self, value)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_inclusive_contains() {
        let range = 1..=10;
        assert!(range.contains(&5));
        assert!(range.contains(&1));
        assert!(range.contains(&10));
        assert!(!range.contains(&0));
        assert!(!range.contains(&11));
    }

    #[test]
    fn test_range_contains() {
        let range = 1..10;
        assert!(range.contains(&5));
        assert!(range.contains(&1));
        assert!(!range.contains(&10));
        assert!(!range.contains(&0));
    }

    #[test]
    fn test_range_from_contains() {
        let range = 1..;
        assert!(range.contains(&5));
        assert!(range.contains(&1));
        assert!(range.contains(&100));
        assert!(!range.contains(&0));
    }

    #[test]
    fn test_range_to_contains() {
        let range = ..10;
        assert!(range.contains(&5));
        assert!(range.contains(&0));
        assert!(!range.contains(&10));
        assert!(!range.contains(&15));
    }

    #[test]
    fn test_range_to_inclusive_contains() {
        let range = ..=10;
        assert!(range.contains(&5));
        assert!(range.contains(&0));
        assert!(range.contains(&10));
        assert!(!range.contains(&11));
    }
}