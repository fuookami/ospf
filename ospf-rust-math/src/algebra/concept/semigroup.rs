//! 半群 trait
//! Semigroup trait

use crate::operator::AddRef;
use std::fmt::Debug;
use std::ops::Add;

// ============================================================================
// Semigroup Trait - 半群
// ============================================================================

/// Semigroup - 半群 trait
/// Semigroup - Semigroup trait
///
/// 表示类型构成半群，满足以下公理：
/// Represents that a type forms a semigroup with the following axioms:
///
/// 1. **封闭性 / Closure**: `a + b` 结果类型相同
///    The result of `a + b` has the same type
///
/// 2. **结合律 / Associativity**: `(a + b) + c = a + (b + c)`
///    The operation is associative
///
/// 半群是最简单的代数结构，只要求封闭性和结合律。
/// A semigroup is the simplest algebraic structure, requiring only closure and associativity.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::Semigroup;
///
/// fn combine<T: Semigroup>(a: T, b: T) -> T {
///     a + b
/// }
///
/// let result = combine(1i32, 2i32);
/// assert_eq!(result, 3);
/// ```
///
/// # 设计说明 / Design Notes
/// 对于物理量计算，半群结构用于描述可叠加的量（如长度、质量）。
/// For physical quantity calculations, semigroup structure describes additive quantities
/// (like length, mass).
pub trait Semigroup: Clone + Debug + Add<Output = Self> {}

// ============================================================================
// 为类型自动实现 Semigroup
// Auto-implement Semigroup for types
// ============================================================================

/// 为满足约束的类型自动实现 Semigroup
/// Auto-implement Semigroup for types satisfying constraints
impl<T> Semigroup for T where T: Clone + Debug + Add<Output = Self> {}

// ============================================================================
// SemigroupRef - 支持引用操作的半群
// ============================================================================

/// SemigroupRef - 支持引用操作的半群
/// SemigroupRef - Semigroup with reference operations
///
/// 继承半群性质，并支持引用相加。
/// Inherits semigroup properties and supports reference addition.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::SemigroupRef;
///
/// fn add_refs<T: SemigroupRef>(a: &T, b: &T) -> T {
///     T::add_ref(a, b)
/// }
///
/// let result = add_refs(&5i32, &3i32);
/// assert_eq!(result, 8);
/// ```
pub trait SemigroupRef: Semigroup + AddRef {}

/// 为满足约束的类型自动实现 SemigroupRef
/// Auto-implement SemigroupRef for types satisfying constraints
impl<T: Semigroup + AddRef> SemigroupRef for T {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i32_semigroup() {
        let a: i32 = 5;
        let b: i32 = 3;

        // 测试封闭性 / Test closure
        let c = a + b;
        assert_eq!(c, 8);

        // 测试结合律 / Test associativity
        let d = 2i32;
        assert_eq!((a + b) + d, a + (b + d));
    }

    #[test]
    fn test_i64_semigroup() {
        let a: i64 = 10;
        let b: i64 = 7;

        assert_eq!(a + b, 17);

        // 测试结合律 / Test associativity
        let c = 3i64;
        assert_eq!((a + b) + c, a + (b + c));
    }

    #[test]
    fn test_generic_function() {
        fn combine<T: Semigroup>(a: T, b: T) -> T {
            a + b
        }

        assert_eq!(combine(3i32, 4i32), 7);
        assert_eq!(combine(10i64, 5i64), 15);
    }

    #[test]
    fn test_semigroup_ref() {
        fn add_refs<T: SemigroupRef>(a: &T, b: &T) -> T {
            T::add_ref(a, b)
        }

        let result = add_refs(&5i32, &3i32);
        assert_eq!(result, 8);
    }
}
