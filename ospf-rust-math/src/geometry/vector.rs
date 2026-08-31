//! 向量实体模块
//! Vector entity module
//!
//! 本模块定义了向量类型，支持任意维度：
//! This module defines vector types with arbitrary dimensions:
//!
//! - [`Vector`] - 泛型向量，使用 const generics 支持编译期维度
//! - [`Vector`] - Generic vector using const generics for compile-time dimensions
//!
//! # 示例 / Examples
//!
//! ```
//! use ospf_rust_math::geometry::Vector2;
//! use ospf_rust_math::algebra::{NormedSpace, InnerProductSpace};
//!
//! let v1 = Vector2::new(3.0, 4.0);
//! let v2 = Vector2::new(1.0, 2.0);
//!
//! // 范数 / Norm
//! let norm_diff: f64 = v1.norm() - 5.0;
//! assert!(norm_diff.abs() < 1e-10);
//!
//! // 内积 / Dot product
//! let dot_diff: f64 = v1.dot(&v2) - 11.0;
//! assert!(dot_diff.abs() < 1e-10);
//!
//! // 单位向量 / Unit vector
//! let unit = v1.normalize().unwrap();
//! let unit_norm_diff: f64 = unit.norm() - 1.0;
//! assert!(unit_norm_diff.abs() < 1e-10);
//! ```

use super::point::Point;
use crate::algebra::{Epsilon, Field, InnerProductSpace, NormedSpace, VectorSpace};
use num_traits::{Float, One, Zero};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::ops::{Add, Index, IndexMut, Mul, Neg, Sub};

// ============================================================================
// Vector - 泛型向量
// ============================================================================

/// Vector - 泛型向量
/// Vector - Generic vector
///
/// 表示 N 维空间中的一个向量，使用 const generics 支持编译期维度。
/// Represents a vector in N-dimensional space using const generics for compile-time dimensions.
///
/// # 泛型参数 / Generic Parameters
/// - `const D: usize`: 维度（编译期常量）/ Dimension (compile-time constant)
/// - `S`: 标量类型，默认为 `f64` / Scalar type, defaults to `f64`
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::geometry::{Vector, Vector2, Vector3};
///
/// // 创建 2D 向量 / Create 2D vector
/// let v2 = Vector2::new(1.0, 2.0);
/// assert_eq!(v2.dim(), 2);
///
/// // 创建 3D 向量 / Create 3D vector
/// let v3 = Vector3::new(1.0, 2.0, 3.0);
/// assert_eq!(v3.dim(), 3);
///
/// // 从数组创建 / Create from array
/// let v = Vector::<3, f64>::from_components([1.0, 2.0, 3.0]);
/// ```
#[derive(Clone, PartialEq)]
pub struct Vector<const D: usize, S = f64> {
    /// 分量数组 / Component array
    components: [S; D],
}

// ============================================================================
// 构造方法 / Constructors
// ============================================================================

impl<const D: usize, S> Vector<D, S> {
    /// 从分量数组创建向量
    /// Create a vector from component array
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::Vector2;
    ///
    /// let v = Vector2::from_components([1.0, 2.0]);
    /// ```
    pub fn from_components(components: [S; D]) -> Self {
        Self { components }
    }

    /// 消费向量并返回分量数组
    /// Consume the vector and return the component array
    pub fn into_components(self) -> [S; D] {
        self.components
    }

    /// 创建零向量
    /// Create a zero vector
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::Vector2;
    /// use num_traits::Zero;
    ///
    /// let zero: Vector2 = Vector2::zero_vector();
    /// assert!(zero.is_zero());
    /// ```
    pub fn zero_vector() -> Self
    where
        S: Zero,
    {
        Self {
            components: std::array::from_fn(|_| S::zero()),
        }
    }

    /// 创建单位向量（沿指定轴）
    /// Create a unit vector along the specified axis
    ///
    /// # 参数 / Parameters
    /// - `axis`: 轴索引 / Axis index
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::Vector3;
    ///
    /// let ex: Vector3 = Vector3::unit_vector(0);  // (1, 0, 0)
    /// let ey: Vector3 = Vector3::unit_vector(1);  // (0, 1, 0)
    /// let ez: Vector3 = Vector3::unit_vector(2);  // (0, 0, 1)
    /// ```
    pub fn unit_vector(axis: usize) -> Self
    where
        S: Zero + One,
    {
        let mut components = std::array::from_fn(|_| S::zero());
        if axis < D {
            components[axis] = S::one();
        }
        Self { components }
    }

