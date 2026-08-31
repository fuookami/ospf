//! 坐标轴和轴置换。
//! Coordinate axes and axis permutations.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::{Cuboid3, Cylinder3, Point2, Point3, Rectangle2, Vector2, Vector3};

/// 二维坐标轴。
/// Two-dimensional coordinate axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Axis2 {
    /// X 轴。
    /// X axis.
    X,
    /// Y 轴。
    /// Y axis.
    Y,
}

impl Axis2 {
    /// 所有二维坐标轴。
    /// All two-dimensional coordinate axes.
    pub const ALL: [Self; 2] = [Self::X, Self::Y];

    /// 返回坐标轴下标。
    /// Return the axis index.
    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
        }
    }

    /// 从下标创建坐标轴。
    /// Create an axis from an index.
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::X),
            1 => Some(Self::Y),
            _ => None,
        }
    }
}

/// 三维坐标轴。
/// Three-dimensional coordinate axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Axis3 {
    /// X 轴。
    /// X axis.
    X,
    /// Y 轴。
    /// Y axis.
    Y,
    /// Z 轴。
    /// Z axis.
    Z,
}

impl Axis3 {
    /// 所有三维坐标轴。
    /// All three-dimensional coordinate axes.
    pub const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    /// 返回坐标轴下标。
    /// Return the axis index.
    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }

    /// 从下标创建坐标轴。
    /// Create an axis from an index.
    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::X),
            1 => Some(Self::Y),
            2 => Some(Self::Z),
            _ => None,
        }
    }
}

/// 三维主平面。
/// Three-dimensional principal plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisPlane3 {
    /// XY 平面，法向轴为 Z。
    /// XY plane, whose normal axis is Z.
    XY,
    /// XZ 平面，法向轴为 Y。
    /// XZ plane, whose normal axis is Y.
    XZ,
    /// YZ 平面，法向轴为 X。
    /// YZ plane, whose normal axis is X.
    YZ,
}

impl AxisPlane3 {
    /// 所有三维主平面。
    /// All three-dimensional principal planes.
    pub const ALL: [Self; 3] = [Self::XY, Self::XZ, Self::YZ];

    /// 平面第一轴。
    /// The first axis of the plane.
    pub const fn first_axis(self) -> Axis3 {
        match self {
            Self::XY | Self::XZ => Axis3::X,
            Self::YZ => Axis3::Y,
        }
    }

    /// 平面第二轴。
    /// The second axis of the plane.
    pub const fn second_axis(self) -> Axis3 {
        match self {
            Self::XY => Axis3::Y,
            Self::XZ | Self::YZ => Axis3::Z,
        }
    }

    /// 平面法向轴。
    /// The normal axis of the plane.
    pub const fn normal_axis(self) -> Axis3 {
        match self {
            Self::XY => Axis3::Z,
            Self::XZ => Axis3::Y,
            Self::YZ => Axis3::X,
        }
    }

    /// 判断指定轴是否属于该平面。
    /// Check whether the specified axis belongs to this plane.
    pub const fn contains(self, axis: Axis3) -> bool {
        axis.index() == self.first_axis().index() || axis.index() == self.second_axis().index()
    }
}

/// 二维轴置换。
/// Two-dimensional axis permutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AxisPermutation2 {
    /// 宽度对应的轴。
    /// Axis corresponding to width.
    pub width_axis: Axis2,
    /// 高度对应的轴。
    /// Axis corresponding to height.
    pub height_axis: Axis2,
}

impl AxisPermutation2 {
    /// 宽度轴为 X，高度轴为 Y。
    /// Width axis is X, height axis is Y.
    pub const XY: Self = Self {
        width_axis: Axis2::X,
        height_axis: Axis2::Y,
    };

    /// 宽度轴为 Y，高度轴为 X。
    /// Width axis is Y, height axis is X.
    pub const YX: Self = Self {
        width_axis: Axis2::Y,
        height_axis: Axis2::X,
    };

