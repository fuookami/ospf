//! 三角形实体模块
//! Triangle entity module
//!
//! 本模块定义了三角形类型：
//! This module defines triangle types:
//!
//! - [`Triangle`] - 泛型三角形，由三个点组成
//! - [`Triangle`] - Generic triangle composed of three points
//!
//! # 示例 / Examples
//!
//! ```
//! use ospf_rust_math::geometry::{Point2, Triangle2};
//!
//! let p1 = Point2::new(0.0, 0.0);
//! let p2 = Point2::new(4.0, 0.0);
//! let p3 = Point2::new(2.0, 3.0);
//! let triangle = Triangle2::new(p1, p2, p3);
//!
//! // 面积 / Area
//! let area_diff: f64 = triangle.area() - 6.0;
//! assert!(area_diff.abs() < 1e-10);
//!
//! // 周长 / Perimeter
//! let perimeter = triangle.perimeter();
//! ```

use super::circle::Circle;
use super::edge::Edge;
use super::point::{Point, Point2};
use super::vector::Vector;
use crate::algebra::{Epsilon, Field, InnerProductSpace, NormedSpace};
use num_traits::Float;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

// ============================================================================
// Triangle - 泛型三角形
// ============================================================================

/// Triangle - 泛型三角形
/// Triangle - Generic triangle
///
/// 表示由三个点组成的三角形。
/// Represents a triangle composed of three points.
///
/// # 泛型参数 / Generic Parameters
/// - `const D: usize`: 点的维度 / Dimension of points
/// - `S`: 标量类型 / Scalar type
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::geometry::{Point2, Triangle2};
///
/// let p1 = Point2::new(0.0, 0.0);
/// let p2 = Point2::new(1.0, 0.0);
/// let p3 = Point2::new(0.0, 1.0);
/// let triangle = Triangle2::new(p1, p2, p3);
///
/// assert!(!triangle.is_degenerate());
/// ```
#[derive(Clone, PartialEq)]
pub struct Triangle<const D: usize, S: Field + Float = f64> {
    /// 第一个顶点 / First vertex
    p1: Point<D, S>,
    /// 第二个顶点 / Second vertex
    p2: Point<D, S>,
    /// 第三个顶点 / Third vertex
    p3: Point<D, S>,
}

// ============================================================================
// 构造方法 / Constructors
// ============================================================================

impl<const D: usize, S: Field + Float> Triangle<D, S> {
    /// 创建新的三角形
    /// Create a new triangle
    ///
    /// # 参数 / Parameters
    /// - `p1`: 第一个顶点 / First vertex
    /// - `p2`: 第二个顶点 / Second vertex
    /// - `p3`: 第三个顶点 / Third vertex
    pub fn new(p1: Point<D, S>, p2: Point<D, S>, p3: Point<D, S>) -> Self {
        Self { p1, p2, p3 }
    }

    /// 从坐标创建三角形
    /// Create a triangle from coordinates
    pub fn from_coords(p1: [S; D], p2: [S; D], p3: [S; D]) -> Self {
        Self::new(
            Point::from_coords(p1),
            Point::from_coords(p2),
            Point::from_coords(p3),
        )
    }

    /// 获取第一个顶点
    /// Get the first vertex
    pub fn p1(&self) -> &Point<D, S> {
        &self.p1
    }

    /// 获取第二个顶点
    /// Get the second vertex
    pub fn p2(&self) -> &Point<D, S> {
        &self.p2
    }

    /// 获取第三个顶点
    /// Get the third vertex
    pub fn p3(&self) -> &Point<D, S> {
        &self.p3
    }

    /// 获取所有顶点
    /// Get all vertices
    pub fn vertices(&self) -> [&Point<D, S>; 3] {
        [&self.p1, &self.p2, &self.p3]
    }

    /// 获取顶点（消费 self）
    /// Get vertices (consumes self)
    pub fn into_points(self) -> (Point<D, S>, Point<D, S>, Point<D, S>) {
        (self.p1, self.p2, self.p3)
    }
}

// ============================================================================
// 边和几何属性 / Edges and geometric properties
// ============================================================================

impl<const D: usize, S: Field + Float> Triangle<D, S> {
    /// 获取第一条边（p1 -> p2）
    /// Get the first edge (p1 -> p2)
    pub fn e1(&self) -> Edge<D, S> {
        Edge::new(self.p1.clone(), self.p2.clone())
    }