    /// 返回维度
    /// Return the dimension
    pub fn dim(&self) -> usize {
        D
    }

    /// 获取分量的不可变引用
    /// Get an immutable reference to components
    pub fn components(&self) -> &[S; D] {
        &self.components
    }

    /// 获取分量的可变引用
    /// Get a mutable reference to components
    pub fn components_mut(&mut self) -> &mut [S; D] {
        &mut self.components
    }

    /// 获取指定维度的分量引用
    /// Get a reference to the component at the specified dimension
    pub fn get_ref(&self, i: usize) -> Option<&S> {
        self.components.get(i)
    }

    /// 设置指定维度的分量值
    /// Set the component value at the specified dimension
    pub fn set(&mut self, i: usize, value: S) -> bool {
        if i < D {
            self.components[i] = value;
            true
        } else {
            false
        }
    }
}

impl<const D: usize, S: Copy> Vector<D, S> {
    /// 获取指定维度的分量值
    /// Get the component value at the specified dimension
    pub fn get(&self, i: usize) -> Option<S> {
        self.components.get(i).copied()
    }
}

// ============================================================================
// 2D 便捷方法 / 2D convenience methods
// ============================================================================

impl<S> Vector<2, S> {
    /// 创建 2D 向量
    /// Create a 2D vector
    pub fn new(x: S, y: S) -> Self {
        Self::from_components([x, y])
    }

    /// 获取 x 分量引用
    /// Get x component reference
    pub fn x_ref(&self) -> &S {
        &self.components[0]
    }

    /// 获取 y 分量引用
    /// Get y component reference
    pub fn y_ref(&self) -> &S {
        &self.components[1]
    }

    /// 设置 x 分量
    /// Set x component
    pub fn set_x(&mut self, x: S) {
        self.components[0] = x;
    }

    /// 设置 y 分量
    /// Set y component
    pub fn set_y(&mut self, y: S) {
        self.components[1] = y;
    }
}

impl<S: Copy> Vector<2, S> {
    /// 获取 x 分量
    /// Get x component
    pub fn x(&self) -> S {
        self.components[0]
    }

    /// 获取 y 分量
    /// Get y component
    pub fn y(&self) -> S {
        self.components[1]
    }
}

impl<S: Field + Float> Vector<2, S> {
    /// 计算 2D 叉积（返回标量）
    /// Calculate 2D cross product (returns scalar)
    ///
    /// 对于 2D 向量，叉积返回一个标量，表示两向量张成的平行四边形的有向面积。
    /// For 2D vectors, cross product returns a scalar representing the signed area
    /// of the parallelogram formed by the two vectors.
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::Vector2;
    ///
    /// let v1 = Vector2::new(1.0, 0.0);
    /// let v2 = Vector2::new(0.0, 1.0);
    /// let diff: f64 = v1.cross_2d(&v2) - 1.0;
    /// assert!(diff.abs() < 1e-10);
    /// ```
    pub fn cross_2d(&self, other: &Self) -> S {
        self.x() * other.y() - self.y() * other.x()
    }

    /// 计算 2D 向量的垂直向量（逆时针旋转 90°）
    /// Calculate the perpendicular vector (90° counter-clockwise rotation)
    pub fn perpendicular(&self) -> Self {
        Self::new(-self.y(), self.x())
    }
}

// ============================================================================
// 3D 便捷方法 / 3D convenience methods
// ============================================================================

impl<S> Vector<3, S> {
    /// 创建 3D 向量
    /// Create a 3D vector
    pub fn new(x: S, y: S, z: S) -> Self {
        Self::from_components([x, y, z])
    }

    /// 获取 x 分量引用
    /// Get x component reference
    pub fn x_ref(&self) -> &S {
        &self.components[0]
    }

    /// 获取 y 分量引用
    /// Get y component reference
    pub fn y_ref(&self) -> &S {
        &self.components[1]
    }

