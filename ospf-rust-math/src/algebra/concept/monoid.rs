//! 幺半群 trait
//! Monoid trait

use num_traits::Zero;
use crate::operator::ZeroRef;
use super::Semigroup;
use super::SemigroupRef;

// ============================================================================
// Monoid Trait - 幺半群
// ============================================================================

/// Monoid - 幺半群 trait
/// Monoid - Monoid trait
///
/// 表示类型构成幺半群，满足以下公理：
/// Represents that a type forms a monoid with the following axioms:
///
/// 1. **封闭性 / Closure**: `a + b` 结果类型相同
///    The result of `a + b` has the same type
///
/// 2. **结合律 / Associativity**: `(a + b) + c = a + (b + c)`
///    The operation is associative
///
/// 3. **单位元 / Identity**: `a + 0 = a = 0 + a`
///    There exists an identity element
///
/// 幺半群是半群的扩展，增加了单位元的概念。
/// A monoid is an extension of semigroup, adding the concept of an identity element.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::Monoid;
/// use num_traits::Zero;
///
/// fn sum_with_identity<T: Monoid>(a: T, b: T) -> T {
///     a + b
/// }
///
/// let result = sum_with_identity(1i32, 2i32);
/// assert_eq!(result, 3);
///
/// // 测试单位元 / Test identity
/// assert_eq!(i32::zero() + 5i32, 5i32);
/// ```
///
/// # 设计说明 / Design Notes
/// 对于物理量计算，幺半群结构描述可叠加且有零值的量。
/// For physical quantity calculations, monoid structure describes additive quantities with a zero value.
pub trait Monoid: Semigroup + Zero {}

// ============================================================================
// 为类型自动实现 Monoid
// Auto-implement Monoid for types
// ============================================================================

/// 为满足约束的类型自动实现 Monoid
/// Auto-implement Monoid for types satisfying constraints
impl<T> Monoid for T where T: Semigroup + Zero {}

// ============================================================================
// MonoidRef - 支持引用操作的幺半群
// ============================================================================

/// MonoidRef - 支持引用操作的幺半群
/// MonoidRef - Monoid with reference operations
///
/// 继承幺半群性质，并支持引用操作和零值引用。
/// Inherits monoid properties and supports reference operations with zero reference.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::MonoidRef;
/// use num_traits::Zero;
///
/// fn add_refs<T: MonoidRef>(a: &T, b: &T) -> T {
///     T::add_ref(a, b)
/// }
///
/// let result = add_refs(&5i32, &3i32);
/// assert_eq!(result, 8);
/// ```
pub trait MonoidRef: Monoid + SemigroupRef + ZeroRef {}

/// 为满足约束的类型自动实现 MonoidRef
/// Auto-implement MonoidRef for types satisfying constraints
impl<T: Monoid + SemigroupRef + ZeroRef> MonoidRef for T {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::Zero;

    #[test]
    fn test_i32_monoid() {
        let a: i32 = 5;

        // 测试单位元 / Test identity
        assert_eq!(a + i32::zero(), a);
        assert_eq!(i32::zero() + a, a);

        // 测试结合律（继承自半群）/ Test associativity (inherited from semigroup)
        let b: i32 = 3;
        let c: i32 = 2;
        assert_eq!((a + b) + c, a + (b + c));
    }

    #[test]
    fn test_i64_monoid() {
        let a: i64 = 10;

        // 测试单位元 / Test identity
        assert_eq!(a + i64::zero(), a);
        assert_eq!(i64::zero() + a, a);
    }

    #[test]
    fn test_f64_monoid() {
        let a: f64 = 1.5;

        // 测试单位元 / Test identity
        assert!((a + f64::zero() - a).abs() < 1e-10);
    }

    #[test]
    fn test_generic_function() {
        fn add_with_identity<T: Monoid>(a: T, b: T) -> T {
            a + b
        }

        assert_eq!(add_with_identity(3i32, 4i32), 7);
    }

    #[test]
    fn test_zero_is_identity() {
        fn test_monoid_identity<T: Monoid + PartialEq + Copy>(a: T) {
            assert_eq!(a + T::zero(), a);
        }

        test_monoid_identity(5i32);
        test_monoid_identity(10i64);
    }

    #[test]
    fn test_monoid_ref() {
        fn add_refs<T: MonoidRef>(a: &T, b: &T) -> T {
            T::add_ref(a, b)
        }

        let result = add_refs(&5i32, &3i32);
        assert_eq!(result, 8);
    }
}
