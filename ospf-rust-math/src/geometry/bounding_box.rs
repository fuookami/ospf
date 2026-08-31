//! 轴对齐包围盒。
//! Axis-aligned bounding boxes.

use crate::algebra::Field;
use crate::geometry::{Box2Shape, Cuboid3, Point2, Point3, Rectangle2};
use num_traits::{Float, Zero};
use std::ops::Add;

/// 二维包围盒。
/// Two-dimensional bounding box.
#[derive(Debug, Clone, PartialEq)]
pub struct Box2<S = f64> {
    /// X 坐标。
    /// X coordinate.
    pub x: S,
    /// Y 坐标。
    /// Y coordinate.
    pub y: S,
    /// 包围的二维形状。
    /// Enclosed two-dimensional shape.
    pub shape: Box2Shape<S>,
}

impl<S> Box2<S> {
    /// 创建二维包围盒。
    /// Create a two-dimensional bounding box.
    pub fn new(x: S, y: S, shape: impl Into<Box2Shape<S>>) -> Self {
        Self {
            x,
            y,
            shape: shape.into(),
        }
    }
}

impl<S> Box2<S>
where
    S: Zero,
{
    /// 在原点创建二维包围盒。
    /// Create a two-dimensional bounding box at the origin.
    pub fn at_origin(shape: impl Into<Box2Shape<S>>) -> Self {
        Self::new(S::zero(), S::zero(), shape)
    }
}

impl<S> Box2<S>
where
    S: Clone,
{
    /// 矩形宽度，非矩形返回 `None`。
    /// Rectangle width, or `None` for non-rectangular shapes.
    pub fn rectangle_width(&self) -> Option<S> {
        self.shape.rectangle_width()
    }

    /// 矩形高度，非矩形返回 `None`。
    /// Rectangle height, or `None` for non-rectangular shapes.
    pub fn rectangle_height(&self) -> Option<S> {
        self.shape.rectangle_height()
    }
}

impl<S> Box2<S>
where
    S: Clone + Add<Output = S>,
{
    /// 矩形 X 轴最大值，非矩形返回 `None`。
    /// Rectangle maximum X value, or `None` for non-rectangular shapes.
    pub fn rectangle_max_x(&self) -> Option<S> {
        Some(self.x.clone() + self.rectangle_width()?)
    }

    /// 矩形 Y 轴最大值，非矩形返回 `None`。
    /// Rectangle maximum Y value, or `None` for non-rectangular shapes.
    pub fn rectangle_max_y(&self) -> Option<S> {
        Some(self.y.clone() + self.rectangle_height()?)
    }
}

impl<S> Box2<S>
where
    S: Clone + Add<Output = S> + PartialOrd,
{
    /// 判断点是否位于矩形包围盒内，非矩形返回 `false`。
    /// Check whether a point lies inside a rectangle box, or `false` for non-rectangular boxes.
    pub fn contains_rectangle_point(&self, point: &Point2<S>) -> bool {
        self.contains_rectangle(
            point.x_ref().clone(),
            point.y_ref().clone(),
            true,
            true,
            true,
        )
    }

    /// 判断坐标是否位于矩形包围盒内，非矩形返回 `false`。
    /// Check whether coordinates lie inside a rectangle box, or `false` for non-rectangular boxes.
    pub fn contains_rectangle(
        &self,
        x: S,
        y: S,
        with_lower_bound: bool,
        with_upper_bound: bool,
        with_border: bool,
    ) -> bool {
        let Some(max_x) = self.rectangle_max_x() else {
            return false;
        };
        let Some(max_y) = self.rectangle_max_y() else {
            return false;
        };
        let include_lower = with_border && with_lower_bound;
        let include_upper = with_border && with_upper_bound;
        contains_ordered_range(x, self.x.clone(), max_x, include_lower, include_upper)
            && contains_ordered_range(y, self.y.clone(), max_y, include_lower, include_upper)
    }
}

