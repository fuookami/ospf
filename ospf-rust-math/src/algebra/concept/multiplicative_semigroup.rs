//! 乘法半群 trait
//! Multiplicative semigroup trait

use crate::operator::MulRef;
use std::fmt::Debug;
use std::ops::Mul;

// ============================================================================
// MultiplicativeSemigroup Trait - 乘法半群
// ============================================================================

/// MultiplicativeSemigroup - 乘法半群 trait
/// MultiplicativeSemigroup - Multiplicative semigroup trait
///
/// 表示类型在乘法下构成半群，满足以下公理：
/// Represents that a type forms a semigroup under multiplication with the following axioms:
///
/// 1. **封闭性 / Closure**: `a * b` 结果类型相同
///    The result of `a * b` has the same type
///
/// 2. **结合律 / Associativity**: `(a * b) * c = a * (b * c)`
///    The operation is associative
///
/// 乘法半群用于描述可相乘的类型。
/// A multiplicative semigroup describes types that can be multiplied.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::MultiplicativeSemigroup;
///
/// fn multiply<T: MultiplicativeSemigroup>(a: T, b: T) -> T {
///     a * b
/// }
///
/// let result = multiply(2i32, 3i32);
/// assert_eq!(result, 6);
/// ```
///
/// # 设计说明 / Design Notes
/// 对于物理量计算，乘法半群用于描述可以相乘的量（如长度乘长度得到面积）。
/// For physical quantity calculations, multiplicative semigroup describes quantities
/// that can be multiplied (like length times length gives area).
pub trait MultiplicativeSemigroup: Clone + Debug + Mul<Output = Self> {}

// ============================================================================
// 为类型自动实现 MultiplicativeSemigroup
// Auto-implement MultiplicativeSemigroup for types
// ============================================================================

/// 为满足约束的类型自动实现 MultiplicativeSemigroup
/// Auto-implement MultiplicativeSemigroup for types satisfying constraints
impl<T> MultiplicativeSemigroup for T where T: Clone + Debug + Mul<Output = Self> {}

// ============================================================================
// MultiplicativeSemigroupRef - 支持引用操作的乘法半群
// ============================================================================

/// MultiplicativeSemigroupRef - 支持引用操作的乘法半群
/// MultiplicativeSemigroupRef - Multiplicative semigroup with reference operations
///
/// 继承乘法半群性质，并支持引用相乘。
/// Inherits multiplicative semigroup properties and supports reference multiplication.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::MultiplicativeSemigroupRef;
///
/// fn mul_refs<T: MultiplicativeSemigroupRef>(a: &T, b: &T) -> T {
///     T::mul_ref(a, b)
/// }
///
/// let result = mul_refs(&5i32, &3i32);
/// assert_eq!(result, 15);
/// ```
pub trait MultiplicativeSemigroupRef: MultiplicativeSemigroup + MulRef {}

/// 为满足约束的类型自动实现 MultiplicativeSemigroupRef
/// Auto-implement MultiplicativeSemigroupRef for types satisfying constraints
impl<T: MultiplicativeSemigroup + MulRef> MultiplicativeSemigroupRef for T {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i32_multiplicative_semigroup() {
        let a: i32 = 5;
        let b: i32 = 3;

        // 测试封闭性 / Test closure
        let c = a * b;
        assert_eq!(c, 15);

        // 测试结合律 / Test associativity
        let d: i32 = 2;
        assert_eq!((a * b) * d, a * (b * d));
    }

    #[test]
    fn test_i64_multiplicative_semigroup() {
        let a: i64 = 10;
        let b: i64 = 7;

        assert_eq!(a * b, 70);

        // 测试结合律 / Test associativity
        let c: i64 = 3;
        assert_eq!((a * b) * c, a * (b * c));
    }

    #[test]
    fn test_generic_function() {
        fn multiply<T: MultiplicativeSemigroup>(a: T, b: T) -> T {
            a * b
        }

        assert_eq!(multiply(3i32, 4i32), 12);
        assert_eq!(multiply(10i64, 5i64), 50);
    }

    #[test]
    fn test_multiplicative_semigroup_ref() {
        fn mul_refs<T: MultiplicativeSemigroupRef>(a: &T, b: &T) -> T {
            T::mul_ref(a, b)
        }

        let result = mul_refs(&5i32, &3i32);
        assert_eq!(result, 15);
    }
}