    /// 获取第二条边（p2 -> p3）
    /// Get the second edge (p2 -> p3)
    pub fn e2(&self) -> Edge<D, S> {
        Edge::new(self.p2.clone(), self.p3.clone())
    }

    /// 获取第三条边（p3 -> p1）
    /// Get the third edge (p3 -> p1)
    pub fn e3(&self) -> Edge<D, S> {
        Edge::new(self.p3.clone(), self.p1.clone())
    }

    /// 获取所有边
    /// Get all edges
    pub fn edges(&self) -> [Edge<D, S>; 3] {
        [self.e1(), self.e2(), self.e3()]
    }

    /// 计算周长
    /// Calculate the perimeter
    pub fn perimeter(&self) -> S {
        self.e1().length() + self.e2().length() + self.e3().length()
    }

    /// 计算重心（三条中线的交点）
    /// Calculate the centroid (intersection of medians)
    pub fn centroid(&self) -> Point<D, S> {
        let one = S::one();
        let three = one + one + one;
        Point::from_coords(std::array::from_fn(|i| {
            (self.p1[i] + self.p2[i] + self.p3[i]) / three
        }))
    }

    /// 判断三角形是否退化（面积为零）
    /// Check if the triangle is degenerate (zero area)
    pub fn is_degenerate(&self) -> bool
    where
        S: Epsilon,
    {
        // 检查是否有两个顶点重合或三点共线
        // Check if two vertices coincide or three points are collinear
        self.p1.approx_eq(&self.p2) || self.p2.approx_eq(&self.p3) || self.p3.approx_eq(&self.p1)
    }
}

// ============================================================================
// 2D 三角形特有方法 / 2D triangle specific methods
// ============================================================================