    /// 获取 z 分量引用
    /// Get z component reference
    pub fn z_ref(&self) -> &S {
        &self.components[2]
    }

    /// 设置 x 分量
    /// Set x component
    pub fn set_x(&mut self, x: S) {
        self.components[0] = x;
    }

    /// 设置 y 分量
    /// Set y component
    pub fn set_y(&mut self, y: S) {
        self.components[1] = y;
    }

    /// 设置 z 分量
    /// Set z component
    pub fn set_z(&mut self, z: S) {
        self.components[2] = z;
    }
}

impl<S: Copy> Vector<3, S> {
    /// 获取 x 分量
    /// Get x component
    pub fn x(&self) -> S {
        self.components[0]
    }

    /// 获取 y 分量
    /// Get y component
    pub fn y(&self) -> S {
        self.components[1]
    }

    /// 获取 z 分量
    /// Get z component
    pub fn z(&self) -> S {
        self.components[2]
    }
}

impl<S: Field + Float> Vector<3, S> {
    /// 计算 3D 叉积
    /// Calculate 3D cross product
    ///
    /// 返回同时垂直于两个输入向量的向量。
    /// Returns a vector perpendicular to both input vectors.
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::Vector3;
    ///
    /// let v1 = Vector3::new(1.0, 0.0, 0.0);
    /// let v2 = Vector3::new(0.0, 1.0, 0.0);
    /// let cross = v1.cross(&v2);
    /// let diff: f64 = cross.z() - 1.0;
    /// assert!(diff.abs() < 1e-10);
    /// ```
    pub fn cross(&self, other: &Self) -> Self {
        Self::new(
            self.y() * other.z() - self.z() * other.y(),
            self.z() * other.x() - self.x() * other.z(),
            self.x() * other.y() - self.y() * other.x(),
        )
    }

    /// 计算 3D 向量的三重标量积（混合积）
    /// Calculate the triple scalar product (scalar triple product)
    ///
    /// 返回 `a · (b × c)`，表示三个向量张成的平行六面体的有向体积。
    /// Returns `a · (b × c)`, the signed volume of the parallelepiped formed by three vectors.
    pub fn triple_product(&self, b: &Self, c: &Self) -> S {
        self.dot(&b.cross(c))
    }
}

// ============================================================================
// 向量操作 / Vector operations
// ============================================================================

impl<const D: usize, S: Field + Float> Vector<D, S> {
    /// 从两点创建向量（从 p1 指向 p2）
    /// Create a vector from two points (from p1 to p2)
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Vector2};
    ///
    /// let p1 = Point2::new(1.0, 2.0);
    /// let p2 = Point2::new(4.0, 6.0);
    /// let v = Vector2::from_points(&p1, &p2);
    /// let x_diff: f64 = v.x() - 3.0;
    /// let y_diff: f64 = v.y() - 4.0;
    /// assert!(x_diff.abs() < 1e-10);
    /// assert!(y_diff.abs() < 1e-10);
    /// ```
    pub fn from_points(p1: &Point<D, S>, p2: &Point<D, S>) -> Self {
        Self::from_components(std::array::from_fn(|i| p2[i] - p1[i]))
    }

    /// 计算两向量的夹角（弧度）
    /// Calculate the angle between two vectors (radians)
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::Vector2;
    /// use std::f64::consts::FRAC_PI_2;
    ///
    /// let v1 = Vector2::new(1.0, 0.0);
    /// let v2 = Vector2::new(0.0, 1.0);
    /// let angle = v1.angle(&v2);
    /// assert!((angle - FRAC_PI_2).abs() < 1e-10);
    /// ```
    pub fn angle(&self, other: &Self) -> S {
        let dot = self.dot(other);
        let norm_product = self.norm() * other.norm();
        if norm_product.is_zero() {
            S::zero()
        } else {
            let cos_angle = dot / norm_product;
            // 限制在 [-1, 1] 范围内以避免浮点误差
            // Clamp to [-1, 1] to avoid floating-point errors
            let clamped = Float::max(Float::min(cos_angle, S::one()), -S::one());
            Float::acos(clamped)
        }
    }

    /// 判断两向量是否正交
    /// Check if two vectors are orthogonal
    pub fn is_orthogonal(&self, other: &Self, epsilon: S) -> bool {
        self.dot(other).abs() < epsilon
    }

    /// 计算投影（self 在 other 方向上的投影）
    /// Calculate projection (projection of self onto other)
    pub fn project(&self, other: &Self) -> Self {
        let norm_sq = other.dot(other);
        if norm_sq.is_zero() {
            Self::zero_vector()
        } else {
            let scalar = self.dot(other) / norm_sq;
            other.scale(scalar)
        }
    }

    /// 计算正交分量（self - proj_other(self)）
    /// Calculate orthogonal component (self - proj_other(self))
    pub fn orthogonal_component(&self, other: &Self) -> Self {
        let projection = self.project(other);
        self.clone() + projection.scale(-S::one())
    }

    /// 判断两向量是否平行
    /// Check if two vectors are parallel
    pub fn is_parallel(&self, other: &Self, epsilon: S) -> bool {
        let cross_norm_sq = self.cross_norm_squared(other);
        cross_norm_sq < epsilon * epsilon
    }

    /// 计算叉积范数的平方（通用版本）
    /// Calculate squared norm of cross product (general version)
    fn cross_norm_squared(&self, other: &Self) -> S {
        // 使用拉格朗日恒等式：|a × b|² = |a|²|b|² - (a · b)²
        // Use Lagrange's identity: |a × b|² = |a|²|b|² - (a · b)²
        let norm_sq_a = self.dot(self);
        let norm_sq_b = other.dot(other);
        let dot = self.dot(other);
        norm_sq_a * norm_sq_b - dot * dot
    }
}

