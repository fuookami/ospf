//! 群 trait
//! Group trait

use super::Monoid;
use super::MonoidRef;
use crate::operator::{NegRef, SubRef};
use std::ops::{Neg, Sub};

// ============================================================================
// Group Trait - 群
// ============================================================================

/// Group - 群 trait
/// Group - Group trait
///
/// 表示类型构成群，满足以下公理：
/// Represents that a type forms a group with the following axioms:
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
/// 4. **逆元 / Inverse**: `a + (-a) = 0`
///    Every element has an inverse
///
/// 群是幺半群的扩展，增加了逆元的概念。
/// A group is an extension of a monoid, adding the concept of an inverse element.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::Group;
///
/// fn subtract<T: Group>(a: T, b: T) -> T {
///     a - b
/// }
///
/// let result = subtract(5i32, 3i32);
/// assert_eq!(result, 2);
/// ```
///
/// # 设计说明 / Design Notes
/// 对于物理量计算，群结构描述可加减且有零值的量（如位移、速度）。
/// For physical quantity calculations, group structure describes additive quantities
/// with zero value (like displacement, velocity).
pub trait Group: Monoid + Sub<Output = Self> + Neg<Output = Self> {}

// ============================================================================
// 为类型自动实现 Group
// Auto-implement Group for types
// ============================================================================

/// 为满足约束的类型自动实现 Group
/// Auto-implement Group for types satisfying constraints
impl<T> Group for T where T: Monoid + Sub<Output = Self> + Neg<Output = Self> {}

// ============================================================================
// GroupRef - 支持引用操作的群
// ============================================================================

/// GroupRef - 支持引用操作的群
/// GroupRef - Group with reference operations
///
/// 继承群性质，并支持引用相加、相减和取负。
/// Inherits group properties and supports reference add, sub, and neg.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::GroupRef;
///
/// fn sub_refs<T: GroupRef>(a: &T, b: &T) -> T {
///     T::sub_ref(a, b)
/// }
///
/// fn neg_ref<T: GroupRef>(a: &T) -> T {
///     T::neg_ref(a)
/// }
///
/// let result = sub_refs(&5i32, &3i32);
/// assert_eq!(result, 2);
///
/// let negated = neg_ref(&5i32);
/// assert_eq!(negated, -5);
/// ```
pub trait GroupRef: Group + MonoidRef + SubRef + NegRef {}

/// 为满足约束的类型自动实现 GroupRef
/// Auto-implement GroupRef for types satisfying constraints
impl<T: Group + MonoidRef + SubRef + NegRef> GroupRef for T {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::Zero;

    #[test]
    fn test_i32_group() {
        let a: i32 = 5;

        // 测试逆元 / Test inverse
        assert_eq!(a + (-a), i32::zero());
        assert_eq!(a - a, i32::zero());

        // 测试减法 / Test subtraction
        let b: i32 = 3;
        assert_eq!(a - b, 2);
    }

    #[test]
    fn test_i64_group() {
        let a: i64 = 10;

        // 测试逆元 / Test inverse
        assert_eq!(a + (-a), i64::zero());

        // 测试减法 / Test subtraction
        let b: i64 = 7;
        assert_eq!(a - b, 3);
    }

    #[test]
    fn test_f64_group() {
        let a: f64 = 1.5;

        // 测试逆元 / Test inverse
        assert!((a + (-a)).abs() < 1e-10);

        // 测试减法 / Test subtraction
        let b: f64 = 0.5;
        assert!((a - b - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_generic_function() {
        fn subtract<T: Group>(a: T, b: T) -> T {
            a - b
        }

        assert_eq!(subtract(5i32, 3i32), 2);
        assert_eq!(subtract(10i64, 7i64), 3);
    }

    #[test]
    fn test_inverse_property() {
        fn test_group_inverse<T: Group + PartialEq + Copy>(a: T) {
            assert_eq!(a + (-a), T::zero());
            assert_eq!(a - a, T::zero());
        }

        test_group_inverse(5i32);
        test_group_inverse(10i64);
    }

    #[test]
    fn test_group_ref() {
        fn sub_refs<T: GroupRef>(a: &T, b: &T) -> T {
            T::sub_ref(a, b)
        }

        fn neg_ref<T: GroupRef>(a: &T) -> T {
            T::neg_ref(a)
        }

        let result = sub_refs(&5i32, &3i32);
        assert_eq!(result, 2);

        let negated = neg_ref(&5i32);
        assert_eq!(negated, -5);
    }
}
