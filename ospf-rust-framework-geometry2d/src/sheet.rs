//! 板材几何 / Sheet geometry
//!
//! 板材是有界的二维区域：宽高固定，左上角为原点。`Sheet2Shape` 同时给出包围盒与尺寸，
//! 供层生成与几何校验共用同一口径。
//! A sheet is a bounded 2D region with fixed width and height and the lower-left corner at the origin.
//! `Sheet2Shape` exposes both the bounding box and the size so that layer generation and geometry checks
//! share one convention.

use ospf_rust_base::error::Ret;
use ospf_rust_math::algebra::concept::Field;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::box_geometry::Box2Geometry;
use crate::quantity_core::{quantity_is_positive, quantity_multiply, quantity_zero_of};
use crate::shape::{PackingShape2, RectanglePackingShape2};

// ============================================================================
// Sheet2Shape - 板材形状 / Sheet shape
// ============================================================================

/// 板材形状 / Sheet shape
#[derive(Debug, Clone)]
pub struct Sheet2Shape<V, U: UnitTrait> {
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
}

impl<V, U> Sheet2Shape<V, U>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建板材形状 / Create a sheet shape
    pub fn new(width: Quantity<V, U>, height: Quantity<V, U>) -> Self {
        Self { width, height }
    }

    /// 板材面积 / Sheet area
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        quantity_multiply(&self.width, &self.height)
    }

    /// 对应的矩形装载形状 / The corresponding rectangle packing shape
    pub fn shape(&self) -> PackingShape2<V, U> {
        PackingShape2::Rectangle(RectanglePackingShape2::new(
            self.width.clone(),
            self.height.clone(),
        ))
    }

    /// 原点起算的板材包围盒 / The sheet bounding box anchored at the origin
    pub fn origin_box(&self) -> Box2Geometry<V, U> {
        Box2Geometry::new(
            quantity_zero_of(&self.width),
            quantity_zero_of(&self.height),
            self.shape(),
        )
    }
}

// ============================================================================
// 构造器 / Constructors
// ============================================================================

/// 构造板材形状；宽高必须为正 / Build a sheet shape; width and height must be positive
pub fn sheet2_shape<V, U>(width: Quantity<V, U>, height: Quantity<V, U>) -> Ret<Sheet2Shape<V, U>>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    if !quantity_is_positive(&width) || !quantity_is_positive(&height) {
        return Err(Box::new(crate::quantity_core::DimensionMismatch::new(
            width.unit.dimension_symbol(),
            height.unit.dimension_symbol(),
            "sheet width and height must be positive",
        )));
    }
    Ok(Sheet2Shape::new(width, height))
}

/// 构造从原点起算的板材包围盒 / Build a sheet bounding box anchored at the origin
///
/// 供只需要包围盒的调用方直接使用，避免重复构造 `Sheet2Shape`。
/// Lets callers that only need a bounding box avoid rebuilding `Sheet2Shape`.
pub fn origin_box<V, U>(
    width: Quantity<V, U>,
    height: Quantity<V, U>,
) -> Box2Geometry<V, U>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let x = quantity_zero_of(&width);
    let y = quantity_zero_of(&height);
    Box2Geometry::of_rectangle(x, y, width, height)
}

// ============================================================================
// Sheet2Geometry - 板材几何别名 / Sheet geometry alias
// ============================================================================

/// 板材几何 / Sheet geometry
///
/// `Sheet2Shape` 的别名，便于下游按「几何」而非「形状」表达语义。
/// Alias of `Sheet2Shape`, letting downstream code speak of "geometry" rather than "shape".
pub type Sheet2Geometry<V, U> = Sheet2Shape<V, U>;