// ============================================================================
// 类型别名 / Type aliases
// ============================================================================

/// 2D 向量类型别名
/// 2D vector type alias
pub type Vector2<S = f64> = Vector<2, S>;

/// 3D 向量类型别名
/// 3D vector type alias
pub type Vector3<S = f64> = Vector<3, S>;

/// 4D 向量类型别名
/// 4D vector type alias
pub type Vector4<S = f64> = Vector<4, S>;

// ============================================================================
// Trait 实现 / Trait implementations
// ============================================================================

impl<const D: usize, S> Index<usize> for Vector<D, S> {
    type Output = S;

    fn index(&self, index: usize) -> &Self::Output {
        &self.components[index]
    }
}

impl<const D: usize, S> IndexMut<usize> for Vector<D, S> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.components[index]
    }
}

impl<const D: usize, S> Add for Vector<D, S>
where
    S: Clone + Add<Output = S>,
{
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::from_components(std::array::from_fn(|i| {
            self.components[i].clone() + other.components[i].clone()
        }))
    }
}

impl<const D: usize, S> Sub for Vector<D, S>
where
    S: Clone + Sub<Output = S>,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::from_components(std::array::from_fn(|i| {
            self.components[i].clone() - other.components[i].clone()
        }))
    }
}

impl<const D: usize, S> Neg for Vector<D, S>
where
    S: Clone + Neg<Output = S>,
{
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::from_components(std::array::from_fn(|i| -self.components[i].clone()))
    }
}

impl<const D: usize, S: Field + Float> Mul<S> for Vector<D, S> {
    type Output = Self;

    fn mul(self, scalar: S) -> Self::Output {
        self.scale(scalar)
    }
}

impl<const D: usize, S> Zero for Vector<D, S>
where
    S: Zero + Clone,
{
    fn zero() -> Self {
        Self::zero_vector()
    }

    fn is_zero(&self) -> bool {
        self.components.iter().all(|c| c.is_zero())
    }
}

impl<const D: usize, S: Field + Float> VectorSpace for Vector<D, S> {
    type Scalar = S;

    fn scale(&self, scalar: Self::Scalar) -> Self {
        Self::from_components(std::array::from_fn(|i| self.components[i] * scalar))
    }
}

impl<const D: usize, S: Field + Float> NormedSpace for Vector<D, S> {
    fn norm(&self) -> Self::Scalar {
        self.components
            .iter()
            .fold(S::zero(), |acc, &c| acc + c * c)
            .sqrt()
    }

