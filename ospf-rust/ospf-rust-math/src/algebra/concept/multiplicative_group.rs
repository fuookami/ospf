//! 乘法群 trait
//! Multiplicative group trait

use super::MultiplicativeMonoid;
use super::MultiplicativeMonoidRef;
use crate::operator::{DivRef, NegOneRef};
use std::ops::Div;

// ============================================================================
// MultiplicativeGroup Trait - 乘法群
// ============================================================================

/// MultiplicativeGroup - 乘法群 trait
/// MultiplicativeGroup - Multiplicative group trait
///
/// 表示类型在乘法下构成群（非零元素），满足以下公理：
/// Represents that a type forms a group under multiplication (non-zero elements) with the following axioms:
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
/// 4. **逆元 / Inverse**: `a * a⁻¹ = 1`
///    Every (non-zero) element has an inverse
///
/// 注意：对于乘法群，零元素没有逆元。
/// Note: For multiplicative groups, zero has no inverse.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::MultiplicativeGroup;
///
/// fn divide<T: MultiplicativeGroup>(a: T, b: T) -> T {
///     a / b
/// }
///
/// let result = divide(6i32, 2i32);
/// assert_eq!(result, 3);
/// ```
///
/// # 设计说明 / Design Notes
/// 对于物理量计算，乘法群用于描述可乘除的类型（如无量纲比例）。
/// 对于优化问题，乘法群结构支持矩阵求逆等操作。
/// For physical quantity calculations, multiplicative group describes types that can be
/// multiplied and divided (like dimensionless ratios). For optimization problems,
/// multiplicative group structure supports operations like matrix inversion.
pub trait MultiplicativeGroup: MultiplicativeMonoid + Div<Output = Self> {}

// ============================================================================
// 为类型自动实现 MultiplicativeGroup
// Auto-implement MultiplicativeGroup for types
// ============================================================================

/// 为满足约束的类型自动实现 MultiplicativeGroup
/// Auto-implement MultiplicativeGroup for types satisfying constraints
impl<T> MultiplicativeGroup for T where T: MultiplicativeMonoid + Div<Output = Self> {}

// ============================================================================
// MultiplicativeGroupRef - 支持引用操作的乘法群
// ============================================================================

/// MultiplicativeGroupRef - 支持引用操作的乘法群
/// MultiplicativeGroupRef - Multiplicative group with reference operations
///
/// 继承乘法群性质，并支持引用相乘、相除和负单位元引用。
/// Inherits multiplicative group properties and supports reference mul, div, and neg one reference.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::MultiplicativeGroupRef;
///
/// fn div_refs<T: MultiplicativeGroupRef>(a: &T, b: &T) -> T {
///     T::div_ref(a, b)
/// }
///
/// let result = div_refs(&6.0f64, &2.0f64);
/// assert!((result - 3.0).abs() < 1e-10);
/// ```
pub trait MultiplicativeGroupRef:
    MultiplicativeGroup + MultiplicativeMonoidRef + DivRef + NegOneRef
{
}

/// 为满足约束的类型自动实现 MultiplicativeGroupRef
/// Auto-implement MultiplicativeGroupRef for types satisfying constraints
impl<T: MultiplicativeGroup + MultiplicativeMonoidRef + DivRef + NegOneRef> MultiplicativeGroupRef
    for T
{
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::One;

    #[test]
    fn test_i32_multiplicative_group() {
        let a: i32 = 6;
        let b: i32 = 2;

        // 测试除法 / Test division
        assert_eq!(a / b, 3);

        // 测试单位元 / Test identity
        assert_eq!(a * i32::one(), a);

        // 测试逆元性质（整数需特殊处理）/ Test inverse property
        // 对于整数类型，除法可能产生截断
        // For integer types, division may truncate
        assert_eq!((a / b) * b, a);
    }

    #[test]
    fn test_i64_multiplicative_group() {
        let a: i64 = 10;
        let b: i64 = 5;

        // 测试除法 / Test division
        assert_eq!(a / b, 2);
    }

    #[test]
    fn test_f64_multiplicative_group() {
        let a: f64 = 6.0;
        let b: f64 = 2.0;

        // 测试除法 / Test division
        assert!((a / b - 3.0).abs() < 1e-10);

        // 测试逆元 / Test inverse
        assert!((a / a - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_generic_function() {
        fn divide<T: MultiplicativeGroup>(a: T, b: T) -> T {
            a / b
        }

        assert_eq!(divide(6i32, 2i32), 3);
        assert_eq!(divide(10i64, 5i64), 2);
    }

    #[test]
    fn test_inverse_property() {
        fn test_multiplicative_inverse<T: MultiplicativeGroup + PartialEq + Copy>(a: T, b: T) {
            // a / b * b = a (当 a 能被 b 整除时)
            // a / b * b = a (when a is divisible by b)
            assert_eq!((a / b) * b, a);
        }

        test_multiplicative_inverse(6i32, 2i32);
        test_multiplicative_inverse(15i64, 3i64);
    }

    #[test]
    fn test_multiplicative_group_ref() {
        fn div_refs<T: MultiplicativeGroupRef>(a: &T, b: &T) -> T {
            T::div_ref(a, b)
        }

        let result = div_refs(&6.0f64, &2.0f64);
        assert!((result - 3.0).abs() < 1e-10);
    }
}
