//! 内积空间 trait
//! Inner product space trait

use super::NormedSpace;
use num_traits::{Float, One, Zero};

// ============================================================================
// InnerProductSpace Trait - 内积空间
// ============================================================================

/// InnerProductSpace - 内积空间 trait
/// InnerProductSpace - Inner product space trait
///
/// 表示类型构成内积空间，即在向量空间上定义了内积。
/// Represents that a type forms an inner product space, i.e., a vector space with an inner product.
///
/// 内积满足以下公理：
/// An inner product satisfies the following axioms:
///
/// 1. **共轭对称性 / Conjugate symmetry**: `⟨u, v⟩ = conj(⟨v, u⟩)`
///    对于实数域：`⟨u, v⟩ = ⟨v, u⟩`
///    For real fields: `⟨u, v⟩ = ⟨v, u⟩`
///
/// 2. **第一变元线性 / Linearity in first argument**: `⟨a*u + b*v, w⟩ = a*⟨u, w⟩ + b*⟨v, w⟩`
///
/// 3. **正定性 / Positive-definiteness**: `⟨v, v⟩ ≥ 0`，且 `⟨v, v⟩ = 0 ⟺ v = 0`
///
/// 内积空间自动构成赋范空间，范数为 `‖v‖ = √⟨v, v⟩`。
/// An inner product space is automatically a normed space with norm `‖v‖ = √⟨v, v⟩`.
///
/// # 示例 / Examples
/// ```
/// use std::ops::{Add, Sub, Neg};
/// use num_traits::Zero;
/// use ospf_rust_math::algebra::concept::{VectorSpace, NormedSpace, InnerProductSpace};
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
///     fn norm(&self) -> f64 { self.dot(self).sqrt() }
/// }
/// impl InnerProductSpace for Vec2 {
///     fn dot(&self, other: &Self) -> f64 { self.x * other.x + self.y * other.y }
/// }
///
/// let u = Vec2 { x: 1.0, y: 0.0 };
/// let v = Vec2 { x: 0.0, y: 1.0 };
/// assert!((u.dot(&v) - 0.0).abs() < 1e-10);  // 正交 / orthogonal
/// ```
///
/// # 设计说明 / Design Notes
/// 内积空间在优化和物理计算中非常重要：
/// - 线性优化：投影算法使用内积
/// - 二次型优化：目标函数可以表示为内积形式
/// - 物理量计算：功是力和位移的内积
/// - 共轭梯度法：依赖于内积结构
///
/// Inner product spaces are important in optimization and physics:
/// - Linear optimization: projection algorithms use inner products
/// - Quadratic optimization: objective functions can be expressed as inner products
/// - Physical quantity calculations: work is the inner product of force and displacement
/// - Conjugate gradient method: depends on inner product structure
pub trait InnerProductSpace: NormedSpace {
    /// 计算内积 / Compute the inner product
    /// 返回 `⟨self, other⟩` / Returns `⟨self, other⟩`
    fn dot(&self, other: &Self) -> Self::Scalar;

    /// 计算两个向量的夹角（弧度）/ Compute the angle between two vectors (radians)
    /// 返回 arccos(⟨u, v⟩ / (‖u‖ * ‖v‖))
    /// Returns arccos(⟨u, v⟩ / (‖u‖ * ‖v‖))
    fn angle(&self, other: &Self) -> Self::Scalar
    where
        Self::Scalar: Float,
    {
        let dot = self.dot(other);
        let norm_product = self.norm() * other.norm();
        if norm_product.is_zero() {
            Self::Scalar::zero()
        } else {
            let cos_angle = dot / norm_product;
            // 限制在 [-1, 1] 范围内以避免浮点误差
            // Clamp to [-1, 1] to avoid floating-point errors
            let clamped = Float::max(
                Float::min(cos_angle, Self::Scalar::one()),
                -Self::Scalar::one(),
            );
            Float::acos(clamped)
        }
    }

    /// 判断两个向量是否正交 / Check if two vectors are orthogonal
    /// 返回 `⟨self, other⟩ = 0` / Returns `⟨self, other⟩ = 0`
    fn is_orthogonal(&self, other: &Self, epsilon: Self::Scalar) -> bool
    where
        Self::Scalar: Float,
    {
        self.dot(other).abs() < epsilon
    }