    fn norm_squared(&self) -> Self::Scalar {
        self.components
            .iter()
            .fold(S::zero(), |acc, &c| acc + c * c)
    }
}

impl<const D: usize, S: Field + Float> InnerProductSpace for Vector<D, S> {
    fn dot(&self, other: &Self) -> Self::Scalar {
        self.components
            .iter()
            .zip(other.components.iter())
            .fold(S::zero(), |acc, (&a, &b)| acc + a * b)
    }
}

impl<const D: usize, S: Debug> Debug for Vector<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Vector{}(", D)?;
        for (i, c) in self.components.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", c)?;
        }
        write!(f, ")")
    }
}

impl<const D: usize, S: Display> Display for Vector<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "[")?;
        for (i, c) in self.components.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", c)?;
        }
        write!(f, "]")
    }
}

impl<const D: usize, S: Zero> Default for Vector<D, S> {
    fn default() -> Self {
        Self::zero_vector()
    }
}

// ============================================================================
// 从 Point 转换 / Conversion from Point
// ============================================================================

impl<const D: usize, S> From<Point<D, S>> for Vector<D, S> {
    fn from(point: Point<D, S>) -> Self {
        Self::from_components(point.into_coords())
    }
}

impl<const D: usize, S> From<Vector<D, S>> for Point<D, S> {
    fn from(vector: Vector<D, S>) -> Self {
        Point::from_coords(vector.components)
    }
}

// ============================================================================
// 容差比较 / Tolerance comparison
// ============================================================================

impl<const D: usize, S: Field + Float + Epsilon> Vector<D, S> {
    /// 使用容差判断两向量是否近似相等
    /// Check if two vectors are approximately equal using tolerance
    pub fn approx_eq(&self, other: &Self) -> bool {
        self.approx_eq_with(other, <S as Epsilon>::epsilon())
    }

