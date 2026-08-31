//! 环 trait
//! Ring trait

use super::{AbelianGroup, MultiplicativeSemigroup};
use super::{AbelianGroupRef, MultiplicativeSemigroupRef};
use num_traits::One;

// ============================================================================
// Ring Trait - 环
// ============================================================================

/// Ring - 环 trait
/// Ring - Ring trait
///
/// 表示类型构成环，满足以下公理：
/// Represents that a type forms a ring with the following axioms:
///
/// **加法公理 / Addition Axioms:**
/// 1. **封闭性 / Closure**: `a + b` 结果类型相同
/// 2. **结合律 / Associativity**: `(a + b) + c = a + (b + c)`
/// 3. **交换律 / Commutativity**: `a + b = b + a`
/// 4. **零元 / Identity**: `a + 0 = a`
/// 5. **负元 / Inverse**: `a + (-a) = 0`
///
/// **乘法公理 / Multiplication Axioms:**
/// 1. **封闭性 / Closure**: `a * b` 结果类型相同
/// 2. **结合律 / Associativity**: `(a * b) * c = a * (b * c)`
/// 3. **单位元 / Identity**: `a * 1 = a`（对于含幺环）
///
/// **分配律 / Distributivity:**
/// - `a * (b + c) = a * b + a * c`
/// - `(a + b) * c = a * c + b * c`
///
/// 环是同时具有加法和乘法运算的代数结构。
/// A ring is an algebraic structure with both addition and multiplication operations.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::Ring;
///
/// fn compute<T: Ring>(a: T, b: T, c: T) -> T {
///     a * (b + c)  // 分配律 / Distributivity
/// }
///
/// let result = compute(2i32, 3i32, 4i32);
/// assert_eq!(result, 14);
/// ```
///
/// # 设计说明 / Design Notes
/// 环结构是线性代数和优化理论的基础。
/// 对于线性优化，约束条件的系数构成环。
/// 对于二次型优化，目标函数的系数构成环。
/// Ring structure is fundamental to linear algebra and optimization theory.
/// For linear optimization, coefficients of constraints form a ring.
/// For quadratic optimization, coefficients of objective functions form a ring.
pub trait Ring: AbelianGroup + MultiplicativeSemigroup + One {}

// ============================================================================
// 为类型自动实现 Ring
// Auto-implement Ring for types
// ============================================================================

/// 为满足约束的类型自动实现 Ring
/// Auto-implement Ring for types satisfying constraints
impl<T> Ring for T where T: AbelianGroup + MultiplicativeSemigroup + One {}

// ============================================================================
// RingRef - 支持引用操作的环
// ============================================================================

/// RingRef - 支持引用操作的环
/// RingRef - Ring with reference operations
///
/// 继承环性质，并支持所有引用操作。
/// Inherits ring properties and supports all reference operations.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::RingRef;
/// use std::ops::Add;
///
/// fn ring_compute<T: RingRef>(a: &T, b: &T, c: &T) -> T
/// where
///     for<'x> &'x T: Add<Output = T>,
/// {
///     // a * b + a * c = a * (b + c)
///     let ab = T::mul_ref(a, b);
///     let ac = T::mul_ref(a, c);
///     T::add_ref(&ab, &ac)
/// }
///
/// let result = ring_compute(&2i32, &3i32, &4i32);
/// assert_eq!(result, 14);  // 2*3 + 2*4 = 6 + 8 = 14
/// ```
pub trait RingRef: Ring + AbelianGroupRef + MultiplicativeSemigroupRef {}

/// 为满足约束的类型自动实现 RingRef
/// Auto-implement RingRef for types satisfying constraints
impl<T: Ring + AbelianGroupRef + MultiplicativeSemigroupRef> RingRef for T {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::Zero;

    #[test]
    fn test_i32_ring() {
        let a: i32 = 2;
        let b: i32 = 3;
        let c: i32 = 4;

        // 测试分配律 / Test distributivity
        assert_eq!(a * (b + c), a * b + a * c);
        assert_eq!((a + b) * c, a * c + b * c);

        // 测试零元 / Test zero
        assert_eq!(a + i32::zero(), a);

        // 测试单位元 / Test one
        assert_eq!(a * i32::one(), a);
    }

    #[test]
    fn test_i64_ring() {
        let a: i64 = 5;
        let b: i64 = 7;

        // 测试分配律 / Test distributivity
        assert_eq!(a * (b + 2), a * b + a * 2);
    }

    #[test]
    fn test_f64_ring() {
        let a: f64 = 2.0;
        let b: f64 = 3.0;
        let c: f64 = 4.0;

        // 测试分配律 / Test distributivity
        assert!((a * (b + c) - (a * b + a * c)).abs() < 1e-10);
    }

    #[test]
    fn test_generic_function() {
        fn compute<T: Ring>(a: T, b: T, c: T) -> T {
            a * (b + c)
        }

        assert_eq!(compute(2i32, 3i32, 4i32), 14);
    }

    #[test]
    fn test_distributivity() {
        fn test_ring_distributivity<T: Ring + PartialEq + Copy>(a: T, b: T, c: T) {
            assert_eq!(a * (b + c), a * b + a * c);
            assert_eq!((a + b) * c, a * c + b * c);
        }

        test_ring_distributivity(2i32, 3i32, 4i32);
        test_ring_distributivity(5i64, 7i64, 2i64);
    }

    #[test]
    fn test_ring_ref() {
        use std::ops::Add;

        fn ring_compute<T: RingRef>(a: &T, b: &T, c: &T) -> T
        where
            for<'x> &'x T: Add<Output = T>,
        {
            // a * b + a * c = a * (b + c)
            let ab = T::mul_ref(a, b);
            let ac = T::mul_ref(a, c);
            T::add_ref(&ab, &ac)
        }

        let result = ring_compute(&2i32, &3i32, &4i32);
        assert_eq!(result, 14); // 2*3 + 2*4 = 6 + 8 = 14
    }
}