    /// 计算投影 / Compute projection
    /// 返回 self 在 other 方向上的投影
    /// Returns the projection of self onto other
    fn project(&self, other: &Self) -> Self
    where
        Self::Scalar: Zero,
    {
        let norm_sq = other.dot(other);
        if norm_sq.is_zero() {
            Self::zero()
        } else {
            let scalar = self.dot(other) / norm_sq;
            other.scale(scalar)
        }
    }

    /// 计算正交分量 / Compute orthogonal component
    /// 返回 self - proj_other(self)
    /// Returns self - proj_other(self)
    fn orthogonal_component(&self, other: &Self) -> Self
    where
        Self: Clone,
        Self::Scalar: One + std::ops::Neg<Output = Self::Scalar>,
    {
        let projection = self.project(other);
        self.clone() + projection.scale(-Self::Scalar::one())
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::concept::VectorSpace;
    use num_traits::Zero;
    use std::ops::{Add, Neg, Sub};

    /// 测试用的二维向量（欧几里得内积）/ 2D vector with Euclidean inner product
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
            self.dot(self).sqrt()
        }
    }

    impl InnerProductSpace for Vec2 {
        fn dot(&self, other: &Self) -> Self::Scalar {
            self.x * other.x + self.y * other.y
        }
    }

    #[test]
    fn test_vec2_inner_product() {
        let u = Vec2::new(1.0, 2.0);
        let v = Vec2::new(3.0, 4.0);

        // 测试内积 / Test inner product
        // ⟨u, v⟩ = 1*3 + 2*4 = 11
        assert!((u.dot(&v) - 11.0).abs() < 1e-10);

        // 测试内积对称性 / Test inner product symmetry
        assert!((u.dot(&v) - v.dot(&u)).abs() < 1e-10);
    }

    #[test]
    fn test_orthogonality() {
        let u = Vec2::new(1.0, 0.0);
        let v = Vec2::new(0.0, 1.0);

        // 测试正交 / Test orthogonality
        assert!(u.is_orthogonal(&v, 1e-10));

        // 测试非正交 / Test non-orthogonality
        let w = Vec2::new(1.0, 1.0);
        assert!(!u.is_orthogonal(&w, 1e-10));
    }

    #[test]
    fn test_angle() {
        let u = Vec2::new(1.0, 0.0);
        let v = Vec2::new(0.0, 1.0);

        // 测试直角 / Test right angle
        let angle = u.angle(&v);
        assert!((angle - std::f64::consts::FRAC_PI_2).abs() < 1e-10);

        // 测试零角度 / Test zero angle
        let angle_same = u.angle(&u);
        assert!(angle_same.abs() < 1e-10);
    }

    #[test]
    fn test_projection() {
        let u = Vec2::new(3.0, 4.0);
        let v = Vec2::new(1.0, 0.0);

        // 测试投影 / Test projection
        let proj = u.project(&v);
        assert!((proj.x - 3.0).abs() < 1e-10);
        assert!((proj.y - 0.0).abs() < 1e-10);

        // 测试正交分量 / Test orthogonal component
        let ortho = u.orthogonal_component(&v);
        assert!((ortho.x - 0.0).abs() < 1e-10);
        assert!((ortho.y - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_norm_from_inner_product() {
        let v = Vec2::new(3.0, 4.0);

        // ‖v‖ = √⟨v, v⟩
        let norm = v.norm();
        let norm_from_dot = v.dot(&v).sqrt();
        assert!((norm - norm_from_dot).abs() < 1e-10);
        assert!((norm - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_cauchy_schwarz() {
        let u = Vec2::new(1.0, 2.0);
        let v = Vec2::new(3.0, 4.0);

        // 柯西-施瓦茨不等式：|⟨u, v⟩| ≤ ‖u‖ * ‖v‖
        // Cauchy-Schwarz inequality: |⟨u, v⟩| ≤ ‖u‖ * ‖v‖
        let dot_abs = u.dot(&v).abs();
        let norm_product = u.norm() * v.norm();
        assert!(dot_abs <= norm_product + 1e-10);
    }
}
