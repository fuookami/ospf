//! 乘法幺半群 trait
//! Multiplicative monoid trait

use super::MultiplicativeSemigroup;
use super::MultiplicativeSemigroupRef;
use crate::operator::OneRef;
use num_traits::One;

// ============================================================================
// MultiplicativeMonoid Trait - 乘法幺半群
// ============================================================================

/// MultiplicativeMonoid - 乘法幺半群 trait
/// MultiplicativeMonoid - Multiplicative monoid trait
///
/// 表示类型在乘法下构成幺半群，满足以下公理：
/// Represents that a type forms a monoid under multiplication with the following axioms:
///
/// 1. **封闭性 / Closure**: `a * b` 结果类型相同
///    The result of `a * b` has the same type
///
/// 2. **结合律 / Associativity**: `(a * b) * c = a * (b * c)`
///    The operation is associative
///
/// 3. **单位元 / Identity**: `a * 1 = a = 1 * a`
///    There exists an identity element (one)
///
/// 乘法幺半群用于描述可相乘且有单位元的类型。
/// A multiplicative monoid describes types that can be multiplied and have an identity element.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::MultiplicativeMonoid;
/// use num_traits::One;
///
/// fn multiply<T: MultiplicativeMonoid>(a: T, b: T) -> T {
///     a * b
/// }
///
/// let result = multiply(2i32, 3i32);
/// assert_eq!(result, 6);
///
/// // 测试单位元 / Test identity
/// assert_eq!(i32::one() * 5i32, 5i32);
/// ```
///
/// # 设计说明 / Design Notes
/// 对于物理量计算，乘法幺半群用于描述无量纲的物理量（如比例系数）。
/// For physical quantity calculations, multiplicative monoid describes dimensionless
/// physical quantities (like scale factors).
pub trait MultiplicativeMonoid: MultiplicativeSemigroup + One {}

// ============================================================================
// 为类型自动实现 MultiplicativeMonoid
// Auto-implement MultiplicativeMonoid for types
// ============================================================================

/// 为满足约束的类型自动实现 MultiplicativeMonoid
/// Auto-implement MultiplicativeMonoid for types satisfying constraints
impl<T> MultiplicativeMonoid for T where T: MultiplicativeSemigroup + One {}

// ============================================================================
// MultiplicativeMonoidRef - 支持引用操作的乘法幺半群
// ============================================================================

/// MultiplicativeMonoidRef - 支持引用操作的乘法幺半群
/// MultiplicativeMonoidRef - Multiplicative monoid with reference operations
///
/// 继承乘法幺半群性质，并支持引用操作和单位元引用。
/// Inherits multiplicative monoid properties and supports reference operations with one reference.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::MultiplicativeMonoidRef;
///
/// fn mul_refs<T: MultiplicativeMonoidRef>(a: &T, b: &T) -> T {
///     T::mul_ref(a, b)
/// }
///
/// let result = mul_refs(&5i32, &3i32);
/// assert_eq!(result, 15);
/// ```
pub trait MultiplicativeMonoidRef:
    MultiplicativeMonoid + MultiplicativeSemigroupRef + OneRef
{
}

/// 为满足约束的类型自动实现 MultiplicativeMonoidRef
/// Auto-implement MultiplicativeMonoidRef for types satisfying constraints
impl<T: MultiplicativeMonoid + MultiplicativeSemigroupRef + OneRef> MultiplicativeMonoidRef for T {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::One;

    #[test]
    fn test_i32_multiplicative_monoid() {
        let a: i32 = 5;

        // 测试单位元 / Test identity
        assert_eq!(a * i32::one(), a);
        assert_eq!(i32::one() * a, a);

        // 测试结合律（继承自半群）/ Test associativity (inherited from semigroup)
        let b: i32 = 3;
        let c: i32 = 2;
        assert_eq!((a * b) * c, a * (b * c));
    }

    #[test]
    fn test_i64_multiplicative_monoid() {
        let a: i64 = 10;

        // 测试单位元 / Test identity
        assert_eq!(a * i64::one(), a);
        assert_eq!(i64::one() * a, a);
    }

    #[test]
    fn test_f64_multiplicative_monoid() {
        let a: f64 = 1.5;

        // 测试单位元 / Test identity
        assert!((a * f64::one() - a).abs() < 1e-10);
    }

    #[test]
    fn test_generic_function() {
        fn multiply<T: MultiplicativeMonoid>(a: T, b: T) -> T {
            a * b
        }

        assert_eq!(multiply(3i32, 4i32), 12);
    }

    #[test]
    fn test_one_is_identity() {
        fn test_monoid_identity<T: MultiplicativeMonoid + PartialEq + Copy>(a: T) {
            assert_eq!(a * T::one(), a);
        }

        test_monoid_identity(5i32);
        test_monoid_identity(10i64);
    }

    #[test]
    fn test_multiplicative_monoid_ref() {
        fn mul_refs<T: MultiplicativeMonoidRef>(a: &T, b: &T) -> T {
            T::mul_ref(a, b)
        }

        let result = mul_refs(&5i32, &3i32);
        assert_eq!(result, 15);
    }
}
