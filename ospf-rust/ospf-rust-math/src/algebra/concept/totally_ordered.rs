//! 全序 trait
//! Totally ordered trait

use std::fmt::Debug;

// ============================================================================
// TotallyOrdered Trait - 全序
// ============================================================================

/// TotallyOrdered - 全序 trait
/// TotallyOrdered - Totally ordered trait
///
/// 表示类型具有全序关系，满足以下公理：
/// Represents that a type has a total order relation satisfying the following axioms:
///
/// 1. **完全性 / Totality**: 对于任意 `a, b`，`a ≤ b` 或 `b ≤ a`
///    For any `a, b`, either `a ≤ b` or `b ≤ a`
///
/// 2. **反对称性 / Antisymmetry**: `a ≤ b` 且 `b ≤ a` 蕴含 `a = b`
///    `a ≤ b` and `b ≤ a` implies `a = b`
///
/// 3. **传递性 / Transitivity**: `a ≤ b` 且 `b ≤ c` 蕴含 `a ≤ c`
///    `a ≤ b` and `b ≤ c` implies `a ≤ c`
///
/// Rust 的 `Ord` trait 已经要求全序，这个 trait 提供额外的语义标记。
/// Rust's `Ord` trait already requires total ordering; this trait provides additional semantic marking.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::TotallyOrdered;
///
/// fn max<T: TotallyOrdered>(a: T, b: T) -> T {
///     if a > b { a } else { b }
/// }
///
/// let result = max(3i32, 5i32);
/// assert_eq!(result, 5);
/// ```
///
/// # 设计说明 / Design Notes
/// 全序在优化问题中很重要：
/// - 线性优化：目标函数的比较需要全序
/// - 物理量计算：大小的比较需要全序
///
/// Total ordering is important in optimization problems:
/// - Linear optimization: objective function comparison requires total ordering
/// - Physical quantity calculations: size comparison requires total ordering
pub trait TotallyOrdered: Clone + Debug + Eq + Ord {
    /// 返回两个值中的较小者 / Return the smaller of two values
    fn min_value(&self, other: &Self) -> Self {
        std::cmp::min(self, other).clone()
    }

    /// 返回两个值中的较大者 / Return the larger of two values
    fn max_value(&self, other: &Self) -> Self {
        std::cmp::max(self, other).clone()
    }

    /// 判断值是否在某个范围内 / Check if value is within a range
    fn is_between(&self, lower: &Self, upper: &Self) -> bool {
        lower <= self && self <= upper
    }

    /// 将值限制在某个范围内 / Clamp value to a range
    fn clamp_value(&self, lower: &Self, upper: &Self) -> Self {
        if self < lower {
            lower.clone()
        } else if self > upper {
            upper.clone()
        } else {
            self.clone()
        }
    }
}

// ============================================================================
// 为类型自动实现 TotallyOrdered
// Auto-implement TotallyOrdered for types
// ============================================================================

/// 为满足约束的类型自动实现 TotallyOrdered
/// Auto-implement TotallyOrdered for types satisfying constraints
impl<T> TotallyOrdered for T where T: Clone + Debug + Eq + Ord {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i32_totally_ordered() {
        let a: i32 = 3;
        let b: i32 = 5;

        // 测试 min/max / Test min/max
        assert_eq!(a.min_value(&b), 3);
        assert_eq!(a.max_value(&b), 5);

        // 测试范围 / Test range
        assert!(a.is_between(&1, &10));
        assert!(!a.is_between(&5, &10));
    }

    #[test]
    fn test_i64_totally_ordered() {
        let a: i64 = 10;
        let b: i64 = 7;

        // 测试比较 / Test comparison
        assert!(a > b);
        assert_eq!(a.min_value(&b), 7);
        assert_eq!(a.max_value(&b), 10);
    }

    #[test]
    fn test_clamp() {
        let value: i32 = 15;
        let lower: i32 = 0;
        let upper: i32 = 10;

        // 测试 clamp / Test clamp
        assert_eq!(value.clamp_value(&lower, &upper), 10);

        let value2: i32 = -5;
        assert_eq!(value2.clamp_value(&lower, &upper), 0);

        let value3: i32 = 5;
        assert_eq!(value3.clamp_value(&lower, &upper), 5);
    }

    #[test]
    fn test_ordering_properties() {
        let a: i32 = 1;
        let b: i32 = 2;
        let c: i32 = 3;

        // 测试传递性 / Test transitivity
        assert!(a < b);
        assert!(b < c);
        assert!(a < c);

        // 测试完全性 / Test totality
        assert!(a <= b || b <= a);
        assert!(a >= b || b >= a);
    }

    #[test]
    fn test_generic_function() {
        fn find_max<T: TotallyOrdered>(values: &[T]) -> Option<T> {
            values.iter().max().cloned()
        }

        let values = vec![3i32, 1, 4, 1, 5, 9, 2, 6];
        assert_eq!(find_max(&values), Some(9));
    }
}