    /// 使用指定容差判断两向量是否近似相等
    /// Check if two vectors are approximately equal using specified tolerance
    pub fn approx_eq_with(&self, other: &Self, epsilon: S) -> bool {
        self.components
            .iter()
            .zip(other.components.iter())
            .all(|(&a, &b)| (a - b).abs() < epsilon)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point2;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn test_vector2_creation() {
        let v = Vector2::new(1.0, 2.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.dim(), 2);
    }

    #[test]
    fn test_vector3_creation() {
        let v = Vector3::new(1.0, 2.0, 3.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
        assert_eq!(v.dim(), 3);
    }

    #[test]
    fn test_vector_operations() {
        let v1 = Vector2::new(1.0, 2.0);
        let v2 = Vector2::new(3.0, 4.0);

        // 加法 / Addition
        let sum = v1.clone() + v2.clone();
        assert_eq!(sum.x(), 4.0);
        assert_eq!(sum.y(), 6.0);

        // 减法 / Subtraction
        let diff = v2.clone() - v1.clone();
        assert_eq!(diff.x(), 2.0);
        assert_eq!(diff.y(), 2.0);

        // 标量乘法 / Scalar multiplication
        let scaled = v1.scale(2.0);
        assert_eq!(scaled.x(), 2.0);
        assert_eq!(scaled.y(), 4.0);
    }

    #[test]
    fn test_norm_and_dot() {
        let v = Vector2::new(3.0, 4.0);

        // 范数 / Norm
        assert!((v.norm() - 5.0).abs() < 1e-10);

        // 范数平方 / Squared norm
        assert!((v.norm_squared() - 25.0).abs() < 1e-10);

        // 内积 / Inner product
        let v1 = Vector2::new(1.0, 2.0);
        let v2 = Vector2::new(3.0, 4.0);
        assert!((v1.dot(&v2) - 11.0).abs() < 1e-10);
    }

    #[test]
    fn test_normalize() {
        let v = Vector2::new(3.0, 4.0);
        let unit = v.normalize().unwrap();

        assert!((unit.norm() - 1.0).abs() < 1e-10);
        assert!((unit.x() - 0.6).abs() < 1e-10);
        assert!((unit.y() - 0.8).abs() < 1e-10);

        // 零向量归一化 / Zero vector normalization
        let zero: Vector2 = Vector2::zero();
        assert!(zero.normalize().is_none());
    }

    #[test]
    fn test_cross_2d() {
        let v1 = Vector2::new(1.0, 0.0);
        let v2 = Vector2::new(0.0, 1.0);

        // 正向 / Positive
        assert!((v1.cross_2d(&v2) - 1.0).abs() < 1e-10);

        // 反向 / Negative
        assert!((v2.cross_2d(&v1) - (-1.0)).abs() < 1e-10);

        // 平行 / Parallel
        let v3 = Vector2::new(2.0, 0.0);
        assert!(v1.cross_2d(&v3).abs() < 1e-10);
    }

    #[test]
    fn test_cross_3d() {
        let v1 = Vector3::new(1.0, 0.0, 0.0);
        let v2 = Vector3::new(0.0, 1.0, 0.0);
        let cross = v1.cross(&v2);

        assert!((cross.x() - 0.0).abs() < 1e-10);
        assert!((cross.y() - 0.0).abs() < 1e-10);
        assert!((cross.z() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_angle() {
        let v1 = Vector2::new(1.0, 0.0);
        let v2 = Vector2::new(0.0, 1.0);

        assert!((v1.angle(&v2) - FRAC_PI_2).abs() < 1e-10);

        // 零角度 / Zero angle
        assert!(v1.angle(&v1).abs() < 1e-10);
    }

    #[test]
    fn test_orthogonal() {
        let v1 = Vector2::new(1.0, 0.0);
        let v2 = Vector2::new(0.0, 1.0);

        assert!(v1.is_orthogonal(&v2, 1e-10));

        let v3 = Vector2::new(1.0, 1.0);
        assert!(!v1.is_orthogonal(&v3, 1e-10));
    }

    #[test]
    fn test_projection() {
        let v = Vector2::new(3.0, 4.0);
        let axis = Vector2::new(1.0, 0.0);

        let proj = v.project(&axis);
        assert!((proj.x() - 3.0).abs() < 1e-10);
        assert!((proj.y() - 0.0).abs() < 1e-10);

        let ortho = v.orthogonal_component(&axis);
        assert!((ortho.x() - 0.0).abs() < 1e-10);
        assert!((ortho.y() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_from_points() {
        let p1 = Point2::new(1.0, 2.0);
        let p2 = Point2::new(4.0, 6.0);

        let v = Vector2::from_points(&p1, &p2);
        assert_eq!(v.x(), 3.0);
        assert_eq!(v.y(), 4.0);
    }

    #[test]
    fn test_unit_vector() {
        let ex: Vector3 = Vector3::unit_vector(0);
        let ey: Vector3 = Vector3::unit_vector(1);
        let ez: Vector3 = Vector3::unit_vector(2);

        assert_eq!(ex.x(), 1.0);
        assert_eq!(ex.y(), 0.0);
        assert_eq!(ex.z(), 0.0);

        assert_eq!(ey.x(), 0.0);
        assert_eq!(ey.y(), 1.0);
        assert_eq!(ey.z(), 0.0);

        assert_eq!(ez.x(), 0.0);
        assert_eq!(ez.y(), 0.0);
        assert_eq!(ez.z(), 1.0);
    }

    #[test]
    fn test_perpendicular() {
        let v = Vector2::new(3.0, 4.0);
        let perp = v.perpendicular();

        assert_eq!(perp.x(), -4.0);
        assert_eq!(perp.y(), 3.0);

        // 垂直于原向量 / Orthogonal to original vector
        assert!(v.is_orthogonal(&perp, 1e-10));
    }

    #[test]
    fn test_approx_eq() {
        let v1 = Vector2::new(1.0, 2.0);
        // 使用一个很小的差值进行测试
        // Use a very small difference for testing
        let diff = 1e-15;
        let v2 = Vector2::new(1.0 + diff, 2.0 + diff);

        // 使用默认 epsilon（项目的 Epsilon trait 为 f64 定义为 1e-10）
        // Use default epsilon (project's Epsilon trait defines 1e-10 for f64)
        assert!(v1.approx_eq(&v2));

        // 使用更小的容差应该不相等
        // Should not be equal with smaller tolerance
        assert!(!v1.approx_eq_with(&v2, 1e-16));
    }
}
