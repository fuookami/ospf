//! 二维包围盒几何 / 2D bounding box geometry
//!
//! 在量纲包围盒之上提供角点、包含与相交能力。圆形件以外接正方形表示，圆心位于该正方形中心，
//! 因此圆×圆、圆×矩形、矩形×矩形的重叠判定都走同一套包围盒逻辑。
//! Adds corners, containment and intersection on top of a quantity bounding box. A circle is represented by
//! its enclosing square with the circle centre at its centre, so circle-circle, circle-rectangle and
//! rectangle-rectangle overlap tests all share one bounding-box implementation.
//!
//! # 严格不等式 / Strict inequalities
//!
//! 重叠统一为**内部重叠**：相切不算重叠。实现刻意使用严格不等式，而不是沿用底层量值库把相切判为
//! 重叠的语义——否则任何规则网格排样都会被判为重叠。
//! Overlap is always **interior** overlap: tangency does not count. The implementation deliberately uses
//! strict inequalities rather than the underlying quantity library's tangency-counts-as-overlap semantics;
//! otherwise no regular grid layout could ever be accepted.

use ospf_rust_base::error::Ret;
use ospf_rust_math::algebra::concept::Field;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::point::{QuantityPoint2, QuantityVector2};
use crate::quantity_core::{quantity_minus_safe, quantity_plus_safe, quantity_ord_or_null};
use crate::shape::PackingShape2;
use std::cmp::Ordering;
use std::ops::Add;

// ============================================================================
// Box2Geometry - 二维包围盒 / 2D bounding box
// ============================================================================

/// 二维包围盒 / 2D bounding box
///
/// 用于装载形状之间的重叠与包含判定。
/// Used for overlap and containment tests between packing shapes.
#[derive(Debug, Clone)]
pub struct Box2Geometry<V, U: UnitTrait> {
    /// 左下角 x 坐标 / Lower-left x coordinate
    pub x: Quantity<V, U>,
    /// 左下角 y 坐标 / Lower-left y coordinate
    pub y: Quantity<V, U>,
    /// 包围形状 / Bounding shape
    pub shape: PackingShape2<V, U>,
}

impl<V, U> Box2Geometry<V, U>
where
    V: Clone + PartialOrd + Add<Output = V>,
    U: UnitTrait,
{
    /// 已归一化矩形的内部重叠，无 Float 或 solver 转换要求。
    /// Interior overlap of normalized rectangles, without Float or solver conversion requirements.
    ///
    /// 非矩形、单位未统一或不可比较时返回 None；调用方先验证有限性和正尺寸。
    /// Returns None for nonrectangles, inconsistent units or incomparable values; callers first validate finite positive extents.
    pub fn rectangle_overlapped(&self, other: &Self) -> Option<bool> {
        let (PackingShape2::Rectangle(first), PackingShape2::Rectangle(second)) = (&self.shape, &other.shape) else { return None; };
        let quantities = [&self.x, &self.y, &first.width, &first.height, &other.x, &other.y, &second.width, &second.height];
        let unit = &self.x.unit;
        if quantities.iter().any(|quantity| quantity.unit.dimension_symbol() != unit.dimension_symbol()
            || quantity.unit.scale_value() != unit.scale_value()) { return None; }
        let first_max_x = self.x.value.clone() + first.width.value.clone();
        let first_max_y = self.y.value.clone() + first.height.value.clone();
        let second_max_x = other.x.value.clone() + second.width.value.clone();
        let second_max_y = other.y.value.clone() + second.height.value.clone();
        Some(first_max_x.partial_cmp(&other.x.value)? == Ordering::Greater
            && second_max_x.partial_cmp(&self.x.value)? == Ordering::Greater
            && first_max_y.partial_cmp(&other.y.value)? == Ordering::Greater
            && second_max_y.partial_cmp(&self.y.value)? == Ordering::Greater)
    }
}

impl<V, U> Box2Geometry<V, U>
where
    U: UnitTrait,
{
    /// 由角点与形状构造 / Build a box from an origin and a shape
    pub fn new(x: Quantity<V, U>, y: Quantity<V, U>, shape: PackingShape2<V, U>) -> Self {
        Self { x, y, shape }
    }

    /// 由角点与尺寸构造矩形包围盒 / Build a rectangle box from an origin and a size
    pub fn of_rectangle(
        x: Quantity<V, U>,
        y: Quantity<V, U>,
        width: Quantity<V, U>,
        height: Quantity<V, U>,
    ) -> Self {
        Self::new(
            x,
            y,
            PackingShape2::Rectangle(crate::shape::RectanglePackingShape2 { width, height }),
        )
    }
}

impl<V, U> Box2Geometry<V, U>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{

    /// 包围宽度 / Bounding width
    pub fn width(&self) -> Quantity<V, U> {
        self.shape.bounding_width()
    }

    /// 包围高度 / Bounding height
    pub fn height(&self) -> Quantity<V, U> {
        self.shape.bounding_height()
    }

    /// 左下角 / Lower-left corner
    pub fn min(&self) -> QuantityPoint2<V, U> {
        QuantityPoint2 {
            x: self.x.clone(),
            y: self.y.clone(),
        }
    }

