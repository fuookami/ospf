//! 圆实体模块
//! Circle entity module
//!
//! 本模块定义了圆类型：
//! This module defines circle types:
//!
//! - [`Circle`] - 泛型圆，由圆心和半径组成
//! - [`Circle`] - Generic circle composed of center and radius
//!
//! # 示例 / Examples
//!
//! ```
//! use ospf_rust_math::geometry::{Point2, Circle2};
//!
//! let center = Point2::new(1.0, 2.0);
//! let circle = Circle2::new(center, 3.0);
//!
//! // 面积 / Area
//! assert!((circle.area() - std::f64::consts::PI * 9.0).abs() < 1e-10);
//!
//! // 周长 / Circumference
//! assert!((circle.circumference() - 2.0 * std::f64::consts::PI * 3.0).abs() < 1e-10);
//! ```

use super::point::{Point, Point2};
use crate::algebra::{Epsilon, Field};
use num_traits::{Float, FloatConst, One, Zero};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

// ============================================================================
// Circle - 泛型圆
// ============================================================================

/// Circle - 泛型圆
/// Circle - Generic circle
///
/// 表示 N 维空间中的圆（2D）或球（3D+）。
/// Represents a circle (2D) or sphere (3D+) in N-dimensional space.
///
/// # 泛型参数 / Generic Parameters
/// - `const D: usize`: 圆心的维度 / Dimension of the center
/// - `S`: 标量类型 / Scalar type
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::geometry::{Point2, Circle2};
///
/// let center = Point2::new(0.0, 0.0);
/// let circle = Circle2::new(center, 5.0);
///
/// let radius_diff: f64 = circle.radius() - 5.0;
/// assert!(radius_diff.abs() < 1e-10);
/// ```
#[derive(Clone, PartialEq)]
pub struct Circle<const D: usize, S = f64> {
    /// 圆心 / Center
    center: Point<D, S>,
    /// 半径 / Radius
    radius: S,
}

// ============================================================================
// 构造方法 / Constructors
// ============================================================================

impl<const D: usize, S> Circle<D, S> {
    /// 创建新的圆
    /// Create a new circle
    ///
    /// # 参数 / Parameters
    /// - `center`: 圆心 / Center
    /// - `radius`: 半径 / Radius
    pub fn new(center: Point<D, S>, radius: S) -> Self {
        Self { center, radius }
    }

    /// 从圆心坐标和半径创建圆
    /// Create a circle from center coordinates and radius
    pub fn from_coords(center: [S; D], radius: S) -> Self {
        Self::new(Point::from_coords(center), radius)
    }

    /// 创建单位圆（圆心在原点，半径为 1）
    /// Create a unit circle (center at origin, radius 1)
    pub fn unit() -> Self
    where
        S: Zero + One,
    {
        Self::new(Point::origin(), S::one())
    }

    /// 获取圆心
    /// Get the center
    pub fn center(&self) -> &Point<D, S> {
        &self.center
    }

    /// 获取半径引用
    /// Get the radius reference
    pub fn radius_ref(&self) -> &S {
        &self.radius
    }

    /// 消费圆并返回圆心和半径
    /// Consume the circle and return center and radius
    pub fn into_parts(self) -> (Point<D, S>, S) {
        (self.center, self.radius)
    }

    /// 设置圆心
    /// Set the center
    pub fn set_center(&mut self, center: Point<D, S>) {
        self.center = center;
    }

    /// 设置半径
    /// Set the radius
    pub fn set_radius(&mut self, radius: S) {
        self.radius = radius;
    }
}

impl<const D: usize, S: Copy> Circle<D, S> {
    /// 获取半径
    /// Get the radius
    pub fn radius(&self) -> S {
        self.radius
    }
}

// ============================================================================
// 几何属性 / Geometric properties
// ============================================================================

impl<S: Field + Float + FloatConst> Circle<2, S> {
    /// 计算圆的面积
    /// Calculate the area of the circle
    ///
    /// 公式 / Formula: `A = π * r²`
    pub fn area(&self) -> S {
        S::PI() * self.radius * self.radius
    }

    /// 计算圆的周长
    /// Calculate the circumference of the circle
    ///
    /// 公式 / Formula: `C = 2 * π * r`
    pub fn circumference(&self) -> S {
        S::PI() * self.radius * (S::one() + S::one())
    }

    /// 计算直径
    /// Calculate the diameter
    pub fn diameter(&self) -> S {
        self.radius * (S::one() + S::one())
    }