impl<S> Box2<S>
where
    S: Field + Float,
{
    /// 宽度。
    /// Width.
    pub fn width(&self) -> S {
        self.shape.width()
    }

    /// 高度。
    /// Height.
    pub fn height(&self) -> S {
        self.shape.height()
    }

    /// X 轴最大值。
    /// Maximum X value.
    pub fn max_x(&self) -> S {
        self.x + self.width()
    }

    /// Y 轴最大值。
    /// Maximum Y value.
    pub fn max_y(&self) -> S {
        self.y + self.height()
    }

    /// 判断点是否位于包围盒内。
    /// Check whether a point lies inside the bounding box.
    pub fn contains_point(&self, point: &Point2<S>) -> bool {
        self.contains(point.x(), point.y(), true, true, true)
    }

    /// 判断坐标是否位于包围盒内。
    /// Check whether coordinates lie inside the bounding box.
    pub fn contains(
        &self,
        x: S,
        y: S,
        with_lower_bound: bool,
        with_upper_bound: bool,
        with_border: bool,
    ) -> bool {
        match &self.shape {
            Box2Shape::Rectangle(_) => {
                let include_lower = with_border && with_lower_bound;
                let include_upper = with_border && with_upper_bound;
                contains_range(x, self.x, self.max_x(), include_lower, include_upper)
                    && contains_range(y, self.y, self.max_y(), include_lower, include_upper)
            }
            Box2Shape::Circle(circle) => {
                let radius = circle.radius();
                let center_x = self.x + radius;
                let center_y = self.y + radius;
                let dx = x - center_x;
                let dy = y - center_y;
                let distance2 = dx * dx + dy * dy;
                let radius2 = radius * radius;
                if with_border {
                    distance2 <= radius2
                } else {
                    distance2 < radius2
                }
            }
        }
    }

    /// 判断两个包围盒是否重叠。
    /// Check whether two bounding boxes overlap.
    pub fn overlapped(&self, rhs: &Self) -> bool {
        match (&self.shape, &rhs.shape) {
            (Box2Shape::Rectangle(_), Box2Shape::Rectangle(_)) => self.rectangle_overlapped(rhs),
            (Box2Shape::Rectangle(_), Box2Shape::Circle(_)) => self.rect_circle_overlapped(rhs),
            (Box2Shape::Circle(_), Box2Shape::Rectangle(_)) => rhs.rect_circle_overlapped(self),
            (Box2Shape::Circle(_), Box2Shape::Circle(_)) => self.circle_overlapped(rhs),
        }
    }

    /// 计算两个包围盒的轴对齐交集。
    /// Compute the axis-aligned intersection of two bounding boxes.
    pub fn intersect(&self, rhs: &Self) -> Option<Self> {
        let min_x = max_value(self.x, rhs.x);
        let max_x = min_value(self.max_x(), rhs.max_x());
        let min_y = max_value(self.y, rhs.y);
        let max_y = min_value(self.max_y(), rhs.max_y());
        if min_x >= max_x || min_y >= max_y {
            return None;
        }
        Some(Self::new(
            min_x,
            min_y,
            Rectangle2::new(max_x - min_x, max_y - min_y),
        ))
    }

    fn rectangle_overlapped(&self, rhs: &Self) -> bool {
        self.max_x() > rhs.x && self.x < rhs.max_x() && self.max_y() > rhs.y && self.y < rhs.max_y()
    }

    fn rect_circle_overlapped(&self, circle_box: &Self) -> bool {
        let radius = circle_box
            .shape
            .radius()
            .expect("rect_circle_overlapped requires a circle box");
        let center_x = circle_box.x + radius;
        let center_y = circle_box.y + radius;
        let closest_x = clamp_value(center_x, self.x, self.max_x());
        let closest_y = clamp_value(center_y, self.y, self.max_y());
        let dx = center_x - closest_x;
        let dy = center_y - closest_y;
        dx * dx + dy * dy <= radius * radius
    }

    fn circle_overlapped(&self, rhs: &Self) -> bool {
        let lhs_radius = self
            .shape
            .radius()
            .expect("circle_overlapped requires a circle box");
        let rhs_radius = rhs
            .shape
            .radius()
            .expect("circle_overlapped requires a circle box");
        let lhs_center_x = self.x + lhs_radius;
        let lhs_center_y = self.y + lhs_radius;
        let rhs_center_x = rhs.x + rhs_radius;
        let rhs_center_y = rhs.y + rhs_radius;
        let dx = lhs_center_x - rhs_center_x;
        let dy = lhs_center_y - rhs_center_y;
        let reach = lhs_radius + rhs_radius;
        dx * dx + dy * dy <= reach * reach
    }
}

