//! 四边形实体模块
//! Quadrilateral entity module
//!
//! 本模块定义了四边形类型：
//! This module defines quadrilateral types:
//!
//! - [`Quadrilateral`] - 泛型四边形，由四个点组成
//! - [`Quadrilateral`] - Generic quadrilateral composed of four points
//!
//! # 示例 / Examples
//!
//! ```
//! use ospf_rust_math::geometry::{Point2, Quadrilateral2};
//!
//! let p1 = Point2::new(0.0, 0.0);
//! let p2 = Point2::new(4.0, 0.0);
//! let p3 = Point2::new(4.0, 3.0);
//! let p4 = Point2::new(0.0, 3.0);
//! let quad = Quadrilateral2::new(p1, p2, p3, p4);
//!
//! // 面积 / Area
//! let area_diff: f64 = quad.area() - 12.0;
//! assert!(area_diff.abs() < 1e-10);
//! ```

use super::edge::Edge;
use super::point::Point;
use super::triangle::Triangle;
use crate::algebra::{Epsilon, Field, InnerProductSpace};
use num_traits::Float;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

// ============================================================================
// Quadrilateral - 泛型四边形
// ============================================================================

/// Quadrilateral - 泛型四边形
/// Quadrilateral - Generic quadrilateral
///
/// 表示由四个点组成的四边形。
/// Represents a quadrilateral composed of four points.
///
/// # 泛型参数 / Generic Parameters
/// - `const D: usize`: 点的维度 / Dimension of points
/// - `S`: 标量类型 / Scalar type
///
/// # 注意 / Note
/// 顶点应按顺时针或逆时针顺序给出。
/// Vertices should be given in clockwise or counter-clockwise order.
#[derive(Clone, PartialEq)]
pub struct Quadrilateral<const D: usize, S: Field + Float = f64> {
    /// 第一个顶点 / First vertex
    p1: Point<D, S>,
    /// 第二个顶点 / Second vertex
    p2: Point<D, S>,
    /// 第三个顶点 / Third vertex
    p3: Point<D, S>,
    /// 第四个顶点 / Fourth vertex
    p4: Point<D, S>,
}

// ============================================================================
// 构造方法 / Constructors
// ============================================================================

impl<const D: usize, S: Field + Float> Quadrilateral<D, S> {
    /// 创建新的四边形
    /// Create a new quadrilateral
    ///
    /// # 参数 / Parameters
    /// - `p1`: 第一个顶点 / First vertex
    /// - `p2`: 第二个顶点 / Second vertex
    /// - `p3`: 第三个顶点 / Third vertex
    /// - `p4`: 第四个顶点 / Fourth vertex
    pub fn new(p1: Point<D, S>, p2: Point<D, S>, p3: Point<D, S>, p4: Point<D, S>) -> Self {
        Self { p1, p2, p3, p4 }
    }

