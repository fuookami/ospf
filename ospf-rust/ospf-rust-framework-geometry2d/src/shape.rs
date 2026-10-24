//! 二维装载形状 / 2D packing shapes
//!
//! 形状提供包围尺寸、实际面积与外接面积。面积口径：矩形 `width × height`，圆形 `πr²`（外接 `4r²`）。
//! 目标函数与 KPI 使用实际面积，网格生成使用包围尺寸。
//! Shapes provide bounding size, actual area and bounding area. Area convention: rectangles use
//! `width × height`, circles use `πr²` (bounding `4r²`). Objectives and KPI use the actual area, while grid
//! generation uses the bounding size.

use ospf_rust_base::error::Ret;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::quantity_core::{pi_value, quantity_is_positive, quantity_multiply, two_value};

// ============================================================================
// PackingShape2Type - 形状类型 / Shape type
// ============================================================================

/// 装载形状类型 / Packing shape type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackingShape2Type {
    /// 矩形件 / Rectangle item
    Rectangle,
    /// 圆形件 / Circle item
    Circle,
}

// ============================================================================
// PackingShape2 - 二维装载形状 / 2D packing shape
// ============================================================================

/// 二维装载形状 / 2D packing shape
///
/// 泛型于数值类型 `V` 与长度单位 `U`。面积以 `Quantity<V, Unit>` 表达，因为面积是长度平方，
/// 与长度不是同一量纲，必须由量值乘法产生。
/// Generic over the value type `V` and the length unit `U`. The area is expressed as `Quantity<V, Unit>`
/// because an area is a squared length and therefore not dimensionally equal to a length; it must be
/// produced by quantity multiplication.
#[derive(Debug, Clone)]
pub enum PackingShape2<V, U: UnitTrait> {
    /// 矩形 / Rectangle
    Rectangle(RectanglePackingShape2<V, U>),
    /// 圆形 / Circle
    Circle(CirclePackingShape2<V, U>),
}

impl<V, U> PackingShape2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 形状类型 / Shape type
    pub fn shape_type(&self) -> PackingShape2Type {
        match self {
            Self::Rectangle(_) => PackingShape2Type::Rectangle,
            Self::Circle(_) => PackingShape2Type::Circle,
        }
    }

    /// 包围宽度 / Bounding width
    pub fn bounding_width(&self) -> Quantity<V, U> {
        match self {
            Self::Rectangle(rectangle) => rectangle.width.clone(),
            Self::Circle(circle) => circle.diameter(),
        }
    }

    /// 包围高度 / Bounding height
    pub fn bounding_height(&self) -> Quantity<V, U> {
        match self {
            Self::Rectangle(rectangle) => rectangle.height.clone(),
            Self::Circle(circle) => circle.diameter(),
        }
    }

    /// 实际占用面积 / Actually occupied area
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        match self {
            Self::Rectangle(rectangle) => rectangle.area(),
            Self::Circle(circle) => circle.area(),
        }
    }

    /// 外接矩形面积 / Enclosing rectangle area
    pub fn bounding_area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        match self {
            Self::Rectangle(rectangle) => rectangle.area(),
            Self::Circle(circle) => circle.bounding_area(),
        }
    }

    /// 旋转 90 度；圆形件旋转后不变 / Rotate by 90 degrees; circles are unchanged
    pub fn rotated(&self) -> Self {
        match self {
            Self::Rectangle(rectangle) => Self::Rectangle(rectangle.rotated()),
            Self::Circle(circle) => Self::Circle(circle.clone()),
        }
    }

    /// 是否圆形件 / Whether this is a circle
    pub fn is_circle(&self) -> bool {
        matches!(self, Self::Circle(_))
    }

    /// 是否矩形件 / Whether this is a rectangle
    pub fn is_rectangle(&self) -> bool {
        matches!(self, Self::Rectangle(_))
    }
}

// ============================================================================
// RectanglePackingShape2 - 矩形装载形状 / Rectangle packing shape
// ============================================================================

/// 矩形装载形状 / Rectangle packing shape
#[derive(Debug, Clone)]
pub struct RectanglePackingShape2<V, U: UnitTrait> {
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
}

impl<V, U> RectanglePackingShape2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建矩形形状 / Create a rectangle shape
    pub fn new(width: Quantity<V, U>, height: Quantity<V, U>) -> Self {
        Self { width, height }
    }

    /// 实际占用面积 / Actually occupied area
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        quantity_multiply(&self.width, &self.height)
    }

    /// 旋转 90 度 / Rotate by 90 degrees
    pub fn rotated(&self) -> Self {
        Self::new(self.height.clone(), self.width.clone())
    }

    /// 旋转 90 度是否得到不同形状 / Whether a 90-degree rotation yields a different shape
    pub fn is_rotatable(&self) -> bool {
        self.width.value != self.height.value
    }
}