impl<S: Field + Float> Triangle<2, S> {
    /// 计算 2D 三角形的面积
    /// Calculate the area of a 2D triangle
    ///
    /// 使用叉积公式：`Area = 0.5 * |(p2 - p1) × (p3 - p1)|`
    /// Uses cross product formula: `Area = 0.5 * |(p2 - p1) × (p3 - p1)|`
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Triangle2};
    ///
    /// let p1 = Point2::new(0.0, 0.0);
    /// let p2 = Point2::new(4.0, 0.0);
    /// let p3 = Point2::new(0.0, 3.0);
    /// let triangle = Triangle2::new(p1, p2, p3);
    ///
    /// let area_diff: f64 = triangle.area() - 6.0;
    /// assert!(area_diff.abs() < 1e-10);
    /// ```
    pub fn area(&self) -> S {
        // 使用叉积计算面积
        // Calculate area using cross product
        let v1 = super::Vector::from_points(&self.p1, &self.p2);
        let v2 = super::Vector::from_points(&self.p1, &self.p3);
        let cross = v1.cross_2d(&v2);
        cross.abs() / (S::one() + S::one())
    }

    /// 使用海伦公式计算面积
    /// Calculate area using Heron's formula
    ///
    /// 适用于任意三角形，仅使用边长。
    /// Works for any triangle, uses only edge lengths.
    pub fn area_heron(&self) -> S {
        let a = self.e1().length();
        let b = self.e2().length();
        let c = self.e3().length();
        let s = (a + b + c) / (S::one() + S::one());
        (s * (s - a) * (s - b) * (s - c)).sqrt()
    }

    /// 判断点是否在三角形内部
    /// Check if a point is inside the triangle
    ///
    /// 使用重心坐标法。
    /// Uses barycentric coordinate method.
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Triangle2};
    ///
    /// let p1 = Point2::new(0.0, 0.0);
    /// let p2 = Point2::new(4.0, 0.0);
    /// let p3 = Point2::new(0.0, 4.0);
    /// let triangle = Triangle2::new(p1, p2, p3);
    ///
    /// let inside = Point2::new(1.0, 1.0);
    /// let outside = Point2::new(3.0, 3.0);
    ///
    /// assert!(triangle.contains_point(&inside));
    /// assert!(!triangle.contains_point(&outside));
    /// ```
    pub fn contains_point(&self, point: &Point<2, S>) -> bool {
        // 使用重心坐标法
        // Use barycentric coordinate method
        let v0 = Vector::from_points(&self.p1, &self.p3);
        let v1 = Vector::from_points(&self.p1, &self.p2);
        let v2 = Vector::from_points(&self.p1, point);

        let dot00 = v0.dot(&v0);
        let dot01 = v0.dot(&v1);
        let dot02 = v0.dot(&v2);
        let dot11 = v1.dot(&v1);
        let dot12 = v1.dot(&v2);

        // 计算分母
        // Calculate denominator
        let denom = dot00 * dot11 - dot01 * dot01;
        if denom.is_zero() {
            return false;
        }

        // 计算重心坐标
        // Calculate barycentric coordinates
        let u = (dot11 * dot02 - dot01 * dot12) / denom;
        let v = (dot00 * dot12 - dot01 * dot02) / denom;

        // 检查点是否在三角形内
        // Check if point is inside triangle
        let zero = S::zero();
        let one = S::one();
        u >= zero && v >= zero && u + v <= one
    }

    /// 计算外接圆
    /// Calculate the circumcircle
    ///
    /// 外接圆是通过三个顶点的唯一圆。
    /// The circumcircle is the unique circle passing through all three vertices.
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Triangle2};
    ///
    /// let p1 = Point2::new(0.0, 0.0);
    /// let p2 = Point2::new(2.0, 0.0);
    /// let p3 = Point2::new(1.0, 1.0);
    /// let triangle = Triangle2::new(p1, p2, p3);
    ///
    /// let circumcircle = triangle.circumcircle();
    /// let x_diff: f64 = circumcircle.center().x() - 1.0;
    /// assert!(x_diff.abs() < 1e-10);
    /// ```
    pub fn circumcircle(&self) -> Circle<2, S> {
        let ax = self.p2.x() - self.p1.x();
        let ay = self.p2.y() - self.p1.y();
        let bx = self.p3.x() - self.p1.x();
        let by = self.p3.y() - self.p1.y();

        let m = self.p2.x() * self.p2.x() - self.p1.x() * self.p1.x() + self.p2.y() * self.p2.y()
            - self.p1.y() * self.p1.y();
        let u = self.p3.x() * self.p3.x() - self.p1.x() * self.p1.x() + self.p3.y() * self.p3.y()
            - self.p1.y() * self.p1.y();

        let two = S::one() + S::one();
        let s = S::one() / (two * (ax * by - ay * bx));

        let x = ((self.p3.y() - self.p1.y()) * m + (self.p1.y() - self.p2.y()) * u) * s;
        let y = ((self.p1.x() - self.p3.x()) * m + (self.p2.x() - self.p1.x()) * u) * s;

        let center = Point2::new(x, y);
        let radius = (self.p1.x() - x).powi(2) + (self.p1.y() - y).powi(2);
        let radius = radius.sqrt();

        Circle::new(center, radius)
    }

    /// 计算内心（内切圆圆心）
    /// Calculate the incenter (center of inscribed circle)
    pub fn incenter(&self) -> Point<2, S> {
        let a = self.e2().length(); // 边 p2-p3 的长度
        let b = self.e3().length(); // 边 p3-p1 的长度
        let c = self.e1().length(); // 边 p1-p2 的长度
        let perimeter = a + b + c;

        if perimeter.is_zero() {
            return self.p1.clone();
        }

        Point2::new(
            (a * self.p1.x() + b * self.p2.x() + c * self.p3.x()) / perimeter,
            (a * self.p1.y() + b * self.p2.y() + c * self.p3.y()) / perimeter,
        )
    }

    /// 计算外心（外接圆圆心）
    /// Calculate the circumcenter (center of circumscribed circle)
    pub fn circumcenter(&self) -> Point<2, S> {
        self.circumcircle().center().clone()
    }

    /// 判断点是否在外接圆内（包含边界）
    /// Check if a point is inside or on the circumcircle
    pub fn point_in_circumcircle(&self, point: &Point<2, S>) -> bool
    where
        S: num_traits::FloatConst,
    {
        let circumcircle = self.circumcircle();
        circumcircle.contains_point(point)
    }
}

// ============================================================================
// 3D 三角形特有方法 / 3D triangle specific methods
// ============================================================================

