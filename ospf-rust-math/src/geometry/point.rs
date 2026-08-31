//! 点实体模块
//! Point entity module
//!
//! 本模块定义了点类型，支持任意维度：
//! This module defines point types with arbitrary dimensions:
//!
//! - [`Point`] - 泛型点，使用 const generics 支持编译期维度
//! - [`Point`] - Generic point using const generics for compile-time dimensions
//!
//! # 示例 / Examples
//!
//! ```
//! use ospf_rust_math::geometry::Point2;
//!
//! let p1 = Point2::new(1.0, 2.0);
//! let p2 = Point2::new(4.0, 6.0);
//!
//! // 距离计算 / Distance calculation
//! let dist: f64 = p1.distance(&p2) - 5.0;
//! assert!(dist.abs() < 1e-10);
//!
//! // 中点 / Midpoint
//! let mid = p1.midpoint(&p2);
//! let x_diff: f64 = mid.x() - 2.5;
//! assert!(x_diff.abs() < 1e-10);
//! ```

use super::distance::{Distance, Euclidean};
use crate::algebra::{Epsilon, Field, InnerProductSpace, NormedSpace, VectorSpace};
use num_traits::{Float, One, Zero};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::ops::{Add, Index, IndexMut, Neg, Sub};

// ============================================================================
// Point - 泛型点
// ============================================================================

/// Point - 泛型点
/// Point - Generic point
///
/// 表示 N 维空间中的一个点，使用 const generics 支持编译期维度。
/// Represents a point in N-dimensional space using const generics for compile-time dimensions.
///
/// # 泛型参数 / Generic Parameters
/// - `const D: usize`: 维度（编译期常量）/ Dimension (compile-time constant)
/// - `S`: 标量类型，默认为 `f64` / Scalar type, defaults to `f64`
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::geometry::{Point, Point2, Point3};
///
/// // 创建 2D 点 / Create 2D point
/// let p2 = Point2::new(1.0, 2.0);
/// assert_eq!(p2.dim(), 2);
///
/// // 创建 3D 点 / Create 3D point
/// let p3 = Point3::new(1.0, 2.0, 3.0);
/// assert_eq!(p3.dim(), 3);
///
/// // 从数组创建 / Create from array
/// let p = Point::<3, f64>::from_coords([1.0, 2.0, 3.0]);
/// ```
#[derive(Clone, PartialEq)]
pub struct Point<const D: usize, S: Field + Float = f64> {
    /// 坐标数组 / Coordinate array
    coords: [S; D],
}

// ============================================================================
// 构造方法 / Constructors
// ============================================================================

impl<const D: usize, S: Field + Float> Point<D, S> {
    /// 从坐标数组创建点
    /// Create a point from coordinate array
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::Point2;
    ///
    /// let p = Point2::from_coords([1.0, 2.0]);
    /// ```
    pub fn from_coords(coords: [S; D]) -> Self {
        Self { coords }
    }

    /// 创建原点（所有坐标为零）
    /// Create the origin point (all coordinates are zero)
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::Point2;
    /// use num_traits::Zero;
    ///
    /// let origin: Point2 = Point2::origin();
    /// assert!(origin.is_zero());
    /// ```
    pub fn origin() -> Self
    where
        S: Zero,
    {
        Self {
            coords: std::array::from_fn(|_| S::zero()),
        }
    }

    /// 返回维度
    /// Return the dimension
    pub fn dim(&self) -> usize {
        D
    }

    /// 获取坐标的不可变引用
    /// Get an immutable reference to coordinates
    pub fn coords(&self) -> &[S; D] {
        &self.coords
    }

    /// 获取坐标的可变引用
    /// Get a mutable reference to coordinates
    pub fn coords_mut(&mut self) -> &mut [S; D] {
        &mut self.coords
    }

    /// 获取指定维度的坐标值
    /// Get the coordinate value at the specified dimension
    pub fn get(&self, i: usize) -> Option<S> {
        self.coords.get(i).copied()
    }

