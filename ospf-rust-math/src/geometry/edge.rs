//! 边实体模块
//! Edge entity module
//!
//! 本模块定义了边类型：
//! This module defines edge types:
//!
//! - [`Edge`] - 泛型边，连接两个点
//! - [`Edge`] - Generic edge connecting two points
//!
//! # 示例 / Examples
//!
//! ```
//! use ospf_rust_math::geometry::{Point2, Edge2};
//!
//! let p1 = Point2::new(0.0, 0.0);
//! let p2 = Point2::new(3.0, 4.0);
//! let edge = Edge2::new(p1, p2);
//!
//! // 长度 / Length
//! let len_diff: f64 = edge.length() - 5.0;
//! assert!(len_diff.abs() < 1e-10);
//!
//! // 中点 / Midpoint
//! let mid = edge.midpoint();
//! let x_diff: f64 = mid.x() - 1.5;
//! assert!(x_diff.abs() < 1e-10);
//! ```

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use num_traits::Float;
use crate::algebra::{Epsilon, Field, InnerProductSpace, NormedSpace};
use super::distance::Distance;
use super::point::Point;
use super::vector::Vector;

// ============================================================================
// Edge - 泛型边
// ============================================================================

/// Edge - 泛型边
/// Edge - Generic edge
///
/// 表示连接两个点的边（线段）。
/// Represents an edge (line segment) connecting two points.
///
/// # 泛型参数 / Generic Parameters
/// - `const D: usize`: 点的维度 / Dimension of points
/// - `S`: 标量类型 / Scalar type
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::geometry::{Point2, Edge2};
///
/// let p1 = Point2::new(0.0, 0.0);
/// let p2 = Point2::new(3.0, 4.0);
/// let edge = Edge2::new(p1, p2);
///
/// let diff: f64 = edge.length() - 5.0;
/// assert!(diff.abs() < 1e-10);
/// ```
#[derive(Clone, PartialEq)]
pub struct Edge<const D: usize, S: Field + Float = f64> {
    /// 起点 / Start point
    from: Point<D, S>,
    /// 终点 / End point
    to: Point<D, S>,
}

// ============================================================================
// 构造方法 / Constructors
// ============================================================================

impl<const D: usize, S: Field + Float> Edge<D, S> {
    /// 创建新的边
    /// Create a new edge
    ///
    /// # 参数 / Parameters
    /// - `from`: 起点 / Start point
    /// - `to`: 终点 / End point
    pub fn new(from: Point<D, S>, to: Point<D, S>) -> Self {
        Self { from, to }
    }

    /// 从坐标创建边
    /// Create an edge from coordinates
    pub fn from_coords(from: [S; D], to: [S; D]) -> Self {
        Self::new(Point::from_coords(from), Point::from_coords(to))
    }

    /// 获取起点
    /// Get the start point
    pub fn from(&self) -> &Point<D, S> {
        &self.from
    }

    /// 获取终点
    /// Get the end point
    pub fn to(&self) -> &Point<D, S> {
        &self.to
    }

    /// 获取起点（消费 self）
    /// Get the start point (consumes self)
    pub fn into_points(self) -> (Point<D, S>, Point<D, S>) {
        (self.from, self.to)
    }
}

// ============================================================================
// 几何属性 / Geometric properties
// ============================================================================

