//! 阿贝尔群（交换群）trait
//! Abelian group (commutative group) trait

use super::Group;
use super::GroupRef;

// ============================================================================
// AbelianGroup Trait - 阿贝尔群
// ============================================================================

/// AbelianGroup - 阿贝尔群 trait
/// AbelianGroup - Abelian group trait
///
/// 表示类型构成阿贝尔群（交换群），满足以下公理：
/// Represents that a type forms an Abelian group (commutative group) with the following axioms:
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
/// 5. **交换律 / Commutativity**: `a + b = b + a`
///    The operation is commutative
///
/// 阿贝尔群是群的扩展，要求运算满足交换律。
/// An Abelian group is an extension of a group, requiring the operation to be commutative.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::AbelianGroup;
///
/// fn add<T: AbelianGroup>(a: T, b: T) -> T {
///     a + b
/// }
///
/// let result = add(1i32, 2i32);
/// assert_eq!(result, 3);
///
/// // 测试交换律 / Test commutativity
/// assert_eq!(add(3i32, 5i32), add(5i32, 3i32));
/// ```
///
/// # 设计说明 / Design Notes
/// 对于物理量计算，阿贝尔群结构描述可加减、有零值且加法可交换的量。
/// 大多数物理量（长度、质量、时间、速度等）都构成阿贝尔群。
/// For physical quantity calculations, Abelian group structure describes additive quantities
/// with zero value and commutative addition. Most physical quantities (length, mass, time,
/// velocity, etc.) form Abelian groups.
///
/// 这与原来的 `AdditiveGroup` 等价，建议使用新的 `AbelianGroup` 名称。
/// This is equivalent to the original `AdditiveGroup`, recommend using the new `AbelianGroup` name.
pub trait AbelianGroup: Group {}

// ============================================================================
// 为类型自动实现 AbelianGroup
// Auto-implement AbelianGroup for types
// ============================================================================

/// 为满足约束的类型自动实现 AbelianGroup
/// Auto-implement AbelianGroup for types satisfying constraints
///
/// 注意：Rust 的 `Add` trait 默认支持交换律（对于数值类型）。
/// Note: Rust's `Add` trait by default supports commutativity (for numeric types).
impl<T> AbelianGroup for T where T: Group {}

// ============================================================================
// AbelianGroupRef - 支持引用操作的阿贝尔群
// ============================================================================

/// AbelianGroupRef - 支持引用操作的阿贝尔群
/// AbelianGroupRef - Abelian group with reference operations
///
/// 继承阿贝尔群性质，并支持所有引用操作。
/// Inherits abelian group properties and supports all reference operations.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::AbelianGroupRef;
///
/// fn add_refs<T: AbelianGroupRef>(a: &T, b: &T) -> T {
///     T::add_ref(a, b)
/// }
///
/// let result = add_refs(&5i32, &3i32);
/// assert_eq!(result, 8);
/// ```
pub trait AbelianGroupRef: AbelianGroup + GroupRef {}

/// 为满足约束的类型自动实现 AbelianGroupRef
/// Auto-implement AbelianGroupRef for types satisfying constraints
impl<T: AbelianGroup + GroupRef> AbelianGroupRef for T {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::Zero;

    #[test]
    fn test_i32_abelian_group() {
        let a: i32 = 5;
        let b: i32 = 3;

        // 测试交换律 / Test commutativity
        assert_eq!(a + b, b + a);

        // 测试所有群性质 / Test all group properties
        assert_eq!(a + i32::zero(), a);
        assert_eq!(a + (-a), i32::zero());
    }

    #[test]
    fn test_i64_abelian_group() {
        let a: i64 = 10;
        let b: i64 = 7;

        // 测试交换律 / Test commutativity
        assert_eq!(a + b, b + a);
    }

    #[test]
    fn test_f64_abelian_group() {
        let a: f64 = 1.5;
        let b: f64 = 2.5;

        // 测试交换律 / Test commutativity
        assert!((a + b - (b + a)).abs() < 1e-10);
    }

    #[test]
    fn test_generic_function() {
        fn add<T: AbelianGroup>(a: T, b: T) -> T {
            a + b
        }

        assert_eq!(add(3i32, 4i32), 7);

        // 验证交换律 / Verify commutativity
        assert_eq!(add(3i32, 5i32), add(5i32, 3i32));
    }

    #[test]
    fn test_commutativity() {
        fn test_abelian_commutativity<T: AbelianGroup + PartialEq + Copy>(a: T, b: T) {
            assert_eq!(a + b, b + a);
        }

        test_abelian_commutativity(5i32, 3i32);
        test_abelian_commutativity(10i64, 7i64);
    }

    #[test]
    fn test_abelian_group_ref() {
        fn add_refs<T: AbelianGroupRef>(a: &T, b: &T) -> T {
            T::add_ref(a, b)
        }

        let result = add_refs(&5i32, &3i32);
        assert_eq!(result, 8);
    }
}