    /// 设置指定维度的坐标值
    /// Set the coordinate value at the specified dimension
    pub fn set(&mut self, i: usize, value: S) -> bool {
        if i < D {
            self.coords[i] = value;
            true
        } else {
            false
        }
    }
}

// ============================================================================
// 2D 便捷方法 / 2D convenience methods
// ============================================================================

impl<S: Field + Float> Point<2, S> {
    /// 创建 2D 点
    /// Create a 2D point
    pub fn new(x: S, y: S) -> Self {
        Self::from_coords([x, y])
    }

    /// 获取 x 坐标
    /// Get x coordinate
    pub fn x(&self) -> S {
        self.coords[0]
    }

    /// 获取 y 坐标
    /// Get y coordinate
    pub fn y(&self) -> S {
        self.coords[1]
    }

    /// 设置 x 坐标
    /// Set x coordinate
    pub fn set_x(&mut self, x: S) {
        self.coords[0] = x;
    }

    /// 设置 y 坐标
    /// Set y coordinate
    pub fn set_y(&mut self, y: S) {
        self.coords[1] = y;
    }
}

// ============================================================================
// 3D 便捷方法 / 3D convenience methods
// ============================================================================

impl<S: Field + Float> Point<3, S> {
    /// 创建 3D 点
    /// Create a 3D point
    pub fn new(x: S, y: S, z: S) -> Self {
        Self::from_coords([x, y, z])
    }

    /// 获取 x 坐标
    /// Get x coordinate
    pub fn x(&self) -> S {
        self.coords[0]
    }

    /// 获取 y 坐标
    /// Get y coordinate
    pub fn y(&self) -> S {
        self.coords[1]
    }

    /// 获取 z 坐标
    /// Get z coordinate
    pub fn z(&self) -> S {
        self.coords[2]
    }

    /// 设置 x 坐标
    /// Set x coordinate
    pub fn set_x(&mut self, x: S) {
        self.coords[0] = x;
    }

    /// 设置 y 坐标
    /// Set y coordinate
    pub fn set_y(&mut self, y: S) {
        self.coords[1] = y;
    }

    /// 设置 z 坐标
    /// Set z coordinate
    pub fn set_z(&mut self, z: S) {
        self.coords[2] = z;
    }
}

// ============================================================================
// 距离和几何操作 / Distance and geometric operations
// ============================================================================

impl<const D: usize, S: Field + Float> Point<D, S> {
    /// 计算到另一个点的欧几里得距离
    /// Calculate the Euclidean distance to another point
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::Point2;
    ///
    /// let p1 = Point2::new(0.0, 0.0);
    /// let p2 = Point2::new(3.0, 4.0);
    /// let diff: f64 = p1.distance(&p2) - 5.0;
    /// assert!(diff.abs() < 1e-10);
    /// ```
    pub fn distance(&self, other: &Self) -> S {
        Euclidean.distance(&self.coords, &other.coords)
    }

    /// 使用指定距离度量计算到另一个点的距离
    /// Calculate the distance to another point using a specified metric
    pub fn distance_with<DIST: Distance<S>>(&self, other: &Self, metric: &DIST) -> S {
        metric.distance(&self.coords, &other.coords)
    }

    /// 计算距离的平方（避免开方运算）
    /// Calculate the squared distance (avoiding square root)
    pub fn distance_squared(&self, other: &Self) -> S {
        self.coords
            .iter()
            .zip(other.coords.iter())
            .fold(S::zero(), |acc, (&a, &b)| {
                let diff = a - b;
                acc + diff * diff
            })
    }

    /// 计算两点的中点
    /// Calculate the midpoint between two points
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::Point2;
    ///
    /// let p1 = Point2::new(0.0, 0.0);
    /// let p2 = Point2::new(4.0, 6.0);
    /// let mid = p1.midpoint(&p2);
    /// let x_diff: f64 = mid.x() - 2.0;
    /// let y_diff: f64 = mid.y() - 3.0;
    /// assert!(x_diff.abs() < 1e-10);
    /// assert!(y_diff.abs() < 1e-10);
    /// ```
    pub fn midpoint(&self, other: &Self) -> Self
    where
        S: One,
    {
        let two = S::one() + S::one();
        Self::from_coords(std::array::from_fn(|i| {
            (self.coords[i] + other.coords[i]) / two
        }))
    }