    /// 判断点是否在圆内（包含边界）
    /// Check if a point is inside or on the circle
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Circle2};
    ///
    /// let center = Point2::new(0.0, 0.0);
    /// let circle = Circle2::new(center, 5.0);
    ///
    /// let inside = Point2::new(3.0, 4.0);   // 距圆心距离 = 5
    /// let outside = Point2::new(4.0, 4.0);  // 距圆心距离 ≈ 5.66
    ///
    /// assert!(circle.contains_point(&inside));
    /// assert!(!circle.contains_point(&outside));
    /// ```
    pub fn contains_point(&self, point: &Point<2, S>) -> bool {
        let dist_sq = self.center.distance_squared(point);
        dist_sq <= self.radius * self.radius
    }

    /// 判断点是否严格在圆内（不包含边界）
    /// Check if a point is strictly inside the circle (not on boundary)
    pub fn contains_point_strict(&self, point: &Point<2, S>) -> bool {
        let dist_sq = self.center.distance_squared(point);
        dist_sq < self.radius * self.radius
    }

    /// 判断点是否在圆上
    /// Check if a point is on the circle boundary
    pub fn point_on_boundary(&self, point: &Point<2, S>, epsilon: S) -> bool {
        let dist = self.center.distance(point);
        (dist - self.radius).abs() < epsilon
    }

    /// 判断两圆是否相交
    /// Check if two circles intersect
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::geometry::{Point2, Circle2};
    ///
    /// let c1 = Circle2::new(Point2::new(0.0, 0.0), 3.0);
    /// let c2 = Circle2::new(Point2::new(5.0, 0.0), 3.0);
    ///
    /// assert!(c1.intersects(&c2));
    /// ```
    pub fn intersects(&self, other: &Self) -> bool {
        let dist = self.center.distance(&other.center);
        let r_sum = self.radius + other.radius;
        dist <= r_sum
    }

    /// 判断两圆是否相切
    /// Check if two circles are tangent
    pub fn is_tangent(&self, other: &Self, epsilon: S) -> bool {
        let dist = self.center.distance(&other.center);
        let r_sum = self.radius + other.radius;
        let r_diff = (self.radius - other.radius).abs();

        // 外切或内切 / External or internal tangency
        (dist - r_sum).abs() < epsilon || (dist - r_diff).abs() < epsilon
    }

    /// 判断两圆是否包含关系
    /// Check if one circle contains another
    pub fn contains_circle(&self, other: &Self) -> bool {
        let dist = self.center.distance(&other.center);
        dist + other.radius <= self.radius
    }

    /// 计算两圆交点
    /// Calculate intersection points of two circles
    ///
    /// 返回 0、1 或 2 个交点。
    /// Returns 0, 1, or 2 intersection points.
    pub fn intersection_points(&self, other: &Self) -> Vec<Point<2, S>> {
        let dx = other.center.x() - self.center.x();
        let dy = other.center.y() - self.center.y();
        let d = self.center.distance(&other.center);

        // 无交点 / No intersection
        if d > self.radius + other.radius || d < (self.radius - other.radius).abs() {
            return vec![];
        }

        // 相切，一个交点 / Tangent, one intersection
        if d.is_zero() && (self.radius - other.radius).abs() < S::epsilon() {
            // 同一个圆 / Same circle
            return vec![];
        }

        let two = S::one() + S::one();
        let a = (self.radius * self.radius - other.radius * other.radius + d * d) / (two * d);
        let h = (self.radius * self.radius - a * a).sqrt();

        if h.is_zero() {
            // 相切 / Tangent
            let px = self.center.x() + a * dx / d;
            let py = self.center.y() + a * dy / d;
            return vec![Point2::new(px, py)];
        }

        // 两个交点 / Two intersections
        let px = self.center.x() + a * dx / d;
        let py = self.center.y() + a * dy / d;

        let p1 = Point2::new(px + h * dy / d, py - h * dx / d);
        let p2 = Point2::new(px - h * dy / d, py + h * dx / d);

        vec![p1, p2]
    }
}

// ============================================================================
// 3D 球体特有方法 / 3D sphere specific methods
// ============================================================================

impl<S: Field + Float + FloatConst> Circle<3, S> {
    /// 计算球的体积
    /// Calculate the volume of the sphere
    ///
    /// 公式 / Formula: `V = (4/3) * π * r³`
    pub fn volume(&self) -> S {
        let four = S::one() + S::one() + S::one() + S::one();
        let three = S::one() + S::one() + S::one();
        four / three * S::PI() * self.radius * self.radius * self.radius
    }

    /// 计算球的表面积
    /// Calculate the surface area of the sphere
    ///
    /// 公式 / Formula: `A = 4 * π * r²`
    pub fn surface_area(&self) -> S {
        let four = S::one() + S::one() + S::one() + S::one();
        four * S::PI() * self.radius * self.radius
    }

    /// 判断点是否在球内（包含边界）
    /// Check if a point is inside or on the sphere
    pub fn contains_point(&self, point: &Point<3, S>) -> bool {
        let dist_sq = self.center.distance_squared(point);
        dist_sq <= self.radius * self.radius
    }
}

// ============================================================================
// 类型别名 / Type aliases
// ============================================================================

/// 2D 圆类型别名
/// 2D circle type alias
pub type Circle2<S = f64> = Circle<2, S>;

/// 3D 球类型别名
/// 3D sphere type alias
pub type Sphere3<S = f64> = Circle<3, S>;

// ============================================================================
// Trait 实现 / Trait implementations
// ============================================================================