impl<const D: usize, S: Field + Float> Edge<D, S> {
    /// 计算边的长度
    /// Calculate the length of the edge
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Edge2};
    ///
    /// let p1 = Point2::new(0.0, 0.0);
    /// let p2 = Point2::new(3.0, 4.0);
    /// let edge = Edge2::new(p1, p2);
    /// let diff: f64 = edge.length() - 5.0;
    /// assert!(diff.abs() < 1e-10);
    /// ```
    pub fn length(&self) -> S {
        self.from.distance(&self.to)
    }

    /// 使用指定距离度量计算边的长度
    /// Calculate the length using a specified distance metric
    pub fn length_with<DIST: Distance<S>>(&self, metric: &DIST) -> S {
        self.from.distance_with(&self.to, metric)
    }

    /// 计算长度平方（避免开方）
    /// Calculate squared length (avoiding square root)
    pub fn length_squared(&self) -> S {
        self.from.distance_squared(&self.to)
    }

    /// 计算边的中点
    /// Calculate the midpoint of the edge
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Edge2};
    ///
    /// let p1 = Point2::new(0.0, 0.0);
    /// let p2 = Point2::new(4.0, 6.0);
    /// let edge = Edge2::new(p1, p2);
    /// let mid = edge.midpoint();
    /// let x_diff: f64 = mid.x() - 2.0;
    /// let y_diff: f64 = mid.y() - 3.0;
    /// assert!(x_diff.abs() < 1e-10);
    /// assert!(y_diff.abs() < 1e-10);
    /// ```
    pub fn midpoint(&self) -> Point<D, S> {
        self.from.midpoint(&self.to)
    }

    /// 获取方向向量（从起点指向终点）
    /// Get the direction vector (from start to end)
    pub fn direction(&self) -> Vector<D, S> {
        Vector::from_points(&self.from, &self.to)
    }

    /// 获取单位方向向量
    /// Get the unit direction vector
    pub fn unit_direction(&self) -> Option<Vector<D, S>> {
        self.direction().normalize()
    }

    /// 计算边上的点（参数化）
    /// Calculate a point on the edge (parametric)
    ///
    /// # 参数 / Parameters
    /// - `t`: 参数，0.0 返回起点，1.0 返回终点
    /// - `t`: Parameter, 0.0 returns start, 1.0 returns end
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Edge2};
    ///
    /// let p1 = Point2::new(0.0, 0.0);
    /// let p2 = Point2::new(4.0, 6.0);
    /// let edge = Edge2::new(p1, p2);
    ///
    /// let q = edge.point_at(0.5);  // 中点 / Midpoint
    /// let diff: f64 = q.x() - 2.0;
    /// assert!(diff.abs() < 1e-10);
    /// ```
    pub fn point_at(&self, t: S) -> Point<D, S> {
        // from + t * (to - from)
        Point::from_coords(std::array::from_fn(|i| {
            self.from[i] + t * (self.to[i] - self.from[i])
        }))
    }

    /// 判断点是否在边上
    /// Check if a point lies on the edge
    pub fn contains_point(&self, point: &Point<D, S>, epsilon: S) -> bool {
        // 检查点到起点和终点的距离之和是否等于边长
        // Check if the sum of distances from point to start and end equals edge length
        let dist_to_from = point.distance(&self.from);
        let dist_to_to = point.distance(&self.to);
        let length = self.length();

        (dist_to_from + dist_to_to - length).abs() < epsilon
    }
}

// ============================================================================
// 2D 边特有方法 / 2D edge specific methods
// ============================================================================

impl<S: Field + Float> Edge<2, S> {
    /// 判断两条 2D 边是否相交
    /// Check if two 2D edges intersect
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Edge2};
    ///
    /// let e1 = Edge2::new(Point2::new(0.0, 0.0), Point2::new(2.0, 2.0));
    /// let e2 = Edge2::new(Point2::new(0.0, 2.0), Point2::new(2.0, 0.0));
    /// assert!(e1.intersects(&e2));
    ///
    /// let e3 = Edge2::new(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0));
    /// let e4 = Edge2::new(Point2::new(0.0, 1.0), Point2::new(1.0, 1.0));
    /// assert!(!e3.intersects(&e4));
    /// ```
    pub fn intersects(&self, other: &Self) -> bool {
        self.intersection_point(other).is_some()
    }

    /// 计算两条 2D 边的交点
    /// Calculate the intersection point of two 2D edges
    ///
    /// 返回 `None` 如果边不相交或重合。
    /// Returns `None` if edges don't intersect or are collinear.
    pub fn intersection_point(&self, other: &Self) -> Option<Point<2, S>> {
        // 使用参数化方法计算交点
        // Use parametric method to calculate intersection
        let p1 = &self.from;
        let p2 = &self.to;
        let p3 = &other.from;
        let p4 = &other.to;

        let d1 = Vector::from_points(p1, p2);
        let d2 = Vector::from_points(p3, p4);

        // 求解 p1 + t * d1 = p3 + s * d2
        // Solve p1 + t * d1 = p3 + s * d2
        let denom = d1.x() * d2.y() - d1.y() * d2.x();

        if denom.is_zero() {
            // 平行或重合 / Parallel or collinear
            return None;
        }

        let dx = p3.x() - p1.x();
        let dy = p3.y() - p1.y();

        let t = (dx * d2.y() - dy * d2.x()) / denom;
        let s = (dx * d1.y() - dy * d1.x()) / denom;

        // 检查 t 和 s 是否在 [0, 1] 范围内
        // Check if t and s are in [0, 1] range
        let zero = S::zero();
        let one = S::one();

        if t >= zero && t <= one && s >= zero && s <= one {
            Some(self.point_at(t))
        } else {
            None
        }
    }

    /// 计算边到点的最近点
    /// Calculate the closest point on the edge to a given point
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Edge2};
    ///
    /// let edge = Edge2::new(Point2::new(0.0, 0.0), Point2::new(4.0, 0.0));
    /// let p = Point2::new(2.0, 3.0);
    /// let closest = edge.closest_point(&p);
    /// let x_diff: f64 = closest.x() - 2.0;
    /// let y_diff: f64 = closest.y() - 0.0;
    /// assert!(x_diff.abs() < 1e-10);
    /// assert!(y_diff.abs() < 1e-10);
    /// ```
    pub fn closest_point(&self, point: &Point<2, S>) -> Point<2, S> {
        let direction = self.direction();
        let to_point = Vector::from_points(&self.from, point);

        let length_sq = direction.dot(&direction);
        if length_sq.is_zero() {
            return self.from.clone();
        }

        let t = to_point.dot(&direction) / length_sq;

        // 限制 t 在 [0, 1] 范围内
        // Clamp t to [0, 1] range
        let zero = S::zero();
        let one = S::one();
        let t_clamped = if t < zero {
            zero
        } else if t > one {
            one
        } else {
            t
        };

        self.point_at(t_clamped)
    }

    /// 计算点到边的距离
    /// Calculate the distance from a point to the edge
    pub fn distance_to_point(&self, point: &Point<2, S>) -> S {
        let closest = self.closest_point(point);
        point.distance(&closest)
    }
}