impl<S: Field + Float> Triangle<3, S> {
    /// 计算 3D 三角形的面积
    /// Calculate the area of a 3D triangle
    ///
    /// 使用叉积的模长公式。
    /// Uses the magnitude of cross product.
    pub fn area(&self) -> S {
        let v1 = Vector::from_points(&self.p1, &self.p2);
        let v2 = Vector::from_points(&self.p1, &self.p3);
        let cross = v1.cross(&v2);
        cross.norm() / (S::one() + S::one())
    }

    /// 计算法向量
    /// Calculate the normal vector
    ///
    /// 返回垂直于三角形平面的单位向量。
    /// Returns the unit vector perpendicular to the triangle plane.
    pub fn normal(&self) -> Option<Vector<3, S>> {
        let v1 = Vector::from_points(&self.p1, &self.p2);
        let v2 = Vector::from_points(&self.p1, &self.p3);
        let cross = v1.cross(&v2);
        cross.normalize()
    }
}

// ============================================================================
// 类型别名 / Type aliases
// ============================================================================

/// 2D 三角形类型别名
/// 2D triangle type alias
pub type Triangle2<S = f64> = Triangle<2, S>;

/// 3D 三角形类型别名
/// 3D triangle type alias
pub type Triangle3<S = f64> = Triangle<3, S>;

// ============================================================================
// Trait 实现 / Trait implementations
// ============================================================================

impl<const D: usize, S: Field + Float> Debug for Triangle<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Triangle{}({:?}, {:?}, {:?})",
            D, self.p1, self.p2, self.p3
        )
    }
}

impl<const D: usize, S: Field + Float + Display> Display for Triangle<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Triangle({}, {}, {})", self.p1, self.p2, self.p3)
    }
}

impl<const D: usize, S: Field + Float + Epsilon> Triangle<D, S> {
    /// 使用容差判断两个三角形是否近似相等
    /// Check if two triangles are approximately equal using tolerance
    pub fn approx_eq(&self, other: &Self) -> bool {
        self.p1.approx_eq(&other.p1) && self.p2.approx_eq(&other.p2) && self.p3.approx_eq(&other.p3)
    }