    /// 创建二维轴置换。
    /// Create a two-dimensional axis permutation.
    pub const fn new(width_axis: Axis2, height_axis: Axis2) -> Self {
        Self {
            width_axis,
            height_axis,
        }
    }

    /// 置换二维点坐标。
    /// Permute a two-dimensional point.
    pub fn apply_point<S>(self, point: &Point2<S>) -> Point2<S>
    where
        S: Field + Float,
    {
        Point2::new(
            point[self.width_axis.index()],
            point[self.height_axis.index()],
        )
    }

    /// 置换二维向量分量。
    /// Permute a two-dimensional vector.
    pub fn apply_vector<S>(self, vector: &Vector2<S>) -> Vector2<S>
    where
        S: Field + Float,
    {
        Vector2::new(
            vector[self.width_axis.index()],
            vector[self.height_axis.index()],
        )
    }

    /// 置换二维矩形投影。
    /// Permute a two-dimensional rectangle projection.
    pub fn apply_rectangle<S>(self, rectangle: &Rectangle2<S>) -> Rectangle2<S>
    where
        S: Field + Float,
    {
        Rectangle2::new(
            rectangle.along(self.width_axis),
            rectangle.along(self.height_axis),
        )
    }
}

/// 三维轴置换。
/// Three-dimensional axis permutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AxisPermutation3 {
    /// 宽度对应的轴。
    /// Axis corresponding to width.
    pub width_axis: Axis3,
    /// 高度对应的轴。
    /// Axis corresponding to height.
    pub height_axis: Axis3,
    /// 深度对应的轴。
    /// Axis corresponding to depth.
    pub depth_axis: Axis3,
}

impl AxisPermutation3 {
    /// 恒等置换。
    /// Identity permutation.
    pub const XYZ: Self = Self {
        width_axis: Axis3::X,
        height_axis: Axis3::Y,
        depth_axis: Axis3::Z,
    };

    /// 完全反转。
    /// Full reversal.
    pub const ZYX: Self = Self {
        width_axis: Axis3::Z,
        height_axis: Axis3::Y,
        depth_axis: Axis3::X,
    };

    /// Y-X-Z 置换。
    /// Y-X-Z permutation.
    pub const YXZ: Self = Self {
        width_axis: Axis3::Y,
        height_axis: Axis3::X,
        depth_axis: Axis3::Z,
    };

    /// Z-X-Y 置换。
    /// Z-X-Y permutation.
    pub const ZXY: Self = Self {
        width_axis: Axis3::Z,
        height_axis: Axis3::X,
        depth_axis: Axis3::Y,
    };

    /// X-Z-Y 置换。
    /// X-Z-Y permutation.
    pub const XZY: Self = Self {
        width_axis: Axis3::X,
        height_axis: Axis3::Z,
        depth_axis: Axis3::Y,
    };

    /// Y-Z-X 置换。
    /// Y-Z-X permutation.
    pub const YZX: Self = Self {
        width_axis: Axis3::Y,
        height_axis: Axis3::Z,
        depth_axis: Axis3::X,
    };

    /// 创建三维轴置换，要求三个轴互不相同。
    /// Create a three-dimensional axis permutation with three distinct axes.
    pub const fn new(width_axis: Axis3, height_axis: Axis3, depth_axis: Axis3) -> Option<Self> {
        if width_axis.index() == height_axis.index()
            || width_axis.index() == depth_axis.index()
            || height_axis.index() == depth_axis.index()
        {
            None
        } else {
            Some(Self {
                width_axis,
                height_axis,
                depth_axis,
            })
        }
    }

    /// 将原始轴映射到置换后的标准轴。
    /// Map an original axis to its permuted standard axis.
    pub const fn map_axis(self, axis: Axis3) -> Option<Axis3> {
        if axis.index() == self.width_axis.index() {
            Some(Axis3::X)
        } else if axis.index() == self.height_axis.index() {
            Some(Axis3::Y)
        } else if axis.index() == self.depth_axis.index() {
            Some(Axis3::Z)
        } else {
            None
        }
    }