/// 二维轴对齐包围盒兼容命名。
/// Compatibility name for two-dimensional axis-aligned bounding boxes.
pub type AxisAlignedBox2<S = f64> = Box2<S>;

/// 三维包围盒。
/// Three-dimensional bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box3<S: Field + Float = f64> {
    /// X 坐标。
    /// X coordinate.
    pub x: S,
    /// Y 坐标。
    /// Y coordinate.
    pub y: S,
    /// Z 坐标。
    /// Z coordinate.
    pub z: S,
    /// 包围的长方体。
    /// Enclosed cuboid.
    pub cuboid: Cuboid3<S>,
}

impl<S> Box3<S>
where
    S: Field + Float,
{
    /// 创建三维包围盒。
    /// Create a three-dimensional bounding box.
    pub fn new(x: S, y: S, z: S, cuboid: Cuboid3<S>) -> Self {
        Self { x, y, z, cuboid }
    }

    /// 在原点创建三维包围盒。
    /// Create a three-dimensional bounding box at the origin.
    pub fn at_origin(cuboid: Cuboid3<S>) -> Self {
        Self::new(S::zero(), S::zero(), S::zero(), cuboid)
    }

    /// 宽度。
    /// Width.
    pub fn width(&self) -> S {
        self.cuboid.width
    }

    /// 高度。
    /// Height.
    pub fn height(&self) -> S {
        self.cuboid.height
    }

    /// 深度。
    /// Depth.
    pub fn depth(&self) -> S {
        self.cuboid.depth
    }

    /// X 轴最大值。
    /// Maximum X value.
    pub fn max_x(&self) -> S {
        self.x + self.width()
    }

    /// Y 轴最大值。
    /// Maximum Y value.
    pub fn max_y(&self) -> S {
        self.y + self.height()
    }

    /// Z 轴最大值。
    /// Maximum Z value.
    pub fn max_z(&self) -> S {
        self.z + self.depth()
    }

    /// 判断点是否位于包围盒内。
    /// Check whether a point lies inside the bounding box.
    pub fn contains_point(&self, point: &Point3<S>) -> bool {
        self.contains(point.x(), point.y(), point.z(), true, true, true)
    }

    /// 判断坐标是否位于包围盒内。
    /// Check whether coordinates lie inside the bounding box.
    pub fn contains(
        &self,
        x: S,
        y: S,
        z: S,
        with_lower_bound: bool,
        with_upper_bound: bool,
        with_border: bool,
    ) -> bool {
        let include_lower = with_border && with_lower_bound;
        let include_upper = with_border && with_upper_bound;
        contains_range(x, self.x, self.max_x(), include_lower, include_upper)
            && contains_range(y, self.y, self.max_y(), include_lower, include_upper)
            && contains_range(z, self.z, self.max_z(), include_lower, include_upper)
    }

    /// 判断两个包围盒是否重叠。
    /// Check whether two bounding boxes overlap.
    pub fn overlapped(&self, rhs: &Self) -> bool {
        self.max_x() > rhs.x
            && self.x < rhs.max_x()
            && self.max_y() > rhs.y
            && self.y < rhs.max_y()
            && self.max_z() > rhs.z
            && self.z < rhs.max_z()
    }

    /// 计算两个包围盒的交集。
    /// Compute the intersection of two bounding boxes.
    pub fn intersect(&self, rhs: &Self) -> Option<Self> {
        let min_x = max_value(self.x, rhs.x);
        let max_x = min_value(self.max_x(), rhs.max_x());
        let min_y = max_value(self.y, rhs.y);
        let max_y = min_value(self.max_y(), rhs.max_y());
        let min_z = max_value(self.z, rhs.z);
        let max_z = min_value(self.max_z(), rhs.max_z());
        if min_x >= max_x || min_y >= max_y || min_z >= max_z {
            return None;
        }
        Some(Self::new(
            min_x,
            min_y,
            min_z,
            Cuboid3::new(max_x - min_x, max_y - min_y, max_z - min_z),
        ))
    }
}

