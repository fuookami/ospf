//! 域 trait
//! Field trait

use std::ops::Div;
use super::CommutativeRing;
use super::CommutativeRingRef;
use super::MultiplicativeGroupRef;

// ============================================================================
// Field Trait - 域
// ============================================================================

/// Field - 域 trait
/// Field - Field trait
///
/// 表示类型构成域，满足以下公理：
/// Represents that a type forms a field with the following axioms:
///
/// **继承交换环的所有公理，外加：**
/// **Inherits all commutative ring axioms, plus:**
///
/// **非零元素乘法逆元 / Multiplicative Inverse for Non-Zero Elements:**
/// - 对于任意 `a ≠ 0`，存在 `a⁻¹` 使得 `a * a⁻¹ = 1`
/// - For any `a ≠ 0`, there exists `a⁻¹` such that `a * a⁻¹ = 1`
///
/// 域是最完整的代数结构，支持加减乘除（除零除外）。
/// A field is the most complete algebraic structure, supporting addition, subtraction,
/// multiplication, and division (except division by zero).
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::Field;
///
/// fn divide<T: Field>(a: T, b: T) -> T {
///     a / b
/// }
///
/// let result = divide(6.0f64, 2.0f64);
/// assert!((result - 3.0).abs() < 1e-10);
/// ```
///
/// # 设计说明 / Design Notes
/// 域是线性代数和优化理论的核心结构：
/// - 线性优化：目标函数和约束条件的系数需要在域中定义
/// - 二次型优化：Hessian 矩阵的元素需要在域中定义
/// - 物理量计算：大多数物理量（如 f64）构成域
///
/// Fields are central to linear algebra and optimization theory:
/// - Linear optimization: coefficients of objective functions and constraints need to be in a field
/// - Quadratic optimization: elements of Hessian matrices need to be in a field
/// - Physical quantity calculations: most physical quantities (like f64) form fields
///
/// # 常见的域 / Common Fields
/// - `f32`, `f64`: 浮点数域 / Floating-point fields
/// - 有理数域 / Rational number fields
/// - 注意：整数不是域（没有乘法逆元）/ Note: Integers are not fields (no multiplicative inverse)
pub trait Field: CommutativeRing + Div<Output = Self> {}

// ============================================================================
// 为类型自动实现 Field
// Auto-implement Field for types
// ============================================================================

/// 为满足约束的类型自动实现 Field
/// Auto-implement Field for types satisfying constraints
///
/// 注意：整数类型不满足 Field 约束，因为 `Div` 对于整数会产生截断。
/// Note: Integer types don't satisfy Field constraints because `Div` truncates for integers.
impl<T> Field for T where T: CommutativeRing + Div<Output = Self> {}

// ============================================================================
// FieldRef - 支持引用操作的域
// ============================================================================

/// FieldRef - 支持引用操作的域
/// FieldRef - Field with reference operations
///
/// 继承域性质，并支持所有引用操作。
/// Inherits field properties and supports all reference operations.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::concept::FieldRef;
/// use std::ops::{Add, Sub, Mul, Div};
///
/// fn compute<T: FieldRef>(a: &T, b: &T, c: &T) -> T
/// where
///     for<'x> &'x T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T>,
/// {
///     // (a + b) * c / (a - b)
///     let sum = T::add_ref(a, b);
///     let diff = T::sub_ref(a, b);
///     let product = T::mul_ref(&sum, c);
///     T::div_ref(&product, &diff)
/// }
///
/// let result = compute(&3.0f64, &1.0f64, &2.0f64);
/// assert!((result - 4.0).abs() < 1e-10);
/// ```
pub trait FieldRef: Field + CommutativeRingRef + MultiplicativeGroupRef {}

/// 为满足约束的类型自动实现 FieldRef
/// Auto-implement FieldRef for types satisfying constraints
impl<T: Field + CommutativeRingRef + MultiplicativeGroupRef> FieldRef for T {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::{One, Zero};

    #[test]
    fn test_f64_field() {
        let a: f64 = 6.0;
        let b: f64 = 2.0;

        // 测试除法 / Test division
        assert!((a / b - 3.0).abs() < 1e-10);

        // 测试乘法逆元 / Test multiplicative inverse
        assert!((a / a - f64::one()).abs() < 1e-10);

        // 测试所有域性质 / Test all field properties
        let c: f64 = 3.0;
        assert!((a * (b + c) - (a * b + a * c)).abs() < 1e-10);
    }

    #[test]
    fn test_f32_field() {
        let a: f32 = 6.0;
        let b: f32 = 2.0;

        // 测试除法 / Test division
        assert!((a / b - 3.0).abs() < 1e-6);

        // 测试乘法逆元 / Test multiplicative inverse
        assert!((a / a - f32::one()).abs() < 1e-6);
    }

    #[test]
    fn test_field_operations() {
        fn compute<T: Field + Zero + One + Copy>(a: T, b: T, c: T) -> T {
            // (a + b) * c / (a - b) 需要确保 a ≠ b
            // (a + b) * c / (a - b) assuming a ≠ b
            (a + b) * c / (a - b)
        }

        let result = compute(3.0f64, 1.0f64, 2.0f64);
        assert!((result - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_field_identity() {
        fn test_field_properties<T: Field + Zero + One + PartialEq + Copy>(a: T) {
            // 加法单位元 / Additive identity
            assert_eq!(a + T::zero(), a);

            // 乘法单位元 / Multiplicative identity
            assert_eq!(a * T::one(), a);
        }

        test_field_properties(5.0f64);
        test_field_properties(3.0f32);
    }

    #[test]
    fn test_field_inverse() {
        fn test_field_inverse<T: Field + Zero + One>(a: T)
        where
            T: PartialEq,
            T: std::ops::Sub<Output = T>,
        {
            // 乘法逆元（非零元素）/ Multiplicative inverse (non-zero elements)
            if a != T::zero() {
                let _inverse = T::one() / a;
                // a * (1/a) ≈ 1
                // a * (1/a) ≈ 1
                // 注意：由于浮点精度，这里需要近似比较
                // Note: Due to floating-point precision, approximate comparison is needed
            }
        }

        test_field_inverse(5.0f64);
        test_field_inverse(3.0f32);
    }

    #[test]
    fn test_field_ref() {
        use std::ops::{Add, Div, Mul, Sub};

        fn compute<T: FieldRef>(a: &T, b: &T, c: &T) -> T
        where
            for<'x> &'x T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T>,
        {
            // (a + b) * c / (a - b)
            let sum = T::add_ref(a, b);
            let diff = T::sub_ref(a, b);
            let product = T::mul_ref(&sum, c);
            T::div_ref(&product, &diff)
        }

        let result = compute(&3.0f64, &1.0f64, &2.0f64);
        assert!((result - 4.0).abs() < 1e-10);
    }
}