    /// 从坐标创建四边形
    /// Create a quadrilateral from coordinates
    pub fn from_coords(p1: [S; D], p2: [S; D], p3: [S; D], p4: [S; D]) -> Self {
        Self::new(
            Point::from_coords(p1),
            Point::from_coords(p2),
            Point::from_coords(p3),
            Point::from_coords(p4),
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

    /// 获取第四个顶点
    /// Get the fourth vertex
    pub fn p4(&self) -> &Point<D, S> {
        &self.p4
    }

    /// 获取所有顶点
    /// Get all vertices
    pub fn vertices(&self) -> [&Point<D, S>; 4] {
        [&self.p1, &self.p2, &self.p3, &self.p4]
    }

    /// 获取顶点（消费 self）
    /// Get vertices (consumes self)
    pub fn into_points(self) -> (Point<D, S>, Point<D, S>, Point<D, S>, Point<D, S>) {
        (self.p1, self.p2, self.p3, self.p4)
    }
}

// ============================================================================
// 边和几何属性 / Edges and geometric properties
// ============================================================================

impl<const D: usize, S: Field + Float> Quadrilateral<D, S> {
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

    /// 获取第三条边（p3 -> p4）
    /// Get the third edge (p3 -> p4)
    pub fn e3(&self) -> Edge<D, S> {
        Edge::new(self.p3.clone(), self.p4.clone())
    }

    /// 获取第四条边（p4 -> p1）
    /// Get the fourth edge (p4 -> p1)
    pub fn e4(&self) -> Edge<D, S> {
        Edge::new(self.p4.clone(), self.p1.clone())
    }

    /// 获取所有边
    /// Get all edges
    pub fn edges(&self) -> [Edge<D, S>; 4] {
        [self.e1(), self.e2(), self.e3(), self.e4()]
    }

    /// 获取两条对角线
    /// Get the two diagonals
    pub fn diagonals(&self) -> [Edge<D, S>; 2] {
        [
            Edge::new(self.p1.clone(), self.p3.clone()),
            Edge::new(self.p2.clone(), self.p4.clone()),
        ]
    }

    /// 计算周长
    /// Calculate the perimeter
    pub fn perimeter(&self) -> S {
        self.e1().length() + self.e2().length() + self.e3().length() + self.e4().length()
    }

    /// 计算重心
    /// Calculate the centroid
    pub fn centroid(&self) -> Point<D, S> {
        let four = S::one() + S::one() + S::one() + S::one();
        Point::from_coords(std::array::from_fn(|i| {
            (self.p1[i] + self.p2[i] + self.p3[i] + self.p4[i]) / four
        }))
    }
}

// ============================================================================
// 2D 四边形特有方法 / 2D quadrilateral specific methods
// ============================================================================

impl<S: Field + Float> Quadrilateral<2, S> {
    /// 计算 2D 四边形的面积
    /// Calculate the area of a 2D quadrilateral
    ///
    /// 使用鞋带公式（Shoelace formula）。
    /// Uses the Shoelace formula.
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Quadrilateral2};
    ///
    /// let p1 = Point2::new(0.0, 0.0);
    /// let p2 = Point2::new(4.0, 0.0);
    /// let p3 = Point2::new(4.0, 3.0);
    /// let p4 = Point2::new(0.0, 3.0);
    /// let quad = Quadrilateral2::new(p1, p2, p3, p4);
    ///
    /// let area_diff: f64 = quad.area() - 12.0;
    /// assert!(area_diff.abs() < 1e-10);
    /// ```
    pub fn area(&self) -> S {
        // 鞋带公式 / Shoelace formula
        let sum1 = self.p1.x() * self.p2.y()
            + self.p2.x() * self.p3.y()
            + self.p3.x() * self.p4.y()
            + self.p4.x() * self.p1.y();

        let sum2 = self.p1.y() * self.p2.x()
            + self.p2.y() * self.p3.x()
            + self.p3.y() * self.p4.x()
            + self.p4.y() * self.p1.x();

        ((sum1 - sum2).abs()) / (S::one() + S::one())
    }

    /// 通过分割成两个三角形计算面积
    /// Calculate area by splitting into two triangles
    pub fn area_by_triangles(&self) -> S {
        let t1 = Triangle::new(self.p1.clone(), self.p2.clone(), self.p3.clone());
        let t2 = Triangle::new(self.p1.clone(), self.p3.clone(), self.p4.clone());
        t1.area() + t2.area()
    }

    /// 判断是否为凸四边形
    /// Check if the quadrilateral is convex
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Quadrilateral2};
    ///
    /// // 凸四边形（矩形）/ Convex (rectangle)
    /// let convex = Quadrilateral2::new(
    ///     Point2::new(0.0, 0.0),
    ///     Point2::new(4.0, 0.0),
    ///     Point2::new(4.0, 3.0),
    ///     Point2::new(0.0, 3.0),
    /// );
    /// assert!(convex.is_convex());
    ///
    /// // 凹四边形 / Concave
    /// let concave = Quadrilateral2::new(
    ///     Point2::new(0.0, 0.0),
    ///     Point2::new(4.0, 0.0),
    ///     Point2::new(1.0, 1.0),  // 内凹点 / Concave point
    ///     Point2::new(0.0, 4.0),
    /// );
    /// assert!(!concave.is_convex());
    /// ```
    pub fn is_convex(&self) -> bool {
        // 检查所有内角是否同向旋转
        // Check if all interior angles rotate in the same direction
        let cross1 = self.cross_product_sign(&self.p1, &self.p2, &self.p3);
        let cross2 = self.cross_product_sign(&self.p2, &self.p3, &self.p4);
        let cross3 = self.cross_product_sign(&self.p3, &self.p4, &self.p1);
        let cross4 = self.cross_product_sign(&self.p4, &self.p1, &self.p2);

        // 所有叉积应该同号
        // All cross products should have the same sign
        let all_positive =
            cross1 > S::zero() && cross2 > S::zero() && cross3 > S::zero() && cross4 > S::zero();
        let all_negative =
            cross1 < S::zero() && cross2 < S::zero() && cross3 < S::zero() && cross4 < S::zero();

        all_positive || all_negative
    }