    /// 计算多点的质心
    /// Calculate the centroid of multiple points
    pub fn centroid(points: &[Self]) -> Self
    where
        S: One,
    {
        if points.is_empty() {
            return Self::origin();
        }

        let n = S::from(points.len() as f64).unwrap_or_else(S::one);
        let sum = points.iter().fold(Self::origin(), |acc, p| acc + p.clone());
        sum.scale(S::one() / n)
    }
}

// ============================================================================
// 类型别名 / Type aliases
// ============================================================================

/// 2D 点类型别名
/// 2D point type alias
pub type Point2<S = f64> = Point<2, S>;

/// 3D 点类型别名
/// 3D point type alias
pub type Point3<S = f64> = Point<3, S>;

/// 4D 点类型别名
/// 4D point type alias
pub type Point4<S = f64> = Point<4, S>;

// ============================================================================
// Trait 实现 / Trait implementations
// ============================================================================

impl<const D: usize, S: Field + Float> Index<usize> for Point<D, S> {
    type Output = S;

    fn index(&self, index: usize) -> &Self::Output {
        &self.coords[index]
    }
}

impl<const D: usize, S: Field + Float> IndexMut<usize> for Point<D, S> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.coords[index]
    }
}

impl<const D: usize, S: Field + Float> Add for Point<D, S> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::from_coords(std::array::from_fn(|i| self.coords[i] + other.coords[i]))
    }
}

impl<const D: usize, S: Field + Float> Sub for Point<D, S> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::from_coords(std::array::from_fn(|i| self.coords[i] - other.coords[i]))
    }
}

impl<const D: usize, S: Field + Float> Neg for Point<D, S> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::from_coords(std::array::from_fn(|i| -self.coords[i]))
    }
}

impl<const D: usize, S: Field + Float> Zero for Point<D, S> {
    fn zero() -> Self {
        Self::origin()
    }

    fn is_zero(&self) -> bool {
        self.coords.iter().all(|c| c.is_zero())
    }
}

impl<const D: usize, S: Field + Float> VectorSpace for Point<D, S> {
    type Scalar = S;

    fn scale(&self, scalar: Self::Scalar) -> Self {
        Self::from_coords(std::array::from_fn(|i| self.coords[i] * scalar))
    }
}

impl<const D: usize, S: Field + Float> NormedSpace for Point<D, S> {
    fn norm(&self) -> Self::Scalar {
        self.coords
            .iter()
            .fold(S::zero(), |acc, &c| acc + c * c)
            .sqrt()
    }
}

impl<const D: usize, S: Field + Float> InnerProductSpace for Point<D, S> {
    fn dot(&self, other: &Self) -> Self::Scalar {
        self.coords
            .iter()
            .zip(other.coords.iter())
            .fold(S::zero(), |acc, (&a, &b)| acc + a * b)
    }
}

impl<const D: usize, S: Field + Float> Debug for Point<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Point{}(", D)?;
        for (i, c) in self.coords.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", c)?;
        }
        write!(f, ")")
    }
}

impl<const D: usize, S: Field + Float + Display> Display for Point<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "(")?;
        for (i, c) in self.coords.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", c)?;
        }
        write!(f, ")")
    }
}

impl<const D: usize, S: Field + Float> Default for Point<D, S> {
    fn default() -> Self {
        Self::origin()
    }
}

// ============================================================================
// 容差比较 / Tolerance comparison
// ============================================================================

impl<const D: usize, S: Field + Float + Epsilon> Point<D, S> {
    /// 使用容差判断两点是否近似相等
    /// Check if two points are approximately equal using tolerance
    pub fn approx_eq(&self, other: &Self) -> bool {
        self.approx_eq_with(other, <S as Epsilon>::epsilon())
    }

