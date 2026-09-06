//! 三维尺寸形状。
//! Three-dimensional dimensional shapes.

use crate::algebra::Field;
use crate::geometry::{Axis3, AxisPermutation3, AxisPlane3, Circle2, Point2, Rectangle2};
use num_traits::{Float, FloatConst};

/// 三维形状 trait。
/// Trait for three-dimensional shapes.
pub trait Shape3<S>
where
    S: Field + Float,
{
    /// 返回包围长方体。
    /// Return the bounding cuboid.
    fn bounding_cuboid(&self) -> Cuboid3<S>;
}

/// 三维长方体形状。
/// Three-dimensional cuboid shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cuboid3<S: Field + Float = f64> {
    /// 宽度（X 轴）。
    /// Width on the X axis.
    pub width: S,
    /// 高度（Y 轴）。
    /// Height on the Y axis.
    pub height: S,
    /// 深度（Z 轴）。
    /// Depth on the Z axis.
    pub depth: S,
}

impl<S> Cuboid3<S>
where
    S: Field + Float,
{
    /// 创建三维长方体。
    /// Create a three-dimensional cuboid.
    pub fn new(width: S, height: S, depth: S) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    /// 通过宽高深创建三维长方体。
    /// Create a three-dimensional cuboid from width, height, and depth.
    pub fn from_size(width: S, height: S, depth: S) -> Self {
        Self::new(width, height, depth)
    }

    /// 体积。
    /// Volume.
    pub fn volume(&self) -> S {
        self.width * self.height * self.depth
    }

    /// 表面积。
    /// Surface area.
    pub fn surface_area(&self) -> S {
        let two = S::one() + S::one();
        two * (self.width * self.height + self.width * self.depth + self.height * self.depth)
    }

    /// 沿指定轴的尺寸。
    /// Dimension along the specified axis.
    pub fn along(&self, axis: Axis3) -> S {
        match axis {
            Axis3::X => self.width,
            Axis3::Y => self.height,
            Axis3::Z => self.depth,
        }
    }

    /// 按轴置换宽高深。
    /// Permute width, height, and depth by axes.
    pub fn permute(&self, permutation: AxisPermutation3) -> Self {
        permutation.apply_cuboid(self)
    }
}

impl<S> Shape3<S> for Cuboid3<S>
where
    S: Field + Float,
{
    fn bounding_cuboid(&self) -> Cuboid3<S> {
        *self
    }
}

/// 三维轴对齐线段。
/// Three-dimensional axis-aligned line segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisLine3<S: Field + Float = f64> {
    /// 对齐轴。
    /// Aligned axis.
    pub axis: Axis3,
    /// 起始值。
    /// Start value.
    pub from: S,
    /// 终止值。
    /// End value.
    pub to: S,
}

impl<S> AxisLine3<S>
where
    S: Field + Float,
{
    /// 创建三维轴对齐线段。
    /// Create a three-dimensional axis-aligned line segment.
    pub fn new(axis: Axis3, from: S, to: S) -> Self {
        Self { axis, from, to }
    }

    /// 线段长度。
    /// Segment length.
    pub fn length(&self) -> S {
        (self.to - self.from).abs()
    }
}

/// 三维圆柱体投影结果。
/// Projection result of a three-dimensional cylinder.
#[derive(Debug, Clone, PartialEq)]
pub enum CylinderProjection2<S: Field + Float = f64> {
    /// 矩形投影。
    /// Rectangle projection.
    Rectangle(Rectangle2<S>),
    /// 圆形投影。
    /// Circle projection.
    Circle(Circle2<S>),
}

/// 三维圆柱体。
/// Three-dimensional cylinder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cylinder3<S: Field + Float = f64> {
    /// 半径。
    /// Radius.
    pub radius: S,
    /// 高度。
    /// Height.
    pub height: S,
    /// 对齐轴。
    /// Alignment axis.
    pub axis: Axis3,
}

impl<S> Cylinder3<S>
where
    S: Field + Float,
{
    /// 创建三维圆柱体。
    /// Create a three-dimensional cylinder.
    pub fn new(radius: S, height: S, axis: Axis3) -> Self {
        Self {
            radius,
            height,
            axis,
        }
    }

    /// 直径。
    /// Diameter.
    pub fn diameter(&self) -> S {
        self.radius * (S::one() + S::one())
    }

    /// 沿指定轴的尺寸。
    /// Dimension along the specified axis.
    pub fn along(&self, axis: Axis3) -> S {
        if axis == self.axis {
            self.height
        } else {
            self.diameter()
        }
    }

    /// 原点处的轴线段。
    /// Axis line segment at the origin.
    pub fn axis_line_at_origin(&self) -> AxisLine3<S> {
        AxisLine3::new(self.axis, S::zero(), self.height)
    }

    /// 包围长方体。
    /// Bounding cuboid.
    pub fn bounding_cuboid(&self) -> Cuboid3<S> {
        match self.axis {
            Axis3::X => Cuboid3::new(self.height, self.diameter(), self.diameter()),
            Axis3::Y => Cuboid3::new(self.diameter(), self.height, self.diameter()),
            Axis3::Z => Cuboid3::new(self.diameter(), self.diameter(), self.height),
        }
    }

    /// 按轴置换。
    /// Permute by axes.
    pub fn permute(&self, permutation: AxisPermutation3) -> Self {
        permutation.apply_cylinder(self)
    }
}

