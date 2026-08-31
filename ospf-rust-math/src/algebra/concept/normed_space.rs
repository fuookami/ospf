//! 赋范空间 trait
//! Normed space trait

use num_traits::{One, Zero};
use super::VectorSpace;

// ============================================================================
// NormedSpace Trait - 赋范空间
// ============================================================================

/// NormedSpace - 赋范空间 trait
/// NormedSpace - Normed space trait
///
/// 表示类型构成赋范空间，即在向量空间上定义了范数。
/// Represents that a type forms a normed space, i.e., a vector space with a norm.
///
/// 范数满足以下公理：
/// A norm satisfies the following axioms:
///
/// 1. **非负性 / Non-negativity**: `‖v‖ ≥ 0`，且 `‖v‖ = 0 ⟺ v = 0`
/// 2. **齐次性 / Homogeneity**: `‖a * v‖ = |a| * ‖v‖`
/// 3. **三角不等式 / Triangle inequality**: `‖u + v‖ ≤ ‖u‖ + ‖v‖`
///
/// # 示例 / Examples
/// ```
/// use std::ops::{Add, Sub, Neg};
/// use num_traits::Zero;
/// use ospf_rust_math::algebra::concept::{VectorSpace, NormedSpace};
///
/// // 定义一个二维向量 / Define a 2D vector
/// #[derive(Clone, Debug, PartialEq)]
/// struct Vec2 { x: f64, y: f64 }
///
/// impl Add for Vec2 {
///     type Output = Self;
///     fn add(self, other: Self) -> Self { Self { x: self.x + other.x, y: self.y + other.y } }
/// }
/// impl Sub for Vec2 {
///     type Output = Self;
///     fn sub(self, other: Self) -> Self { Self { x: self.x - other.x, y: self.y - other.y } }
/// }
/// impl Neg for Vec2 {
///     type Output = Self;
///     fn neg(self) -> Self { Self { x: -self.x, y: -self.y } }
/// }
/// impl Zero for Vec2 {
///     fn zero() -> Self { Self { x: 0.0, y: 0.0 } }
///     fn is_zero(&self) -> bool { self.x.is_zero() && self.y.is_zero() }
/// }
/// impl VectorSpace for Vec2 {
///     type Scalar = f64;
///     fn scale(&self, s: f64) -> Self { Self { x: self.x * s, y: self.y * s } }
/// }
/// impl NormedSpace for Vec2 {
///     fn norm(&self) -> f64 { (self.x * self.x + self.y * self.y).sqrt() }
/// }
///
/// let v = Vec2 { x: 3.0, y: 4.0 };
/// assert!((v.norm() - 5.0).abs() < 1e-10);
/// ```
///
/// # 设计说明 / Design Notes
/// 赋范空间在优化中非常重要：
/// - 线性优化：约束条件通常用范数表示
/// - 二次型优化：目标函数可以是范数的平方
/// - 物理量计算：向量的长度就是范数
///
/// Normed spaces are important in optimization:
/// - Linear optimization: constraints are often expressed using norms
/// - Quadratic optimization: objective functions can be squared norms
/// - Physical quantity calculations: vector length is a norm
pub trait NormedSpace: VectorSpace {
    /// 计算向量的范数 / Compute the norm of the vector
    /// 返回向量的"长度" / Returns the "length" of the vector
    fn norm(&self) -> Self::Scalar;

    /// 计算范数的平方 / Compute the squared norm
    /// 对于某些优化问题，避免开方运算更高效
    /// For some optimization problems, avoiding square root is more efficient
    fn norm_squared(&self) -> Self::Scalar {
        let n = self.norm();
        n.clone() * n
    }

    /// 归一化向量（单位向量）/ Normalize to unit vector
    /// 返回 `v / ‖v‖`，如果 v = 0 返回 None
    /// Returns `v / ‖v‖`, or None if v = 0
    fn normalize(&self) -> Option<Self>
    where
        Self: Sized,
        Self::Scalar: Zero + One,
    {
        let n = self.norm();
        if n.is_zero() {
            None
        } else {
            Some(self.scale(Self::Scalar::one() / n))
        }
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::Zero;
    use std::ops::{Add, Neg, Sub};

    /// 测试用的二维向量（欧几里得范数）/ 2D vector with Euclidean norm
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

    impl NormedSpace for Vec2 {
        fn norm(&self) -> Self::Scalar {
            (self.x * self.x + self.y * self.y).sqrt()
        }

        fn norm_squared(&self) -> Self::Scalar {
            self.x * self.x + self.y * self.y
        }
    }

    #[test]
    fn test_vec2_norm() {
        let v = Vec2::new(3.0, 4.0);

        // 测试范数（勾股定理）/ Test norm (Pythagorean theorem)
        assert!((v.norm() - 5.0).abs() < 1e-10);

        // 测试范数平方 / Test squared norm
        assert!((v.norm_squared() - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_normalization() {
        let v = Vec2::new(3.0, 4.0);

        // 测试归一化 / Test normalization
        let unit = v.normalize().unwrap();
        assert!((unit.norm() - 1.0).abs() < 1e-10);

        // 测试零向量归一化 / Test zero vector normalization
        let zero = Vec2::zero();
        assert!(zero.normalize().is_none());
    }

    #[test]
    fn test_norm_properties() {
        let u = Vec2::new(1.0, 0.0);
        let v = Vec2::new(0.0, 1.0);
        let a = 2.0;

        // 测试齐次性 / Test homogeneity
        // ‖a * v‖ = |a| * ‖v‖
        let scaled = u.scale(a);
        assert!((scaled.norm() - a.abs() * u.norm()).abs() < 1e-10);

        // 测试三角不等式 / Test triangle inequality
        // ‖u + v‖ ≤ ‖u‖ + ‖v‖
        let sum = u.clone() + v.clone();
        assert!(sum.norm() <= u.norm() + v.norm());
    }

    #[test]
    fn test_zero_norm() {
        let zero = Vec2::zero();

        // 零向量的范数为 0 / Zero vector has norm 0
        assert!((zero.norm() - 0.0).abs() < 1e-10);
    }
}
