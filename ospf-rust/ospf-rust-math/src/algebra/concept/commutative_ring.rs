//! 交换环 trait
//! Commutative ring trait

use super::Ring;
use super::RingRef;

// ============================================================================
// CommutativeRing Trait - 交换环
// ============================================================================

/// CommutativeRing - 交换环 trait
/// CommutativeRing - Commutative ring trait
///
/// 表示类型构成交换环，满足以下公理：
/// Represents that a type forms a commutative ring with the following axioms:
///
/// **继承环的所有公理，外加：**
/// **Inherits all ring axioms, plus:**
///
/// **乘法交换律 / Multiplicative Commutativity:**
/// - `a * b = b * a`
///
/// 交换环是乘法满足交换律的环。
/// A commutative ring is a ring where multiplication is commutative.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::CommutativeRing;
///
/// fn multiply<T: CommutativeRing>(a: T, b: T) -> T {
///     a * b
/// }
///
/// // 测试交换律 / Test commutativity
/// assert_eq!(multiply(2i32, 3i32), multiply(3i32, 2i32));
/// ```
///
/// # 设计说明 / Design Notes
/// 对于物理量计算，大多数标量类型（如 f64, i32）都构成交换环。
/// 多项式系数环也是交换环，这对于优化问题中的多项式求解很重要。
/// For physical quantity calculations, most scalar types (like f64, i32) form commutative rings.
/// Polynomial coefficient rings are also commutative, which is important for polynomial
/// solving in optimization problems.
pub trait CommutativeRing: Ring {}

// ============================================================================
// 为类型自动实现 CommutativeRing
// Auto-implement CommutativeRing for types
// ============================================================================

/// 为满足约束的类型自动实现 CommutativeRing
/// Auto-implement CommutativeRing for types satisfying constraints
///
/// 注意：Rust 的 `Mul` trait 默认支持交换律（对于数值类型）。
/// Note: Rust's `Mul` trait by default supports commutativity (for numeric types).
impl<T> CommutativeRing for T where T: Ring {}

// ============================================================================
// CommutativeRingRef - 支持引用操作的交换环
// ============================================================================

/// CommutativeRingRef - 支持引用操作的交换环
/// CommutativeRingRef - Commutative ring with reference operations
///
/// 继承交换环性质，并支持所有引用操作。
/// Inherits commutative ring properties and supports all reference operations.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::CommutativeRingRef;
///
/// fn mul_refs<T: CommutativeRingRef>(a: &T, b: &T) -> T {
///     T::mul_ref(a, b)
/// }
///
/// let result = mul_refs(&5i32, &3i32);
/// assert_eq!(result, 15);
/// ```
pub trait CommutativeRingRef: CommutativeRing + RingRef {}

/// 为满足约束的类型自动实现 CommutativeRingRef
/// Auto-implement CommutativeRingRef for types satisfying constraints
impl<T: CommutativeRing + RingRef> CommutativeRingRef for T {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i32_commutative_ring() {
        let a: i32 = 2;
        let b: i32 = 3;

        // 测试乘法交换律 / Test multiplication commutativity
        assert_eq!(a * b, b * a);

        // 测试所有环性质 / Test all ring properties
        let c: i32 = 4;
        assert_eq!(a * (b + c), a * b + a * c);
    }

    #[test]
    fn test_i64_commutative_ring() {
        let a: i64 = 5;
        let b: i64 = 7;

        // 测试乘法交换律 / Test multiplication commutativity
        assert_eq!(a * b, b * a);
    }

    #[test]
    fn test_f64_commutative_ring() {
        let a: f64 = 2.0;
        let b: f64 = 3.0;

        // 测试乘法交换律 / Test multiplication commutativity
        assert!((a * b - b * a).abs() < 1e-10);
    }

    #[test]
    fn test_generic_function() {
        fn multiply<T: CommutativeRing>(a: T, b: T) -> T {
            a * b
        }

        assert_eq!(multiply(2i32, 3i32), 6);
        assert_eq!(multiply(3i32, 2i32), 6);
    }

    #[test]
    fn test_multiplication_commutativity() {
        fn test_commutative_ring_commutativity<T: CommutativeRing + PartialEq + Copy>(a: T, b: T) {
            assert_eq!(a * b, b * a);
        }

        test_commutative_ring_commutativity(2i32, 3i32);
        test_commutative_ring_commutativity(5i64, 7i64);
    }

    #[test]
    fn test_commutative_ring_ref() {
        fn mul_refs<T: CommutativeRingRef>(a: &T, b: &T) -> T {
            T::mul_ref(a, b)
        }

        let result = mul_refs(&5i32, &3i32);
        assert_eq!(result, 15);
    }
}