    /// 使用指定容差判断两点是否近似相等
    /// Check if two points are approximately equal using specified tolerance
    pub fn approx_eq_with(&self, other: &Self, epsilon: S) -> bool {
        self.coords
            .iter()
            .zip(other.coords.iter())
            .all(|(&a, &b)| (a - b).abs() < epsilon)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point2_creation() {
        let p = Point2::new(1.0, 2.0);
        assert_eq!(p.x(), 1.0);
        assert_eq!(p.y(), 2.0);
        assert_eq!(p.dim(), 2);
    }

    #[test]
    fn test_point3_creation() {
        let p = Point3::new(1.0, 2.0, 3.0);
        assert_eq!(p.x(), 1.0);
        assert_eq!(p.y(), 2.0);
        assert_eq!(p.z(), 3.0);
        assert_eq!(p.dim(), 3);
    }

    #[test]
    fn test_point_operations() {
        let p1 = Point2::new(1.0, 2.0);
        let p2 = Point2::new(3.0, 4.0);

        // 加法 / Addition
        let sum = p1.clone() + p2.clone();
        assert_eq!(sum.x(), 4.0);
        assert_eq!(sum.y(), 6.0);

        // 减法 / Subtraction
        let diff = p2.clone() - p1.clone();
        assert_eq!(diff.x(), 2.0);
        assert_eq!(diff.y(), 2.0);

        // 标量乘法 / Scalar multiplication
        let scaled = p1.scale(2.0);
        assert_eq!(scaled.x(), 2.0);
        assert_eq!(scaled.y(), 4.0);
    }

    #[test]
    fn test_distance() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(3.0, 4.0);

        // 欧几里得距离 / Euclidean distance
        assert!((p1.distance(&p2) - 5.0).abs() < 1e-10);

        // 距离平方 / Squared distance
        assert!((p1.distance_squared(&p2) - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_midpoint() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(4.0, 6.0);

        let mid = p1.midpoint(&p2);
        assert!((mid.x() - 2.0).abs() < 1e-10);
        assert!((mid.y() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_centroid() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(2.0, 3.0),
        ];

        let centroid = Point2::centroid(&points);
        assert!((centroid.x() - 2.0).abs() < 1e-10);
        assert!((centroid.y() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_norm_and_dot() {
        let p = Point2::new(3.0, 4.0);

        // 范数 / Norm
        assert!((p.norm() - 5.0).abs() < 1e-10);

        // 内积 / Inner product
        let p1 = Point2::new(1.0, 2.0);
        let p2 = Point2::new(3.0, 4.0);
        assert!((p1.dot(&p2) - 11.0).abs() < 1e-10);
    }

    #[test]
    fn test_approx_eq() {
        let p1 = Point2::new(1.0, 2.0);
        // 使用一个很小的差值进行测试
        // Use a very small difference for testing
        let diff = 1e-15;
        let p2 = Point2::new(1.0 + diff, 2.0 + diff);

        // 使用默认 epsilon（项目的 Epsilon trait 为 f64 定义为 1e-10）
        // Use default epsilon (project's Epsilon trait defines 1e-10 for f64)
        assert!(p1.approx_eq(&p2));

        // 使用更小的容差应该不相等
        // Should not be equal with smaller tolerance
        assert!(!p1.approx_eq_with(&p2, 1e-16));
    }

    #[test]
    fn test_index() {
        let p = Point3::new(1.0, 2.0, 3.0);

        assert_eq!(p[0], 1.0);
        assert_eq!(p[1], 2.0);
        assert_eq!(p[2], 3.0);
    }

    #[test]
    fn test_zero() {
        let zero: Point2 = Point2::zero();
        assert!(zero.is_zero());
        assert_eq!(zero.x(), 0.0);
        assert_eq!(zero.y(), 0.0);
    }
}