/// 三维轴对齐包围盒兼容命名。
/// Compatibility name for three-dimensional axis-aligned bounding boxes.
pub type AxisAlignedBox3<S = f64> = Box3<S>;

fn contains_range<S>(value: S, lower: S, upper: S, include_lower: bool, include_upper: bool) -> bool
where
    S: Field + Float,
{
    let lower_ok = if include_lower {
        value >= lower
    } else {
        value > lower
    };
    let upper_ok = if include_upper {
        value <= upper
    } else {
        value < upper
    };
    lower_ok && upper_ok
}

fn contains_ordered_range<S>(
    value: S,
    lower: S,
    upper: S,
    include_lower: bool,
    include_upper: bool,
) -> bool
where
    S: PartialOrd,
{
    let lower_ok = if include_lower {
        value >= lower
    } else {
        value > lower
    };
    let upper_ok = if include_upper {
        value <= upper
    } else {
        value < upper
    };
    lower_ok && upper_ok
}

fn min_value<S>(lhs: S, rhs: S) -> S
where
    S: Field + Float,
{
    if lhs <= rhs { lhs } else { rhs }
}

fn max_value<S>(lhs: S, rhs: S) -> S
where
    S: Field + Float,
{
    if lhs >= rhs { lhs } else { rhs }
}

fn clamp_value<S>(value: S, lower: S, upper: S) -> S
where
    S: Field + Float,
{
    max_value(lower, min_value(value, upper))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Circle2, Point2};

    #[test]
    fn box2_reports_rectangle_bounds_and_contains_points() {
        let bbox = Box2::new(1.0, 2.0, Rectangle2::new(3.0, 4.0));

        assert_eq!(bbox.width(), 3.0);
        assert_eq!(bbox.height(), 4.0);
        assert_eq!(bbox.max_x(), 4.0);
        assert_eq!(bbox.max_y(), 6.0);
        assert!(bbox.contains_point(&Point2::new(2.0, 3.0)));
        assert!(!bbox.contains(4.0, 3.0, true, true, false));
    }

    #[test]
    fn box2_handles_circle_contains_and_overlap() {
        let circle = Box2::new(0.0, 0.0, Circle2::new(Point2::origin(), 2.0));
        let rectangle = Box2::new(3.0, 1.0, Rectangle2::new(2.0, 2.0));

        assert!(circle.contains(2.0, 2.0, true, true, true));
        assert!(circle.contains(4.0, 2.0, true, true, true));
        assert!(!circle.contains(4.0, 2.0, true, true, false));
        assert!(rectangle.overlapped(&circle));
    }

    #[test]
    fn box2_intersection_returns_rectangle_box() {
        let lhs = Box2::new(0.0, 0.0, Rectangle2::new(4.0, 4.0));
        let rhs = Box2::new(2.0, 1.0, Rectangle2::new(4.0, 2.0));

        let intersection = lhs.intersect(&rhs).unwrap();

        assert_eq!(intersection.x, 2.0);
        assert_eq!(intersection.y, 1.0);
        assert_eq!(intersection.width(), 2.0);
        assert_eq!(intersection.height(), 2.0);
    }

    #[test]
    fn box3_contains_overlaps_and_intersects() {
        let lhs = Box3::new(0.0, 0.0, 0.0, Cuboid3::new(4.0, 5.0, 6.0));
        let rhs = Box3::new(2.0, 3.0, 4.0, Cuboid3::new(4.0, 4.0, 4.0));

        assert!(lhs.contains_point(&Point3::new(1.0, 1.0, 1.0)));
        assert!(lhs.overlapped(&rhs));
        assert_eq!(
            lhs.intersect(&rhs),
            Some(Box3::new(2.0, 3.0, 4.0, Cuboid3::new(2.0, 2.0, 2.0)))
        );
    }
}
