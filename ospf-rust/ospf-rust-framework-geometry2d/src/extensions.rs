//! 二维几何扩展 / 2D geometry extensions
//!
//! 提供面积、包围尺寸、边界与板材包含等派生查询。所有查询在量纲不兼容时以错误或 `Option` 暴露，
//! 不做隐式单位换算。
//! Provides derived queries for area, bounding size, boundaries and sheet containment. Every query surfaces
//! dimension mismatches as errors or `Option` values; no implicit unit conversion is performed.

use ospf_rust_base::error::Ret;
use ospf_rust_math::algebra::concept::Field;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::box_geometry::Box2Geometry;
use crate::quantity_core::quantity_multiply;
use crate::shape::PackingShape2;

// ============================================================================
// 面积 / Area
// ============================================================================

/// 形状的实际占用面积 / The actual occupied area of a shape
///
/// 矩形 `w × h`；圆形 `πr²`。
/// Rectangles use `w × h`; circles use `πr²`.
pub fn area_of<V, U>(shape: &PackingShape2<V, U>) -> Quantity<V, ospf_rust_quantities::unit::Unit>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    shape.area()
}

/// 包围盒的包围宽度 / The bounding width of a box
pub fn bounding_width<V, U>(b: &Box2Geometry<V, U>) -> Quantity<V, U>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    b.width()
}

/// 包围盒的包围高度 / The bounding height of a box
pub fn bounding_height<V, U>(b: &Box2Geometry<V, U>) -> Quantity<V, U>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    b.height()
}

/// 包围盒的外接矩形面积 / The enclosing rectangle area of a box
pub fn bounding_area<V, U>(
    b: &Box2Geometry<V, U>,
) -> Quantity<V, ospf_rust_quantities::unit::Unit>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    quantity_multiply(&b.width(), &b.height())
}

// ============================================================================
// 边界 / Boundaries
// ============================================================================

/// 左边界 x 坐标 / Left boundary x coordinate
pub fn min_x<V, U>(b: &Box2Geometry<V, U>) -> Quantity<V, U>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    b.x.clone()
}

/// 下边界 y 坐标 / Lower boundary y coordinate
pub fn min_y<V, U>(b: &Box2Geometry<V, U>) -> Quantity<V, U>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    b.y.clone()
}

/// 右边界 x 坐标 / Right boundary x coordinate
pub fn max_x<V, U>(b: &Box2Geometry<V, U>) -> Ret<Quantity<V, U>>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    b.max_x()
}

/// 上边界 y 坐标 / Upper boundary y coordinate
pub fn max_y<V, U>(b: &Box2Geometry<V, U>) -> Ret<Quantity<V, U>>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    b.max_y()
}

// ============================================================================
// 板材包含 / Sheet containment
// ============================================================================

/// 判断包围盒是否完全落在板材内 / Whether a box lies fully inside a sheet
///
/// 板材以原点起算的包围盒表达；边界贴合视为在内（板材边界不是刀缝）。
/// The sheet is given as its origin-anchored bounding box; touching the boundary counts as inside, since a
/// sheet boundary is not a kerf.
pub fn inside_sheet<V, U>(b: &Box2Geometry<V, U>, sheet: &Box2Geometry<V, U>) -> bool
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    sheet.contains_box(b)
}