    /// 计算叉积符号（辅助函数）
    /// Calculate cross product sign (helper)
    fn cross_product_sign(&self, p1: &Point<2, S>, p2: &Point<2, S>, p3: &Point<2, S>) -> S {
        let v1 = super::Vector::from_points(p1, p2);
        let v2 = super::Vector::from_points(p2, p3);
        v1.cross_2d(&v2)
    }

    /// 判断是否为矩形
    /// Check if the quadrilateral is a rectangle
    pub fn is_rectangle(&self, epsilon: S) -> bool {
        // 检查所有角是否为直角
        // Check if all angles are right angles
        let edges = self.edges();

        // 检查相邻边是否垂直
        // Check if adjacent edges are perpendicular
        for i in 0..4 {
            let e1_dir = edges[i].direction();
            let e2_dir = edges[(i + 1) % 4].direction();
            let dot = e1_dir.dot(&e2_dir);
            if dot.abs() > epsilon {
                return false;
            }
        }
        true
    }

    /// 判断是否为正方形
    /// Check if the quadrilateral is a square
    pub fn is_square(&self, epsilon: S) -> bool {
        if !self.is_rectangle(epsilon) {
            return false;
        }

        // 检查所有边是否等长
        // Check if all edges have equal length
        let edges = self.edges();
        let len = edges[0].length();
        for edge in &edges[1..] {
            if (edge.length() - len).abs() > epsilon {
                return false;
            }
        }
        true
    }

    /// 判断是否为平行四边形
    /// Check if the quadrilateral is a parallelogram
    pub fn is_parallelogram(&self, epsilon: S) -> bool {
        // 平行四边形：对边平行且相等
        // Parallelogram: opposite sides are parallel and equal
        let e1_dir = self.e1().direction();
        let e3_dir = self.e3().direction();
        let e2_dir = self.e2().direction();
        let e4_dir = self.e4().direction();

        // 检查对边是否平行
        // Check if opposite sides are parallel
        let cross1 = e1_dir.cross_2d(&e3_dir);
        let cross2 = e2_dir.cross_2d(&e4_dir);

        cross1.abs() < epsilon && cross2.abs() < epsilon
    }

    /// 判断是否为梯形
    /// Check if the quadrilateral is a trapezoid
    pub fn is_trapezoid(&self, epsilon: S) -> bool {
        // 梯形：至少有一组对边平行
        // Trapezoid: at least one pair of opposite sides are parallel
        let e1_dir = self.e1().direction();
        let e3_dir = self.e3().direction();
        let e2_dir = self.e2().direction();
        let e4_dir = self.e4().direction();

        let cross1 = e1_dir.cross_2d(&e3_dir);
        let cross2 = e2_dir.cross_2d(&e4_dir);

        cross1.abs() < epsilon || cross2.abs() < epsilon
    }

    /// 判断点是否在四边形内部
    /// Check if a point is inside the quadrilateral
    pub fn contains_point(&self, point: &Point<2, S>) -> bool {
        // 使用分割三角形方法
        // Use split triangle method
        let t1 = Triangle::new(self.p1.clone(), self.p2.clone(), self.p3.clone());
        let t2 = Triangle::new(self.p1.clone(), self.p3.clone(), self.p4.clone());

        t1.contains_point(point) || t2.contains_point(point)
    }
}

// ============================================================================
// 类型别名 / Type aliases
// ============================================================================

/// 2D 四边形类型别名
/// 2D quadrilateral type alias
pub type Quadrilateral2<S = f64> = Quadrilateral<2, S>;

/// 3D 四边形类型别名
/// 3D quadrilateral type alias
pub type Quadrilateral3<S = f64> = Quadrilateral<3, S>;