// ============================================================================
// 类型别名 / Type aliases
// ============================================================================

/// 2D 边类型别名
/// 2D edge type alias
pub type Edge2<S = f64> = Edge<2, S>;

/// 3D 边类型别名
/// 3D edge type alias
pub type Edge3<S = f64> = Edge<3, S>;

// ============================================================================
// Trait 实现 / Trait implementations
// ============================================================================

impl<const D: usize, S: Field + Float> Debug for Edge<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Edge{}({:?} -> {:?})", D, self.from, self.to)
    }
}

impl<const D: usize, S: Field + Float + Display> Display for Edge<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} -> {}", self.from, self.to)
    }
}

impl<const D: usize, S: Field + Float + Epsilon> Edge<D, S> {
    /// 使用容差判断两条边是否近似相等
    /// Check if two edges are approximately equal using tolerance
    pub fn approx_eq(&self, other: &Self) -> bool {
        self.from.approx_eq(&other.from) && self.to.approx_eq(&other.to)
    }

    /// 判断两条边是否近似相等（忽略方向）
    /// Check if two edges are approximately equal (ignoring direction)
    pub fn approx_eq_undirected(&self, other: &Self) -> bool {
        (self.from.approx_eq(&other.from) && self.to.approx_eq(&other.to))
            || (self.from.approx_eq(&other.to) && self.to.approx_eq(&other.from))
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point2, Point3};

    #[test]
    fn test_edge_creation() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(3.0, 4.0);
        let edge = Edge2::new(p1.clone(), p2.clone());

