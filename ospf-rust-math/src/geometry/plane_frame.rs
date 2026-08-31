//! 三维平面坐标框架。
//! Three-dimensional plane coordinate frames.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::{Axis3, AxisPlane3, Cuboid3, Point2, Point3, Rectangle2, Vector3};

/// 平面二维点兼容命名。
/// Compatibility name for two-dimensional points in a plane frame.
pub type PlanePoint2<S = f64> = Point2<S>;

/// 平面三维点兼容命名。
/// Compatibility name for three-dimensional points in a plane frame.
pub type PlanePoint3<S = f64> = Point3<S>;

/// 平面三维向量兼容命名。
/// Compatibility name for three-dimensional vectors in a plane frame.
pub type PlaneVector3<S = f64> = Vector3<S>;

/// 三维轴向取值。
/// Axis-based value access in three dimensions.
pub trait AlongAxis3<S>
where
    S: Field + Float,
{
    /// 沿指定轴的值。
    /// Value along the specified axis.
    fn along(&self, axis: Axis3) -> S;
}

impl<S> AlongAxis3<S> for Point3<S>
where
    S: Field + Float,
{
    fn along(&self, axis: Axis3) -> S {
        self[axis.index()]
    }
}

impl<S> AlongAxis3<S> for Vector3<S>
where
    S: Field + Float,
{
    fn along(&self, axis: Axis3) -> S {
        self[axis.index()]
    }
}

/// 三维平面坐标框架。
/// Three-dimensional plane coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlaneFrame3 {
    /// 平面第一轴。
    /// The first axis of the plane.
    pub first_axis: Axis3,
    /// 平面第二轴。
    /// The second axis of the plane.
    pub second_axis: Axis3,
}

impl PlaneFrame3 {
    /// X-Y 平面。
    /// X-Y plane.
    pub const XY: Self = Self {
        first_axis: Axis3::X,
        second_axis: Axis3::Y,
    };

    /// Y-X 平面。
    /// Y-X plane.
    pub const YX: Self = Self {
        first_axis: Axis3::Y,
        second_axis: Axis3::X,
    };

    /// X-Z 平面。
    /// X-Z plane.
    pub const XZ: Self = Self {
        first_axis: Axis3::X,
        second_axis: Axis3::Z,
    };

    /// Z-X 平面。
    /// Z-X plane.
    pub const ZX: Self = Self {
        first_axis: Axis3::Z,
        second_axis: Axis3::X,
    };

    /// Y-Z 平面。
    /// Y-Z plane.
    pub const YZ: Self = Self {
        first_axis: Axis3::Y,
        second_axis: Axis3::Z,
    };

    /// Z-Y 平面。
    /// Z-Y plane.
    pub const ZY: Self = Self {
        first_axis: Axis3::Z,
        second_axis: Axis3::Y,
    };

    /// 创建平面坐标框架，要求两个轴不同。
    /// Create a plane coordinate frame with two distinct axes.
    pub const fn new(first_axis: Axis3, second_axis: Axis3) -> Option<Self> {
        if first_axis.index() == second_axis.index() {
            None
        } else {
            Some(Self {
                first_axis,
                second_axis,
            })
        }
    }

    /// 平面法向轴。
    /// The normal axis of the plane.
    pub const fn normal_axis(self) -> Axis3 {
        if (self.first_axis.index() == Axis3::X.index()
            && self.second_axis.index() == Axis3::Y.index())
            || (self.first_axis.index() == Axis3::Y.index()
                && self.second_axis.index() == Axis3::X.index())
        {
            Axis3::Z
        } else if (self.first_axis.index() == Axis3::X.index()
            && self.second_axis.index() == Axis3::Z.index())
            || (self.first_axis.index() == Axis3::Z.index()
                && self.second_axis.index() == Axis3::X.index())
        {
            Axis3::Y
        } else {
            Axis3::X
        }
    }