// ============================================================================
// Trait 实现 / Trait implementations
// ============================================================================

impl<const D: usize, S: Field + Float> Debug for Quadrilateral<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Quadrilateral{}({:?}, {:?}, {:?}, {:?})",
            D, self.p1, self.p2, self.p3, self.p4
        )
    }
}

impl<const D: usize, S: Field + Float + Display> Display for Quadrilateral<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Quadrilateral({}, {}, {}, {})",
            self.p1, self.p2, self.p3, self.p4
        )
    }
}

impl<const D: usize, S: Field + Float + Epsilon> Quadrilateral<D, S> {
    /// 使用容差判断两个四边形是否近似相等
    /// Check if two quadrilaterals are approximately equal using tolerance
    pub fn approx_eq(&self, other: &Self) -> bool {
        self.p1.approx_eq(&other.p1)
            && self.p2.approx_eq(&other.p2)
            && self.p3.approx_eq(&other.p3)
            && self.p4.approx_eq(&other.p4)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point2;

    #[test]
    fn test_quadrilateral_creation() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(1.0, 0.0);
        let p3 = Point2::new(1.0, 1.0);
        let p4 = Point2::new(0.0, 1.0);
        let quad = Quadrilateral2::new(p1.clone(), p2.clone(), p3.clone(), p4.clone());

        assert_eq!(quad.p1(), &p1);
        assert_eq!(quad.p2(), &p2);
        assert_eq!(quad.p3(), &p3);
        assert_eq!(quad.p4(), &p4);
    }

    #[test]
    fn test_quadrilateral_area() {
        // 矩形 / Rectangle
        let rect = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 3.0),
            Point2::new(0.0, 3.0),
        );
        assert!((rect.area() - 12.0).abs() < 1e-10);

        // 正方形 / Square
        let square = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
        );
        assert!((square.area() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_quadrilateral_perimeter() {
        let rect = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 3.0),
            Point2::new(0.0, 3.0),
        );
        // 4 + 3 + 4 + 3 = 14
        assert!((rect.perimeter() - 14.0).abs() < 1e-10);
    }

    #[test]
    fn test_quadrilateral_is_convex() {
        // 凸四边形（矩形）/ Convex (rectangle)
        let convex = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 3.0),
            Point2::new(0.0, 3.0),
        );
        assert!(convex.is_convex());

        // 凹四边形 / Concave
        let concave = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 4.0),
        );
        assert!(!concave.is_convex());
    }

    #[test]
    fn test_quadrilateral_is_rectangle() {
        let rect = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 3.0),
            Point2::new(0.0, 3.0),
        );
        assert!(rect.is_rectangle(1e-10));

        let non_rect = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(3.0, 3.0),
            Point2::new(0.0, 3.0),
        );
        assert!(!non_rect.is_rectangle(1e-10));
    }

    #[test]
    fn test_quadrilateral_is_square() {
        let square = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
        );
        assert!(square.is_square(1e-10));

        let rect = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 2.0),
            Point2::new(0.0, 2.0),
        );
        assert!(!rect.is_square(1e-10));
    }

    #[test]
    fn test_quadrilateral_is_parallelogram() {
        let para = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(5.0, 3.0),
            Point2::new(1.0, 3.0),
        );
        assert!(para.is_parallelogram(1e-10));

        let non_para = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(5.0, 3.0),
            Point2::new(0.0, 3.0),
        );
        assert!(!non_para.is_parallelogram(1e-10));
    }

    #[test]
    fn test_quadrilateral_contains_point() {
        let rect = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 3.0),
            Point2::new(0.0, 3.0),
        );

        // 内部点 / Inside point
        assert!(rect.contains_point(&Point2::new(2.0, 1.5)));

        // 外部点 / Outside point
        assert!(!rect.contains_point(&Point2::new(5.0, 1.5)));

        // 边界点 / Boundary point
        assert!(rect.contains_point(&Point2::new(2.0, 0.0)));
    }

    #[test]
    fn test_quadrilateral_centroid() {
        let rect = Quadrilateral2::new(
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 4.0),
            Point2::new(0.0, 4.0),
        );

        let centroid = rect.centroid();
        assert!((centroid.x() - 2.0).abs() < 1e-10);
        assert!((centroid.y() - 2.0).abs() < 1e-10);
    }
}