        assert_eq!(edge.from(), &p1);
        assert_eq!(edge.to(), &p2);
    }

    #[test]
    fn test_edge_length() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(3.0, 4.0);
        let edge = Edge2::new(p1, p2);

        assert!((edge.length() - 5.0).abs() < 1e-10);
        assert!((edge.length_squared() - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_edge_midpoint() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(4.0, 6.0);
        let edge = Edge2::new(p1, p2);

        let mid = edge.midpoint();
        assert!((mid.x() - 2.0).abs() < 1e-10);
        assert!((mid.y() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_edge_direction() {
        let p1 = Point2::new(1.0, 2.0);
        let p2 = Point2::new(4.0, 6.0);
        let edge = Edge2::new(p1, p2);

        let dir = edge.direction();
        assert_eq!(dir.x(), 3.0);
        assert_eq!(dir.y(), 4.0);
    }

    #[test]
    fn test_edge_point_at() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(4.0, 6.0);
        let edge = Edge2::new(p1, p2);

        // t = 0 返回起点 / t = 0 returns start
        let q0 = edge.point_at(0.0);
        assert!((q0.x() - 0.0).abs() < 1e-10);
        assert!((q0.y() - 0.0).abs() < 1e-10);

        // t = 1 返回终点 / t = 1 returns end
        let q1 = edge.point_at(1.0);
        assert!((q1.x() - 4.0).abs() < 1e-10);
        assert!((q1.y() - 6.0).abs() < 1e-10);

        // t = 0.5 返回中点 / t = 0.5 returns midpoint
        let q_half = edge.point_at(0.5);
        assert!((q_half.x() - 2.0).abs() < 1e-10);
        assert!((q_half.y() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_edge_intersection() {
        // 相交的边 / Intersecting edges
        let e1 = Edge2::new(Point2::new(0.0, 0.0), Point2::new(2.0, 2.0));
        let e2 = Edge2::new(Point2::new(0.0, 2.0), Point2::new(2.0, 0.0));

        assert!(e1.intersects(&e2));

        let intersection = e1.intersection_point(&e2).unwrap();
        assert!((intersection.x() - 1.0).abs() < 1e-10);
        assert!((intersection.y() - 1.0).abs() < 1e-10);

        // 不相交的边 / Non-intersecting edges
        let e3 = Edge2::new(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0));
        let e4 = Edge2::new(Point2::new(0.0, 1.0), Point2::new(1.0, 1.0));

        assert!(!e3.intersects(&e4));
        assert!(e3.intersection_point(&e4).is_none());
    }

    #[test]
    fn test_edge_closest_point() {
        let edge = Edge2::new(Point2::new(0.0, 0.0), Point2::new(4.0, 0.0));

        // 点在边正上方 / Point directly above edge
        let p1 = Point2::new(2.0, 3.0);
        let closest1 = edge.closest_point(&p1);
        assert!((closest1.x() - 2.0).abs() < 1e-10);
        assert!((closest1.y() - 0.0).abs() < 1e-10);

        // 点在边延长线上 / Point on extended line
        let p2 = Point2::new(5.0, 0.0);
        let closest2 = edge.closest_point(&p2);
        assert!((closest2.x() - 4.0).abs() < 1e-10); // 限制在边上 / Clamped to edge

        // 点在起点外侧 / Point outside start
        let p3 = Point2::new(-1.0, 2.0);
        let closest3 = edge.closest_point(&p3);
        assert!((closest3.x() - 0.0).abs() < 1e-10); // 限制在边上 / Clamped to edge
    }

    #[test]
    fn test_edge_contains_point() {
        let edge = Edge2::new(Point2::new(0.0, 0.0), Point2::new(4.0, 0.0));

        // 点在边上 / Point on edge
        let p1 = Point2::new(2.0, 0.0);
        assert!(edge.contains_point(&p1, 1e-10));

        // 点不在边上 / Point not on edge
        let p2 = Point2::new(2.0, 1.0);
        assert!(!edge.contains_point(&p2, 1e-10));
    }

    #[test]
    fn test_edge_approx_eq() {
        let e1 = Edge2::new(Point2::new(0.0, 0.0), Point2::new(3.0, 4.0));
        // 使用一个很小的差值进行测试
        // Use a very small difference for testing
        let diff = 1e-15;
        let e2 = Edge2::new(
            Point2::new(0.0 + diff, 0.0 + diff),
            Point2::new(3.0 + diff, 4.0 + diff),
        );

        // 使用默认 epsilon（项目的 Epsilon trait 为 f64 定义为 1e-10）
        // Use default epsilon (project's Epsilon trait defines 1e-10 for f64)
        assert!(e1.approx_eq(&e2));
    }

    #[test]
    fn test_edge_approx_eq_undirected() {
        let e1 = Edge2::new(Point2::new(0.0, 0.0), Point2::new(3.0, 4.0));
        let e2 = Edge2::new(Point2::new(3.0, 4.0), Point2::new(0.0, 0.0));

        assert!(e1.approx_eq_undirected(&e2));
    }

    #[test]
    fn test_edge_3d() {
        let p1 = Point3::new(1.0, 2.0, 3.0);
        let p2 = Point3::new(4.0, 6.0, 8.0);
        let edge = Edge3::new(p1, p2);

        // 长度：sqrt(3^2 + 4^2 + 5^2) = sqrt(50) ≈ 7.07
        let expected_length = (3.0_f64 * 3.0 + 4.0 * 4.0 + 5.0 * 5.0).sqrt();
        assert!((edge.length() - expected_length).abs() < 1e-10);

        // 方向向量 / Direction vector
        let dir = edge.direction();
        assert_eq!(dir.x(), 3.0);
        assert_eq!(dir.y(), 4.0);
        assert_eq!(dir.z(), 5.0);
    }
}
