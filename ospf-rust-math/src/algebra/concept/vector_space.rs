//! 向量空间 trait
//! Vector space trait

use super::Field;
use num_traits::Zero;
use std::fmt::Debug;
use std::ops::{Add, Neg, Sub};

// ============================================================================
// VectorSpace Trait - 向量空间
// ============================================================================

/// VectorSpace - 向量空间 trait
/// VectorSpace - Vector space trait
///
/// 表示类型在给定标量域上构成向量空间。
/// Represents that a type forms a vector space over a given scalar field.
///
/// 向量空间满足以下公理：
/// A vector space satisfies the following axioms:
///
/// **向量加法公理 / Vector Addition Axioms:**
/// 1. **封闭性 / Closure**: `u + v` 结果类型相同
/// 2. **结合律 / Associativity**: `(u + v) + w = u + (v + w)`
/// 3. **交换律 / Commutativity**: `u + v = v + u`
/// 4. **零向量 / Zero**: `v + 0 = v`
/// 5. **负向量 / Inverse**: `v + (-v) = 0`
///
/// **标量乘法公理 / Scalar Multiplication Axioms:**
/// 1. **封闭性 / Closure**: `a * v` 结果类型相同
/// 2. **分配律 1 / Distributivity 1**: `a * (u + v) = a * u + a * v`
/// 3. **分配律 2 / Distributivity 2**: `(a + b) * v = a * v + b * v`
/// 4. **结合律 / Associativity**: `a * (b * v) = (a * b) * v`
/// 5. **单位元 / Identity**: `1 * v = v`
///
/// # 泛型关联类型 (GAT) / Generic Associated Type (GAT)
/// 使用 GAT 来定义标量类型，允许更灵活的向量空间定义。
/// Uses GAT to define the scalar type, allowing more flexible vector space definitions.
///
/// # 示例 / Examples
/// ```
/// use std::ops::{Add, Sub, Neg};
/// use num_traits::Zero;
/// use ospf_rust_math::algebra::concept::{VectorSpace, Field};
///
/// // 定义一个简单的二维向量 / Define a simple 2D vector
/// #[derive(Clone, Debug, PartialEq)]
/// struct Vec2 {
///     x: f64,
///     y: f64,
/// }
///
/// impl Add for Vec2 {
///     type Output = Self;
///     fn add(self, other: Self) -> Self {
///         Self { x: self.x + other.x, y: self.y + other.y }
///     }
/// }
///
/// impl Sub for Vec2 {
///     type Output = Self;
///     fn sub(self, other: Self) -> Self {
///         Self { x: self.x - other.x, y: self.y - other.y }
///     }
/// }
///
/// impl Neg for Vec2 {
///     type Output = Self;
///     fn neg(self) -> Self {
///         Self { x: -self.x, y: -self.y }
///     }
/// }
///
/// impl Zero for Vec2 {
///     fn zero() -> Self { Self { x: 0.0, y: 0.0 } }
///     fn is_zero(&self) -> bool { self.x.is_zero() && self.y.is_zero() }
/// }
///
/// impl VectorSpace for Vec2 {
///     type Scalar = f64;
///     fn scale(&self, scalar: Self::Scalar) -> Self {
///         Self { x: self.x * scalar, y: self.y * scalar }
///     }
/// }
///
/// let v = Vec2 { x: 1.0, y: 2.0 };
/// let scaled = v.scale(2.0);
/// assert_eq!(scaled.x, 2.0);
/// assert_eq!(scaled.y, 4.0);
/// ```
///
/// # 设计说明 / Design Notes
/// 向量空间是线性代数和优化理论的核心：
/// - 线性优化：解空间是向量空间
/// - 二次型优化：梯度下降在向量空间中进行
/// - 物理量计算：位移、速度、力等构成向量空间
///
/// Vector spaces are central to linear algebra and optimization:
/// - Linear optimization: solution space is a vector space
/// - Quadratic optimization: gradient descent operates in a vector space
/// - Physical quantity calculations: displacement, velocity, force form vector spaces
pub trait VectorSpace:
    Clone + Debug + PartialEq + Add<Output = Self> + Sub<Output = Self> + Neg<Output = Self> + Zero
{
    /// 标量域类型 / Scalar field type
    type Scalar: Field;

    /// 标量乘法 / Scalar multiplication
    /// 返回 `self * scalar` / Returns `self * scalar`
    fn scale(&self, scalar: Self::Scalar) -> Self;

    /// 获取零向量 / Get the zero vector
    fn zero_vector() -> Self {
        Self::zero()
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用的二维向量 / 2D vector for testing
    #[derive(Clone, Debug, PartialEq)]
    struct Vec2 {
        x: f64,
        y: f64,
    }

    impl Vec2 {
        fn new(x: f64, y: f64) -> Self {
            Self { x, y }
        }
    }

    impl Add for Vec2 {
        type Output = Self;
        fn add(self, other: Self) -> Self {
            Self {
                x: self.x + other.x,
                y: self.y + other.y,
            }
        }
    }

    impl Sub for Vec2 {
        type Output = Self;
        fn sub(self, other: Self) -> Self {
            Self {
                x: self.x - other.x,
                y: self.y - other.y,
            }
        }
    }

    impl Neg for Vec2 {
        type Output = Self;
        fn neg(self) -> Self {
            Self {
                x: -self.x,
                y: -self.y,
            }
        }
    }

    impl Zero for Vec2 {
        fn zero() -> Self {
            Self { x: 0.0, y: 0.0 }
        }
        fn is_zero(&self) -> bool {
            self.x.is_zero() && self.y.is_zero()
        }
    }

    impl VectorSpace for Vec2 {
        type Scalar = f64;

        fn scale(&self, scalar: Self::Scalar) -> Self {
            Self {
                x: self.x * scalar,
                y: self.y * scalar,
            }
        }
    }

    #[test]
    fn test_vec2_vector_space() {
        let v1 = Vec2::new(1.0, 2.0);
        let v2 = Vec2::new(3.0, 4.0);

        // 测试向量加法 / Test vector addition
        let sum = v1.clone() + v2.clone();
        assert_eq!(sum, Vec2::new(4.0, 6.0));

        // 测试标量乘法 / Test scalar multiplication
        let scaled = v1.scale(2.0);
        assert_eq!(scaled, Vec2::new(2.0, 4.0));

        // 测试零向量 / Test zero vector
        let zero = Vec2::zero_vector();
        assert_eq!(zero, Vec2::new(0.0, 0.0));

        // 测试负向量 / Test negation
        let neg = -v1.clone();
        assert_eq!(neg, Vec2::new(-1.0, -2.0));
    }

    #[test]
    fn test_vector_space_axioms() {
        let v1 = Vec2::new(1.0, 2.0);
        let v2 = Vec2::new(3.0, 4.0);
        let v3 = Vec2::new(5.0, 6.0);

        // 测试加法结合律 / Test addition associativity
        let left = (v1.clone() + v2.clone()) + v3.clone();
        let right = v1.clone() + (v2.clone() + v3.clone());
        assert_eq!(left, right);

        // 测试加法交换律 / Test addition commutativity
        assert_eq!(v1.clone() + v2.clone(), v2.clone() + v1.clone());

        // 测试标量乘法分配律 / Test scalar multiplication distributivity
        let a = 2.0;
        let b = 3.0;
        let left = v1.scale(a + b);
        let right = v1.clone().scale(a) + v1.clone().scale(b);
        assert_eq!(left, right);
    }

    #[test]
    fn test_scalar_multiplication_axioms() {
        let v = Vec2::new(1.0, 2.0);
        let a = 2.0;
        let b = 3.0;

        // 测试标量乘法结合律 / Test scalar multiplication associativity
        // a * (b * v) = (a * b) * v
        let left = v.clone().scale(b).scale(a);
        let right = v.scale(a * b);
        assert_eq!(left, right);

        // 测试单位元 / Test identity
        let identity_scaled = v.clone().scale(1.0);
        assert_eq!(identity_scaled, v);
    }
}
