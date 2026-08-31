//! 指数类型约束
//! Exponent type constraint

use num_traits::One;
use crate::algebra::concept::AbelianGroup;

// ============================================================================
// Exponent Trait - 指数类型约束
// ============================================================================

/// Exponent - 指数类型约束
/// Exponent - Exponent type constraint
///
/// 要求类型构成阿贝尔群且具有乘法单位元。
/// Requires the type to form an Abelian group and have a multiplicative identity.
///
/// # 数学背景 / Mathematical Background
///
/// 指数需要满足以下性质：
/// Exponents need to satisfy the following properties:
///
/// 1. **加法封闭 / Additive closure**: `e1 + e2` 结果类型相同
/// 2. **加法交换律 / Additive commutativity**: `e1 + e2 = e2 + e1`
/// 3. **加法结合律 / Additive associativity**: `(e1 + e2) + e3 = e1 + (e2 + e3)`
/// 4. **零元 / Zero**: `e + 0 = e`
/// 5. **负元 / Negative**: `e + (-e) = 0`
/// 6. **单位元 / One**: `e * 1 = e`（用于默认幂次）
///
/// # 适用类型 / Applicable Types
///
/// - `i8`, `i16`, `i32`, `i64`, `i128`
/// - `BigInt`（需要启用相应 feature）
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::operator::Exponent;
///
/// fn double_exponent<E: Exponent + Copy>(e: E) -> E {
///     e + e
/// }
///
/// assert_eq!(double_exponent(3i32), 6);
/// ```
pub trait Exponent: AbelianGroup + One {}

// ============================================================================
// 为类型自动实现 Exponent
// Auto-implement Exponent for types
// ============================================================================

/// 为满足约束的类型自动实现 Exponent
/// Auto-implement Exponent for types satisfying constraints
impl<T> Exponent for T where T: AbelianGroup + One {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i32_exponent() {
        let e: i32 = 5;

        // 测试加法 / Test addition
        assert_eq!(e + 3, 8);

        // 测试零元 / Test zero
        assert_eq!(e + 0, e);

        // 测试单位元 / Test one
        assert_eq!(e * 1, e);

        // 测试负元 / Test negative
        assert_eq!(e + (-e), 0);
    }

    #[test]
    fn test_i64_exponent() {
        let e: i64 = 10;

        // 测试加法 / Test addition
        assert_eq!(e + 5, 15);

        // 测试单位元 / Test one
        assert_eq!(e * 1, e);
    }

    #[test]
    fn test_generic_function() {
        fn add_exponents<E: Exponent>(e1: E, e2: E) -> E {
            e1 + e2
        }

        assert_eq!(add_exponents(3i32, 4i32), 7);
        assert_eq!(add_exponents(10i64, 5i64), 15);
    }

    #[test]
    fn test_exponent_properties() {
        fn test_exponent_properties<E: Exponent + PartialEq + Copy>(e: E) {
            // 交换律 / Commutativity
            assert_eq!(e + E::zero(), e);
            assert_eq!(E::zero() + e, e);

            // 单位元 / Identity
            assert_eq!(e * E::one(), e);
        }

        test_exponent_properties(5i32);
        test_exponent_properties(10i64);
    }
}