impl<const D: usize, S: Debug> Debug for Circle<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Circle{}(center: {:?}, radius: {:?})",
            D, self.center, self.radius
        )
    }
}

impl<const D: usize, S: Display> Display for Circle<D, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Circle(center: {}, radius: {})",
            self.center, self.radius
        )
    }
}

impl<const D: usize, S: Field + Float + Epsilon> Circle<D, S> {
    /// 使用容差判断两圆是否近似相等
    /// Check if two circles are approximately equal using tolerance
    pub fn approx_eq(&self, other: &Self) -> bool {
        self.center.approx_eq(&other.center)
            && (self.radius - other.radius).abs() < Epsilon::epsilon()
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::point::Point3;

    #[test]
    fn test_circle_creation() {
        let center = Point2::new(1.0, 2.0);
        let circle = Circle2::new(center.clone(), 5.0);

        assert_eq!(circle.center(), &center);
        assert!((circle.radius() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_circle_area() {
        let circle = Circle2::new(Point2::new(0.0, 0.0), 3.0);

        // 面积 = π * r² = π * 9
        let expected_area = std::f64::consts::PI * 9.0;
        assert!((circle.area() - expected_area).abs() < 1e-10);
    }

    #[test]
    fn test_circle_circumference() {
        let circle = Circle2::new(Point2::new(0.0, 0.0), 3.0);

        // 周长 = 2 * π * r = 2 * π * 3 = 6π
        let expected_circumference = 2.0 * std::f64::consts::PI * 3.0;
        assert!((circle.circumference() - expected_circumference).abs() < 1e-10);
    }

    #[test]
    fn test_circle_contains_point() {
        let center = Point2::new(0.0, 0.0);
        let circle = Circle2::new(center, 5.0);

        // 圆心在圆内 / Center is inside
        assert!(circle.contains_point(&Point2::new(0.0, 0.0)));

        // 圆上的点 / Point on boundary
        assert!(circle.contains_point(&Point2::new(5.0, 0.0)));
        assert!(circle.contains_point(&Point2::new(3.0, 4.0)));

        // 圆外的点 / Point outside
        assert!(!circle.contains_point(&Point2::new(6.0, 0.0)));
        assert!(!circle.contains_point(&Point2::new(4.0, 4.0)));
    }

    #[test]
    fn test_circle_intersects() {
        let c1 = Circle2::new(Point2::new(0.0, 0.0), 3.0);

        // 相交 / Intersect
        let c2 = Circle2::new(Point2::new(5.0, 0.0), 3.0);
        assert!(c1.intersects(&c2));

        // 相离 / Separate
        let c3 = Circle2::new(Point2::new(10.0, 0.0), 3.0);
        assert!(!c1.intersects(&c3));

        // 包含 / Contained
        let c4 = Circle2::new(Point2::new(1.0, 0.0), 1.0);
        assert!(c1.intersects(&c4));
    }

    #[test]
    fn test_circle_intersection_points() {
        // 两个交点 / Two intersections
        let c1 = Circle2::new(Point2::new(0.0, 0.0), 5.0);
        let c2 = Circle2::new(Point2::new(4.0, 0.0), 3.0);

        let intersections = c1.intersection_points(&c2);
        assert_eq!(intersections.len(), 2);

        // 验证交点在两圆上 / Verify intersections are on both circles
        for p in &intersections {
            assert!(c1.point_on_boundary(p, 1e-10));
            assert!(c2.point_on_boundary(p, 1e-10));
        }

        // 无交点 / No intersection
        let c3 = Circle2::new(Point2::new(20.0, 0.0), 3.0);
        assert!(c1.intersection_points(&c3).is_empty());
    }

    #[test]
    fn test_sphere_volume() {
        let sphere = Sphere3::new(Point3::new(0.0, 0.0, 0.0), 3.0);

        // 体积 = (4/3) * π * r³ = (4/3) * π * 27 = 36π
        let expected_volume = (4.0 / 3.0) * std::f64::consts::PI * 27.0;
        assert!((sphere.volume() - expected_volume).abs() < 1e-10);
    }

    #[test]
    fn test_sphere_surface_area() {
        let sphere = Sphere3::new(Point3::new(0.0, 0.0, 0.0), 3.0);

        // 表面积 = 4 * π * r² = 4 * π * 9 = 36π
        let expected_area = 4.0 * std::f64::consts::PI * 9.0;
        assert!((sphere.surface_area() - expected_area).abs() < 1e-10);
    }

    #[test]
    fn test_unit_circle() {
        let unit: Circle2 = Circle2::unit();

        assert!(unit.center().is_zero());
        assert!((unit.radius() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_circle_contains_circle() {
        let outer = Circle2::new(Point2::new(0.0, 0.0), 10.0);
        let inner = Circle2::new(Point2::new(2.0, 2.0), 3.0);
        let partial = Circle2::new(Point2::new(8.0, 0.0), 5.0);

        assert!(outer.contains_circle(&inner));
        assert!(!outer.contains_circle(&partial));
    }
}
