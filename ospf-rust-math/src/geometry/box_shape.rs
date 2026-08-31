//! 包围盒形状枚举。
//! Bounding-box shape enums.

use num_traits::{Float, FloatConst};
use crate::algebra::Field;
use crate::geometry::{Circle2, Cuboid3, Cylinder3, Projection2, Rectangle2, Shape3};

/// 可用于二维包围盒的形状。
/// Shape that can be used by a two-dimensional bounding box.
#[derive(Debug, Clone, PartialEq)]
pub enum Box2Shape<S = f64> {
    /// 轴对齐矩形。
    /// Axis-aligned rectangle.
    Rectangle(Rectangle2<S>),
    /// 圆形投影，包围盒逻辑只使用半径。
    /// Circular projection; bounding-box logic only uses the radius.
    Circle(Circle2<S>),
}

impl<S> Box2Shape<S>
where
    S: Clone,
{
    /// 矩形宽度，非矩形返回 `None`。
    /// Rectangle width, or `None` for non-rectangular shapes.
    pub fn rectangle_width(&self) -> Option<S> {
        match self {
            Self::Rectangle(rectangle) => Some(rectangle.width()),
            Self::Circle(_) => None,
        }
    }

    /// 矩形高度，非矩形返回 `None`。
    /// Rectangle height, or `None` for non-rectangular shapes.
    pub fn rectangle_height(&self) -> Option<S> {
        match self {
            Self::Rectangle(rectangle) => Some(rectangle.height()),
            Self::Circle(_) => None,
        }
    }
}

impl<S> Box2Shape<S>
where
    S: Field + Float,
{
    /// 形状宽度。
    /// Shape width.
    pub fn width(&self) -> S {
        match self {
            Self::Rectangle(rectangle) => rectangle.width,
            Self::Circle(circle) => circle.radius() * (S::one() + S::one()),
        }
    }

    /// 形状高度。
    /// Shape height.
    pub fn height(&self) -> S {
        match self {
            Self::Rectangle(rectangle) => rectangle.height,
            Self::Circle(circle) => circle.radius() * (S::one() + S::one()),
        }
    }

    /// 圆半径，非圆形返回 `None`。
    /// Circle radius, or `None` for non-circular shapes.
    pub fn radius(&self) -> Option<S> {
        match self {
            Self::Rectangle(_) => None,
            Self::Circle(circle) => Some(circle.radius()),
        }
    }
}

impl<S> Projection2<S> for Box2Shape<S>
where
    S: Field + Float + FloatConst,
{
    fn width(&self) -> S {
        self.width()
    }

    fn height(&self) -> S {
        self.height()
    }

    fn area(&self) -> S {
        match self {
            Self::Rectangle(rectangle) => rectangle.area(),
            Self::Circle(circle) => circle.area(),
        }
    }
}

impl<S> From<Rectangle2<S>> for Box2Shape<S> {
    fn from(value: Rectangle2<S>) -> Self {
        Self::Rectangle(value)
    }
}

impl<S> From<Circle2<S>> for Box2Shape<S> {
    fn from(value: Circle2<S>) -> Self {
        Self::Circle(value)
    }
}

/// 可用于三维放置的形状。
/// Shape that can be used by a three-dimensional placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape3Kind<S: Field + Float = f64> {
    /// 长方体。
    /// Cuboid.
    Cuboid(Cuboid3<S>),
    /// 轴对齐圆柱体。
    /// Axis-aligned cylinder.
    Cylinder(Cylinder3<S>),
}

impl<S> Shape3Kind<S>
where
    S: Field + Float,
{
    /// 返回包围长方体。
    /// Return the bounding cuboid.
    pub fn bounding_cuboid(&self) -> Cuboid3<S> {
        match self {
            Self::Cuboid(cuboid) => cuboid.bounding_cuboid(),
            Self::Cylinder(cylinder) => cylinder.bounding_cuboid(),
        }
    }
}

impl<S> Shape3<S> for Shape3Kind<S>
where
    S: Field + Float,
{
    fn bounding_cuboid(&self) -> Cuboid3<S> {
        self.bounding_cuboid()
    }
}

impl<S> From<Cuboid3<S>> for Shape3Kind<S>
where
    S: Field + Float,
{
    fn from(value: Cuboid3<S>) -> Self {
        Self::Cuboid(value)
    }
}

impl<S> From<Cylinder3<S>> for Shape3Kind<S>
where
    S: Field + Float,
{
    fn from(value: Cylinder3<S>) -> Self {
        Self::Cylinder(value)
    }
}