    /// 置换三维点坐标。
    /// Permute a three-dimensional point.
    pub fn apply_point<S>(self, point: &Point3<S>) -> Point3<S>
    where
        S: Field + Float,
    {
        Point3::new(
            point[self.width_axis.index()],
            point[self.height_axis.index()],
            point[self.depth_axis.index()],
        )
    }

    /// 置换三维向量分量。
    /// Permute a three-dimensional vector.
    pub fn apply_vector<S>(self, vector: &Vector3<S>) -> Vector3<S>
    where
        S: Field + Float,
    {
        Vector3::new(
            vector[self.width_axis.index()],
            vector[self.height_axis.index()],
            vector[self.depth_axis.index()],
        )
    }

    /// 置换三维长方体尺寸。
    /// Permute three-dimensional cuboid dimensions.
    pub fn apply_cuboid<S>(self, cuboid: &Cuboid3<S>) -> Cuboid3<S>
    where
        S: Field + Float,
    {
        Cuboid3::new(
            cuboid.along(self.width_axis),
            cuboid.along(self.height_axis),
            cuboid.along(self.depth_axis),
        )
    }

    /// 置换三维圆柱体对齐轴。
    /// Permute the alignment axis of a three-dimensional cylinder.
    pub fn apply_cylinder<S>(self, cylinder: &Cylinder3<S>) -> Cylinder3<S>
    where
        S: Field + Float,
    {
        Cylinder3::new(
            cylinder.radius,
            cylinder.height,
            self.map_axis(cylinder.axis).unwrap_or(cylinder.axis),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_indices_are_stable() {
        assert_eq!(Axis2::X.index(), 0);
        assert_eq!(Axis2::Y.index(), 1);
        assert_eq!(Axis3::Z.index(), 2);
        assert_eq!(Axis3::from_index(1), Some(Axis3::Y));
        assert_eq!(Axis3::from_index(3), None);
    }

    #[test]
    fn axis_plane_exposes_axes() {
        assert_eq!(AxisPlane3::XY.first_axis(), Axis3::X);
        assert_eq!(AxisPlane3::XY.second_axis(), Axis3::Y);
        assert_eq!(AxisPlane3::XY.normal_axis(), Axis3::Z);
        assert!(AxisPlane3::XZ.contains(Axis3::Z));
        assert!(!AxisPlane3::XZ.contains(Axis3::Y));
    }

    #[test]
    fn two_dimensional_permutation_applies_to_point_and_vector() {
        let point = Point2::new(1.0, 2.0);
        let vector = Vector2::new(3.0, 4.0);
        let rectangle = Rectangle2::new(5.0, 6.0);

        assert_eq!(
            AxisPermutation2::YX.apply_point(&point),
            Point2::new(2.0, 1.0)
        );
        assert_eq!(
            AxisPermutation2::YX.apply_vector(&vector),
            Vector2::new(4.0, 3.0)
        );
        assert_eq!(
            AxisPermutation2::YX.apply_rectangle(&rectangle),
            Rectangle2::new(6.0, 5.0)
        );
    }

    #[test]
    fn three_dimensional_permutation_requires_distinct_axes() {
        assert!(AxisPermutation3::new(Axis3::X, Axis3::Y, Axis3::Z).is_some());
        assert!(AxisPermutation3::new(Axis3::X, Axis3::X, Axis3::Z).is_none());
    }

    #[test]
    fn three_dimensional_permutation_maps_axes_and_values() {
        let permutation = AxisPermutation3::ZXY;
        let point = Point3::new(1.0, 2.0, 3.0);
        let vector = Vector3::new(4.0, 5.0, 6.0);

        assert_eq!(permutation.map_axis(Axis3::Z), Some(Axis3::X));
        assert_eq!(permutation.map_axis(Axis3::X), Some(Axis3::Y));
        assert_eq!(permutation.apply_point(&point), Point3::new(3.0, 1.0, 2.0));
        assert_eq!(
            permutation.apply_vector(&vector),
            Vector3::new(6.0, 4.0, 5.0)
        );
    }
}