    /// 对应的无向主平面。
    /// The corresponding unordered principal plane.
    pub const fn axis_plane(self) -> AxisPlane3 {
        match self.normal_axis() {
            Axis3::X => AxisPlane3::YZ,
            Axis3::Y => AxisPlane3::XZ,
            Axis3::Z => AxisPlane3::XY,
        }
    }

    /// 计算点到平面的距离。
    /// Compute the distance from a point to the plane.
    pub fn distance<S>(self, point: &PlanePoint3<S>) -> S
    where
        S: Field + Float,
    {
        point.along(self.normal_axis())
    }

    /// 将三维点投影到平面二维坐标。
    /// Project a three-dimensional point to two-dimensional plane coordinates.
    pub fn point2<S>(self, point: &PlanePoint3<S>) -> PlanePoint2<S>
    where
        S: Field + Float,
    {
        Point2::new(point.along(self.first_axis), point.along(self.second_axis))
    }

    /// 将平面二维坐标还原为三维点。
    /// Convert two-dimensional plane coordinates back to a three-dimensional point.
    pub fn point3<S>(self, point: &PlanePoint2<S>, distance: S) -> PlanePoint3<S>
    where
        S: Field + Float,
    {
        Point3::new(
            self.value_for_axis(Axis3::X, point, distance),
            self.value_for_axis(Axis3::Y, point, distance),
            self.value_for_axis(Axis3::Z, point, distance),
        )
    }

    /// 根据距离值创建法向量。
    /// Create a normal vector from a distance value.
    pub fn vector<S>(self, distance: S) -> PlaneVector3<S>
    where
        S: Field + Float,
    {
        match self.normal_axis() {
            Axis3::X => Vector3::new(distance, S::zero(), S::zero()),
            Axis3::Y => Vector3::new(S::zero(), distance, S::zero()),
            Axis3::Z => Vector3::new(S::zero(), S::zero(), distance),
        }
    }

    /// 计算长方体在平面上的投影矩形。
    /// Compute the footprint rectangle of a cuboid on the plane.
    pub fn footprint<S>(self, cuboid: &Cuboid3<S>) -> Rectangle2<S>
    where
        S: Field + Float,
    {
        Rectangle2::new(
            cuboid.along(self.first_axis),
            cuboid.along(self.second_axis),
        )
    }

    fn value_for_axis<S>(self, axis: Axis3, point: &PlanePoint2<S>, distance: S) -> S
    where
        S: Field + Float,
    {
        if self.first_axis == axis {
            point.x()
        } else if self.second_axis == axis {
            point.y()
        } else {
            distance
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_frame_reports_axes_and_distance() {
        let frame = PlaneFrame3::XZ;
        let point = Point3::new(2.0, 3.0, 4.0);

        assert_eq!(frame.normal_axis(), Axis3::Y);
        assert_eq!(frame.axis_plane(), AxisPlane3::XZ);
        assert_eq!(frame.distance(&point), 3.0);
    }

    #[test]
    fn plane_frame_converts_points_between_2d_and_3d() {
        let frame = PlaneFrame3::ZY;
        let point3 = Point3::new(1.0, 2.0, 3.0);

        let point2 = frame.point2(&point3);

        assert_eq!(point2, Point2::new(3.0, 2.0));
        assert_eq!(frame.point3(&point2, 1.0), point3);
    }

    #[test]
    fn plane_frame_creates_normal_vector_and_footprint() {
        let frame = PlaneFrame3::YX;
        let cuboid = Cuboid3::new(2.0, 3.0, 4.0);

        assert_eq!(frame.vector(5.0), Vector3::new(0.0, 0.0, 5.0));
        assert_eq!(frame.footprint(&cuboid), Rectangle2::new(3.0, 2.0));
    }

    #[test]
    fn plane_frame_new_rejects_duplicate_axes() {
        assert_eq!(PlaneFrame3::new(Axis3::X, Axis3::X), None);
        assert_eq!(PlaneFrame3::new(Axis3::X, Axis3::Y), Some(PlaneFrame3::XY));
    }
}