    /// 右上角 / Top-right corner
    pub fn max(&self) -> Ret<QuantityPoint2<V, U>> {
        let x = quantity_plus_safe(&self.x, &self.width())?;
        let y = quantity_plus_safe(&self.y, &self.height())?;
        Ok(QuantityPoint2 { x, y })
    }

    /// 是否包含另一点 / Whether the box contains a point
    ///
    /// 使用闭区间：边界上的点属于该盒，与「相切不算重叠」并不冲突——包含与重叠是不同问题。
    /// Uses closed intervals: a point on the boundary belongs to the box. This does not conflict with
    /// "tangency is not overlap", since containment and overlap are different questions.
    pub fn contains(&self, point: &QuantityPoint2<V, U>) -> bool {
        let (Ok(max_x), Ok(max_y)) = (self.max_x(), self.max_y()) else {
            return false;
        };
        matches!(
            quantity_ord_or_null(&point.x, &self.x),
            Some(Ordering::Greater) | Some(Ordering::Equal)
        ) && matches!(
            quantity_ord_or_null(&point.x, &max_x),
            Some(Ordering::Less) | Some(Ordering::Equal)
        ) && matches!(
            quantity_ord_or_null(&point.y, &self.y),
            Some(Ordering::Greater) | Some(Ordering::Equal)
        ) && matches!(
            quantity_ord_or_null(&point.y, &max_y),
            Some(Ordering::Less) | Some(Ordering::Equal)
        )
    }

    /// 是否包含另一个盒 / Whether the box contains another box
    pub fn contains_box(&self, other: &Self) -> bool {
        let (Ok(other_max_x), Ok(other_max_y)) = (other.max_x(), other.max_y()) else {
            return false;
        };
        let (Ok(self_max_x), Ok(self_max_y)) = (self.max_x(), self.max_y()) else {
            return false;
        };
        matches!(
            quantity_ord_or_null(&other.x, &self.x),
            Some(Ordering::Greater) | Some(Ordering::Equal)
        ) && matches!(
            quantity_ord_or_null(&other.y, &self.y),
            Some(Ordering::Greater) | Some(Ordering::Equal)
        ) && matches!(
            quantity_ord_or_null(&other_max_x, &self_max_x),
            Some(Ordering::Less) | Some(Ordering::Equal)
        ) && matches!(
            quantity_ord_or_null(&other_max_y, &self_max_y),
            Some(Ordering::Less) | Some(Ordering::Equal)
        )
    }

    /// 是否内部重叠 / Whether the boxes overlap in their interiors
    ///
    /// 相切（任一侧边贴合，或圆与圆外切）**不算**重叠。四个方向都必须严格相交才判重叠。
    /// Tangency (shared edge, or two externally tangent circles) does **not** count as overlap. All four
    /// directions must strictly intersect.
    pub fn overlapped(&self, other: &Self) -> bool {
        let (Ok(self_max_x), Ok(self_max_y)) = (self.max_x(), self.max_y()) else {
            return false;
        };
        let (Ok(other_max_x), Ok(other_max_y)) = (other.max_x(), other.max_y()) else {
            return false;
        };
        let strictly_greater_left = matches!(
            quantity_ord_or_null(&self_max_x, &other.x),
            Some(Ordering::Greater)
        );
        let strictly_greater_right = matches!(
            quantity_ord_or_null(&other_max_x, &self.x),
            Some(Ordering::Greater)
        );
        let strictly_greater_bottom = matches!(
            quantity_ord_or_null(&self_max_y, &other.y),
            Some(Ordering::Greater)
        );
        let strictly_greater_top = matches!(
            quantity_ord_or_null(&other_max_y, &self.y),
            Some(Ordering::Greater)
        );
        strictly_greater_left && strictly_greater_right && strictly_greater_bottom && strictly_greater_top
    }

    /// 是否与任意其他盒重叠 / Whether the box overlaps any of the given boxes
    pub fn overlaps_any(&self, others: &[Self]) -> bool {
        others.iter().any(|other| self.overlapped(other))
    }

    /// 按向量平移 / Translate by a vector
    pub fn translated(&self, offset: &QuantityVector2<V, U>) -> Ret<Self> {
        Ok(Self {
            x: quantity_plus_safe(&self.x, &offset.x)?,
            y: quantity_plus_safe(&self.y, &offset.y)?,
            shape: self.shape.clone(),
        })
    }

    /// 平移到指定角点 / Move to the given lower-left corner
    pub fn at(&self, x: Quantity<V, U>, y: Quantity<V, U>) -> Self {
        Self {
            x,
            y,
            shape: self.shape.clone(),
        }
    }

    /// 右边界 x 坐标 / Right boundary x coordinate
    pub fn max_x(&self) -> Ret<Quantity<V, U>> {
        quantity_plus_safe(&self.x, &self.width())
    }

    /// 上边界 y 坐标 / Upper boundary y coordinate
    pub fn max_y(&self) -> Ret<Quantity<V, U>> {
        quantity_plus_safe(&self.y, &self.height())
    }

    /// 宽度差 / Width difference
    pub fn width_delta(&self, other: &Self) -> Ret<Quantity<V, U>> {
        quantity_minus_safe(&self.width(), &other.width())
    }
}