impl<S> Cylinder3<S>
where
    S: Field + Float + FloatConst,
{
    /// 底面积。
    /// Base area.
    pub fn base_area(&self) -> S {
        S::PI() * self.radius * self.radius
    }

    /// 体积。
    /// Volume.
    pub fn volume(&self) -> S {
        self.base_area() * self.height
    }

    /// 在指定主平面上的投影。
    /// Projection on the specified principal plane.
    pub fn projection_on(&self, plane: AxisPlane3) -> CylinderProjection2<S> {
        if plane.contains(self.axis) {
            CylinderProjection2::Rectangle(Rectangle2::new(
                self.along(plane.first_axis()),
                self.along(plane.second_axis()),
            ))
        } else {
            CylinderProjection2::Circle(Circle2::new(Point2::origin(), self.radius))
        }
    }
}

impl<S> Shape3<S> for Cylinder3<S>
where
    S: Field + Float,
{
    fn bounding_cuboid(&self) -> Cuboid3<S> {
        self.bounding_cuboid()
    }
}

/// 三维轴对齐圆柱体兼容命名。
/// Compatibility name for three-dimensional axis-aligned cylinders.
pub type AxisAlignedCylinder3<S = f64> = Cylinder3<S>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuboid_reports_dimensions_and_volume() {
        let cuboid = Cuboid3::new(2.0, 3.0, 4.0);

        assert_eq!(cuboid.along(Axis3::X), 2.0);
        assert_eq!(cuboid.along(Axis3::Y), 3.0);
        assert_eq!(cuboid.along(Axis3::Z), 4.0);
        assert_eq!(cuboid.volume(), 24.0);
        assert_eq!(cuboid.surface_area(), 52.0);
    }

    #[test]
    fn cuboid_permutation_reorders_dimensions() {
        let cuboid = Cuboid3::new(2.0, 3.0, 4.0);

        assert_eq!(cuboid.permute(AxisPermutation3::XYZ), cuboid);
        assert_eq!(
            cuboid.permute(AxisPermutation3::ZXY),
            Cuboid3::new(4.0, 2.0, 3.0)
        );
    }

    #[test]
    fn cylinder_reports_dimensions_and_bounding_cuboid() {
        let cylinder = Cylinder3::new(2.0, 5.0, Axis3::Y);

        assert_eq!(cylinder.diameter(), 4.0);
        assert_eq!(cylinder.along(Axis3::Y), 5.0);
        assert_eq!(cylinder.along(Axis3::X), 4.0);
        assert_eq!(cylinder.bounding_cuboid(), Cuboid3::new(4.0, 5.0, 4.0));
        assert_eq!(
            cylinder.axis_line_at_origin(),
            AxisLine3::new(Axis3::Y, 0.0, 5.0)
        );
    }

    #[test]
    fn cylinder_volume_uses_pi() {
        let cylinder = Cylinder3::new(2.0, 5.0, Axis3::Z);

        assert!((cylinder.base_area() - std::f64::consts::PI * 4.0).abs() < 1e-10);
        assert!((cylinder.volume() - std::f64::consts::PI * 20.0).abs() < 1e-10);
    }

    #[test]
    fn cylinder_projection_matches_axis_relation() {
        let cylinder = Cylinder3::new(2.0, 5.0, Axis3::Z);

        assert_eq!(
            cylinder.projection_on(AxisPlane3::XY),
            CylinderProjection2::Circle(Circle2::new(Point2::origin(), 2.0))
        );
        assert_eq!(
            cylinder.projection_on(AxisPlane3::XZ),
            CylinderProjection2::Rectangle(Rectangle2::new(4.0, 5.0))
        );
    }

    #[test]
    fn cylinder_permutation_maps_alignment_axis() {
        let cylinder = Cylinder3::new(2.0, 5.0, Axis3::Z);

        assert_eq!(
            cylinder.permute(AxisPermutation3::ZXY),
            Cylinder3::new(2.0, 5.0, Axis3::X)
        );
    }
}
