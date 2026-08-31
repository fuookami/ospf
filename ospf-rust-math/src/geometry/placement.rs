//! 几何放置。
//! Geometric placements.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::{Box2, Box2Shape, Box3, Point2, Point3, Shape3Kind};

/// 二维放置。
/// Two-dimensional placement.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement2<S: Field + Float = f64> {
    /// X 坐标。
    /// X coordinate.
    pub x: S,
    /// Y 坐标。
    /// Y coordinate.
    pub y: S,
    /// 放置的二维形状。
    /// Placed two-dimensional shape.
    pub shape: Box2Shape<S>,
}

impl<S> Placement2<S>
where
    S: Field + Float,
{
    /// 创建二维放置。
    /// Create a two-dimensional placement.
    pub fn new(x: S, y: S, shape: impl Into<Box2Shape<S>>) -> Self {
        Self {
            x,
            y,
            shape: shape.into(),
        }
    }

    /// 返回对应包围盒。
    /// Return the corresponding bounding box.
    pub fn box2(&self) -> Box2<S> {
        Box2::new(self.x, self.y, self.shape.clone())
    }

    /// 宽度。
    /// Width.
    pub fn width(&self) -> S {
        self.box2().width()
    }

    /// 高度。
    /// Height.
    pub fn height(&self) -> S {
        self.box2().height()
    }

    /// X 轴最大值。
    /// Maximum X value.
    pub fn max_x(&self) -> S {
        self.box2().max_x()
    }

    /// Y 轴最大值。
    /// Maximum Y value.
    pub fn max_y(&self) -> S {
        self.box2().max_y()
    }

    /// 判断点是否位于放置区域内。
    /// Check whether a point lies inside the placement region.
    pub fn contains_point(&self, point: &Point2<S>) -> bool {
        self.box2().contains_point(point)
    }

    /// 判断坐标是否位于放置区域内。
    /// Check whether coordinates lie inside the placement region.
    pub fn contains(
        &self,
        x: S,
        y: S,
        with_lower_bound: bool,
        with_upper_bound: bool,
        with_border: bool,
    ) -> bool {
        self.box2()
            .contains(x, y, with_lower_bound, with_upper_bound, with_border)
    }

    /// 判断两个放置是否重叠。
    /// Check whether two placements overlap.
    pub fn overlapped(&self, rhs: &Self) -> bool {
        self.box2().overlapped(&rhs.box2())
    }

    /// 计算两个放置的轴对齐交集。
    /// Compute the axis-aligned intersection of two placements.
    pub fn intersect(&self, rhs: &Self) -> Option<Self> {
        let intersection = self.box2().intersect(&rhs.box2())?;
        Some(Self::new(
            intersection.x,
            intersection.y,
            intersection.shape,
        ))
    }
}

/// 三维放置。
/// Three-dimensional placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement3<S: Field + Float = f64> {
    /// X 坐标。
    /// X coordinate.
    pub x: S,
    /// Y 坐标。
    /// Y coordinate.
    pub y: S,
    /// Z 坐标。
    /// Z coordinate.
    pub z: S,
    /// 放置的三维形状。
    /// Placed three-dimensional shape.
    pub shape: Shape3Kind<S>,
}

impl<S> Placement3<S>
where
    S: Field + Float,
{
    /// 创建三维放置。
    /// Create a three-dimensional placement.
    pub fn new(x: S, y: S, z: S, shape: impl Into<Shape3Kind<S>>) -> Self {
        Self {
            x,
            y,
            z,
            shape: shape.into(),
        }
    }

    /// 返回对应包围盒。
    /// Return the corresponding bounding box.
    pub fn box3(&self) -> Box3<S> {
        Box3::new(self.x, self.y, self.z, self.shape.bounding_cuboid())
    }

    /// 宽度。
    /// Width.
    pub fn width(&self) -> S {
        self.box3().width()
    }

    /// 高度。
    /// Height.
    pub fn height(&self) -> S {
        self.box3().height()
    }

    /// 深度。
    /// Depth.
    pub fn depth(&self) -> S {
        self.box3().depth()
    }

    /// X 轴最大值。
    /// Maximum X value.
    pub fn max_x(&self) -> S {
        self.box3().max_x()
    }

    /// Y 轴最大值。
    /// Maximum Y value.
    pub fn max_y(&self) -> S {
        self.box3().max_y()
    }

    /// Z 轴最大值。
    /// Maximum Z value.
    pub fn max_z(&self) -> S {
        self.box3().max_z()
    }

    /// 判断点是否位于放置区域内。
    /// Check whether a point lies inside the placement region.
    pub fn contains_point(&self, point: &Point3<S>) -> bool {
        self.box3().contains_point(point)
    }

    /// 判断坐标是否位于放置区域内。
    /// Check whether coordinates lie inside the placement region.
    pub fn contains(
        &self,
        x: S,
        y: S,
        z: S,
        with_lower_bound: bool,
        with_upper_bound: bool,
        with_border: bool,
    ) -> bool {
        self.box3()
            .contains(x, y, z, with_lower_bound, with_upper_bound, with_border)
    }

    /// 判断两个放置是否重叠。
    /// Check whether two placements overlap.
    pub fn overlapped(&self, rhs: &Self) -> bool {
        self.box3().overlapped(&rhs.box3())
    }

    /// 计算两个放置的轴对齐交集。
    /// Compute the axis-aligned intersection of two placements.
    pub fn intersect(&self, rhs: &Self) -> Option<Self> {
        let intersection = self.box3().intersect(&rhs.box3())?;
        Some(Self::new(
            intersection.x,
            intersection.y,
            intersection.z,
            intersection.cuboid,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Axis3, Circle2, Cuboid3, Cylinder3, Point2, Point3, Rectangle2};

    #[test]
    fn placement2_delegates_to_box2() {
        let lhs = Placement2::new(0.0, 0.0, Rectangle2::new(4.0, 4.0));
        let rhs = Placement2::new(2.0, 2.0, Circle2::new(Point2::origin(), 1.0));

        assert_eq!(lhs.width(), 4.0);
        assert!(lhs.contains_point(&Point2::new(1.0, 1.0)));
        assert!(lhs.overlapped(&rhs));
        assert_eq!(lhs.intersect(&rhs).unwrap().width(), 2.0);
    }

    #[test]
    fn placement3_uses_shape_bounding_cuboid() {
        let cuboid = Placement3::new(0.0, 0.0, 0.0, Cuboid3::new(4.0, 4.0, 4.0));
        let cylinder = Placement3::new(2.0, 2.0, 1.0, Cylinder3::new(1.0, 3.0, Axis3::Z));

        assert_eq!(cylinder.width(), 2.0);
        assert_eq!(cylinder.depth(), 3.0);
        assert!(cuboid.contains_point(&Point3::new(1.0, 1.0, 1.0)));
        assert!(cuboid.overlapped(&cylinder));
        assert_eq!(
            cuboid.intersect(&cylinder).unwrap().shape,
            Shape3Kind::Cuboid(Cuboid3::new(2.0, 2.0, 3.0))
        );
    }
}