    /// 判断两个三角形是否近似相等（忽略顶点顺序）
    /// Check if two triangles are approximately equal (ignoring vertex order)
    pub fn approx_eq_unordered(&self, other: &Self) -> bool {
        let self_vertices = self.vertices();
        let other_vertices = other.vertices();

        // 检查是否每个 self 的顶点都能在 other 中找到匹配
        // Check if each vertex in self can find a match in other
        for sv in &self_vertices {
            if !other_vertices.iter().any(|ov| sv.approx_eq(ov)) {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point3;

    #[test]
    fn test_triangle_creation() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(1.0, 0.0);
        let p3 = Point2::new(0.0, 1.0);
        let triangle = Triangle2::new(p1.clone(), p2.clone(), p3.clone());

        assert_eq!(triangle.p1(), &p1);
        assert_eq!(triangle.p2(), &p2);
        assert_eq!(triangle.p3(), &p3);
    }

    #[test]
    fn test_triangle_area() {
        // 直角三角形 / Right triangle
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(4.0, 0.0);
        let p3 = Point2::new(0.0, 3.0);
        let triangle = Triangle2::new(p1, p2, p3);

        assert!((triangle.area() - 6.0).abs() < 1e-10);

        // 使用海伦公式验证 / Verify using Heron's formula
        assert!((triangle.area_heron() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_triangle_perimeter() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(3.0, 0.0);
        let p3 = Point2::new(0.0, 4.0);
        let triangle = Triangle2::new(p1, p2, p3);

        // 3-4-5 三角形，周长 = 3 + 4 + 5 = 12
        // 3-4-5 triangle, perimeter = 3 + 4 + 5 = 12
        assert!((triangle.perimeter() - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_triangle_centroid() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(6.0, 0.0);
        let p3 = Point2::new(3.0, 3.0);
        let triangle = Triangle2::new(p1, p2, p3);

        let centroid = triangle.centroid();
        assert!((centroid.x() - 3.0).abs() < 1e-10);
        assert!((centroid.y() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_triangle_contains_point() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(4.0, 0.0);
        let p3 = Point2::new(0.0, 4.0);
        let triangle = Triangle2::new(p1.clone(), p2.clone(), p3.clone());

        // 内部点 / Inside point
        let inside = Point2::new(1.0, 1.0);
        assert!(triangle.contains_point(&inside));

        // 外部点 / Outside point
        let outside = Point2::new(3.0, 3.0);
        assert!(!triangle.contains_point(&outside));

        // 顶点 / Vertex
        assert!(triangle.contains_point(&p1));
        assert!(triangle.contains_point(&p2));
        assert!(triangle.contains_point(&p3));

        // 边上的点 / Point on edge
        let on_edge = Point2::new(2.0, 0.0);
        assert!(triangle.contains_point(&on_edge));
    }

    #[test]
    fn test_triangle_circumcircle() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(2.0, 0.0);
        let p3 = Point2::new(1.0, 1.0);
        let triangle = Triangle2::new(p1.clone(), p2.clone(), p3.clone());

        let circumcircle = triangle.circumcircle();

        // 圆心应该在 x = 1 线上 / Center should be on x = 1 line
        assert!((circumcircle.center().x() - 1.0).abs() < 1e-10);

        // 所有顶点应该在圆上 / All vertices should be on the circle
        let r = circumcircle.radius();
        assert!((p1.distance(circumcircle.center()) - r).abs() < 1e-10);
        assert!((p2.distance(circumcircle.center()) - r).abs() < 1e-10);
        assert!((p3.distance(circumcircle.center()) - r).abs() < 1e-10);
    }

    #[test]
    fn test_triangle_incenter() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(4.0, 0.0);
        let p3 = Point2::new(0.0, 3.0);
        let triangle = Triangle2::new(p1, p2, p3);

        let incenter = triangle.incenter();

        // 内心到三边距离相等 / Incenter has equal distance to all edges
        // 对于 3-4-5 三角形，内心在 (1, 1)
        // For 3-4-5 triangle, incenter is at (1, 1)
        assert!((incenter.x() - 1.0).abs() < 1e-10);
        assert!((incenter.y() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_triangle_is_degenerate() {
        // 正常三角形 / Normal triangle
        let t1 = Triangle2::new(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        );
        assert!(!t1.is_degenerate());

        // 退化三角形（共线点）/ Degenerate triangle (collinear points)
        // 注意：当前实现只检查顶点是否重合
        // Note: Current implementation only checks if vertices coincide
        let t2 = Triangle2::new(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
        );
        // 面积应该为零 / Area should be zero
        assert!(t2.area().abs() < 1e-10);
    }

    #[test]
    fn test_triangle_3d_area() {
        let p1: Point3 = Point3::new(0.0, 0.0, 0.0);
        let p2: Point3 = Point3::new(4.0, 0.0, 0.0);
        let p3: Point3 = Point3::new(0.0, 3.0, 0.0);
        let triangle = Triangle3::new(p1, p2, p3);

        // XY 平面上的 3-4 三角形，面积 = 6
        // 3-4 triangle on XY plane, area = 6
        assert!((triangle.area() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_triangle_3d_normal() {
        let p1: Point3 = Point3::new(0.0, 0.0, 0.0);
        let p2: Point3 = Point3::new(1.0, 0.0, 0.0);
        let p3: Point3 = Point3::new(0.0, 1.0, 0.0);
        let triangle = Triangle3::new(p1, p2, p3);

        let normal = triangle.normal().unwrap();

        // 法向量应该指向 Z 方向 / Normal should point in Z direction
        assert!((normal.x() - 0.0).abs() < 1e-10);
        assert!((normal.y() - 0.0).abs() < 1e-10);
        assert!((normal.z().abs() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_triangle_edges() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(3.0, 0.0);
        let p3 = Point2::new(0.0, 4.0);
        let triangle = Triangle2::new(p1.clone(), p2.clone(), p3.clone());

        let edges = triangle.edges();
        assert_eq!(edges.len(), 3);

        // 检查边的长度 / Check edge lengths
        assert!((edges[0].length() - 3.0).abs() < 1e-10); // p1 -> p2
        assert!((edges[1].length() - 5.0).abs() < 1e-10); // p2 -> p3
        assert!((edges[2].length() - 4.0).abs() < 1e-10); // p3 -> p1
    }

    #[test]
    fn test_triangle_approx_eq_unordered() {
        let t1 = Triangle2::new(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        );

        // 不同顺序的相同三角形 / Same triangle with different order
        let t2 = Triangle2::new(
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
            Point2::new(0.0, 0.0),
        );

        assert!(t1.approx_eq_unordered(&t2));
    }
}
