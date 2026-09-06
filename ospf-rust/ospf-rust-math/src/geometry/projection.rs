//! 二维投影形状。
//! Two-dimensional projection shapes.

use crate::algebra::Field;
use crate::geometry::{Axis2, AxisPermutation2, Circle2, Quadrilateral2};
use num_traits::{Float, FloatConst};

/// 二维投影形状 trait。
/// Trait for two-dimensional projection shapes.
pub trait Projection2<S>
where
    S: Field + Float,
{
    /// 投影宽度。
    /// Projection width.
    fn width(&self) -> S;

    /// 投影高度。
    /// Projection height.
    fn height(&self) -> S;

    /// 投影面积。
    /// Projection area.
    fn area(&self) -> S;
}

/// 二维形状 trait 兼容命名。
/// Compatibility name for two-dimensional shape traits.
pub trait Shape2<S>: Projection2<S>
where
    S: Field + Float,
{
}

impl<S, P> Shape2<S> for P
where
    S: Field + Float,
    P: Projection2<S>,
{
}

/// 二维轴对齐矩形投影。
/// Two-dimensional axis-aligned rectangle projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle2<S = f64> {
    /// 宽度。
    /// Width.
    pub width: S,
    /// 高度。
    /// Height.
    pub height: S,
}

impl<S> Rectangle2<S> {
    /// 创建二维矩形投影。
    /// Create a two-dimensional rectangle projection.
    pub fn new(width: S, height: S) -> Self {
        Self { width, height }
    }

    /// 通过宽高创建二维矩形投影。
    /// Create a two-dimensional rectangle projection from width and height.
    pub fn from_size(width: S, height: S) -> Self {
        Self::new(width, height)
    }

    /// 宽度引用。
    /// Width reference.
    pub fn width_ref(&self) -> &S {
        &self.width
    }

    /// 高度引用。
    /// Height reference.
    pub fn height_ref(&self) -> &S {
        &self.height
    }
}

impl<S> Rectangle2<S>
where
    S: Field + Float,
{
    /// 投影面积。
    /// Projection area.
    pub fn area(&self) -> S {
        self.width * self.height
    }

    /// 投影周长。
    /// Projection perimeter.
    pub fn perimeter(&self) -> S {
        (self.width + self.height) * (S::one() + S::one())
    }
}

impl<S> Rectangle2<S>
where
    S: Clone,
{
    /// 宽度。
    /// Width.
    pub fn width(&self) -> S {
        self.width.clone()
    }

    /// 高度。
    /// Height.
    pub fn height(&self) -> S {
        self.height.clone()
    }

    /// 沿指定轴的尺寸。
    /// Dimension along the specified axis.
    pub fn along(&self, axis: Axis2) -> S {
        match axis {
            Axis2::X => self.width.clone(),
            Axis2::Y => self.height.clone(),
        }
    }
}

impl<S> Rectangle2<S>
where
    S: Field + Float,
{
    /// 创建以原点为左下角的四边形。
    /// Create a quadrilateral with the origin as the lower-left corner.
    pub fn to_quadrilateral_at_origin(&self) -> Quadrilateral2<S> {
        crate::geometry::Quadrilateral2::from_coords(
            [S::zero(), S::zero()],
            [self.width, S::zero()],
            [self.width, self.height],
            [S::zero(), self.height],
        )
    }

    /// 按轴置换宽高。
    /// Permute width and height by axes.
    pub fn permute(&self, permutation: AxisPermutation2) -> Self {
        permutation.apply_rectangle(self)
    }
}

impl<S> Projection2<S> for Rectangle2<S>
where
    S: Field + Float,
{
    fn width(&self) -> S {
        self.width
    }

    fn height(&self) -> S {
        self.height
    }

    fn area(&self) -> S {
        self.area()
    }
}

impl<S> Projection2<S> for Circle2<S>
where
    S: Field + Float + FloatConst,
{
    fn width(&self) -> S {
        self.diameter()
    }

    fn height(&self) -> S {
        self.diameter()
    }

    fn area(&self) -> S {
        self.area()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point2, Projection2};

    #[test]
    fn rectangle_reports_dimensions_and_area() {
        let rectangle = Rectangle2::new(3.0, 4.0);

        assert_eq!(rectangle.width(), 3.0);
        assert_eq!(rectangle.height(), 4.0);
        assert_eq!(rectangle.area(), 12.0);
        assert_eq!(rectangle.along(Axis2::X), 3.0);
        assert_eq!(rectangle.along(Axis2::Y), 4.0);
    }

    #[test]
    fn rectangle_permutation_swaps_dimensions() {
        let rectangle = Rectangle2::new(3.0, 4.0);

        assert_eq!(rectangle.permute(AxisPermutation2::XY), rectangle);
        assert_eq!(
            rectangle.permute(AxisPermutation2::YX),
            Rectangle2::new(4.0, 3.0)
        );
    }

    #[test]
    fn rectangle_converts_to_origin_quadrilateral() {
        let rectangle = Rectangle2::new(3.0, 4.0);
        let quadrilateral = rectangle.to_quadrilateral_at_origin();

        assert_eq!(quadrilateral.p1(), &Point2::new(0.0, 0.0));
        assert_eq!(quadrilateral.p3(), &Point2::new(3.0, 4.0));
        assert_eq!(quadrilateral.area(), 12.0);
    }

    #[test]
    fn existing_circle2_implements_projection2() {
        let circle = Circle2::new(Point2::new(1.0, 2.0), 3.0);

        assert_eq!(Projection2::width(&circle), 6.0);
        assert_eq!(Projection2::height(&circle), 6.0);
        assert!((Projection2::area(&circle) - std::f64::consts::PI * 9.0).abs() < 1e-10);
    }
}