// ============================================================================
// CirclePackingShape2 - 圆形装载形状 / Circle packing shape
// ============================================================================

/// 圆形装载形状 / Circle packing shape
#[derive(Debug, Clone)]
pub struct CirclePackingShape2<V, U: UnitTrait> {
    /// 半径 / Radius
    pub radius: Quantity<V, U>,
}

impl<V, U> CirclePackingShape2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建圆形形状 / Create a circle shape
    pub fn new(radius: Quantity<V, U>) -> Self {
        Self { radius }
    }

    /// 直径 / Diameter
    pub fn diameter(&self) -> Quantity<V, U> {
        crate::quantity_core::quantity_scaled_by(&self.radius, &two_value::<V>())
    }

    /// 实际占用面积 `πr²` / Actual occupied area `πr²`
    ///
    /// 圆形件的实际面积与包围面积不同，这是面积口径的关键差异。
    /// A circle's actual area differs from its bounding area, which is the key difference in the area
    /// convention.
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        let squared = quantity_multiply(&self.radius, &self.radius);
        let pi = pi_value::<V>();
        Quantity::new(squared.value * pi, squared.unit)
    }

    /// 外接矩形面积 `4r²` / Enclosing rectangle area `4r²`
    pub fn bounding_area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        let diameter = self.diameter();
        quantity_multiply(&diameter, &diameter)
    }
}

// ============================================================================
// Shape2Candidate - 候选形状 / Candidate shape
// ============================================================================

/// 带旋转标记的候选形状 / Candidate shape carrying its rotation flag
#[derive(Debug, Clone)]
pub struct Shape2Candidate<V, U: UnitTrait> {
    /// 候选形状 / Candidate shape
    pub shape: PackingShape2<V, U>,
    /// 是否旋转 90 度 / Whether rotated by 90 degrees
    pub rotated: bool,
}

// ============================================================================
// 候选生成 / Candidate generation
// ============================================================================

/// 按允许旋转与否返回候选形状列表 / Candidate shapes honouring the rotation allowance
///
/// 圆形件与不可旋转矩形只返回自身。
/// Circle shapes and non-rotatable rectangles only return themselves.
pub fn candidate_shapes<V, U>(
    shape: &PackingShape2<V, U>,
    allow_rotate: bool,
) -> Vec<Shape2Candidate<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let upright = Shape2Candidate {
        shape: shape.clone(),
        rotated: false,
    };
    if !allow_rotate {
        return vec![upright];
    }
    let Some(rectangle) = shape.rectangle().filter(|rectangle| rectangle.is_rotatable()) else {
        return vec![upright];
    };
    vec![
        upright,
        Shape2Candidate {
            shape: PackingShape2::Rectangle(rectangle.rotated()),
            rotated: true,
        },
    ]
}

impl<V, U> PackingShape2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 取矩形视图 / Take the rectangle view
    pub fn rectangle(&self) -> Option<&RectanglePackingShape2<V, U>> {
        match self {
            Self::Rectangle(rectangle) => Some(rectangle),
            Self::Circle(_) => None,
        }
    }

    /// 取圆形视图 / Take the circle view
    pub fn circle(&self) -> Option<&CirclePackingShape2<V, U>> {
        match self {
            Self::Circle(circle) => Some(circle),
            Self::Rectangle(_) => None,
        }
    }
}

// ============================================================================
// 构造器 / Constructors
// ============================================================================

/// 生成矩形形状；宽高必须为正 / Build a rectangle shape; width and height must be positive
pub fn rectangle_shape2<V, U>(
    width: Quantity<V, U>,
    height: Quantity<V, U>,
) -> Ret<RectanglePackingShape2<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    if !quantity_is_positive(&width) || !quantity_is_positive(&height) {
        return Err(Box::new(super::quantity_core::DimensionMismatch::new(
            width.unit.dimension_symbol(),
            height.unit.dimension_symbol(),
            "rectangle width and height must be positive",
        )));
    }
    Ok(RectanglePackingShape2::new(width, height))
}

/// 生成圆形形状；半径必须为正 / Build a circle shape; the radius must be positive
pub fn circle_shape2<V, U>(radius: Quantity<V, U>) -> Ret<CirclePackingShape2<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    if !quantity_is_positive(&radius) {
        return Err(Box::new(super::quantity_core::DimensionMismatch::new(
            radius.unit.dimension_symbol(),
            radius.unit.dimension_symbol(),
            "circle radius must be positive",
        )));
    }
    Ok(CirclePackingShape2::new(radius))
}
