//! 二维装载放置 / 2D packing placement
//!
//! 一个放置由形状、左下角坐标、旋转标记与物品标识组成。物品标识用于层内件数统计。
//! A placement consists of a shape, a lower-left position, a rotation flag and an item identifier; the
//! identifier drives per-layer amount statistics.

use ospf_rust_base::error::Ret;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::point::QuantityPoint2;
use crate::quantity_core::quantity_plus_safe;
use crate::shape::PackingShape2;

// ============================================================================
// Placement2 - 二维放置 / 2D placement
// ============================================================================

/// 二维放置 / 2D placement
#[derive(Debug, Clone)]
pub struct Placement2<V, U: UnitTrait> {
    /// 装载形状 / Packing shape
    pub shape: PackingShape2<V, U>,
    /// 左下角坐标 / Lower-left position
    pub position: QuantityPoint2<V, U>,
    /// 是否旋转 90 度 / Whether rotated by 90 degrees
    pub rotated: bool,
    /// 物品标识 / Item identifier
    pub item_id: Option<String>,
}

impl<V, U> Placement2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建放置 / Create a placement
    pub fn new(shape: PackingShape2<V, U>, position: QuantityPoint2<V, U>) -> Self {
        Self {
            shape,
            position,
            rotated: false,
            item_id: None,
        }
    }

    /// 设置旋转标记 / Set the rotation flag
    pub fn with_rotated(mut self, rotated: bool) -> Self {
        self.rotated = rotated;
        self
    }

    /// 设置物品标识 / Set the item identifier
    pub fn with_item_id(mut self, item_id: impl Into<String>) -> Self {
        self.item_id = Some(item_id.into());
        self
    }

    /// 按既有物品标识设置标识 / Set the item identifier from an existing value
    ///
    /// 供平移、复制等场景直接搬运标识，避免调用方解包 `Option`。
    /// Lets translation and copying move the identifier directly, so callers need not unwrap the `Option`.
    pub fn with_item_id_ref(mut self, item_id: Option<String>) -> Self {
        self.item_id = item_id;
        self
    }

    /// x 坐标 / x coordinate
    pub fn x(&self) -> Quantity<V, U> {
        self.position.x.clone()
    }

    /// y 坐标 / y coordinate
    pub fn y(&self) -> Quantity<V, U> {
        self.position.y.clone()
    }

    /// 包围宽度 / Bounding width
    pub fn width(&self) -> Quantity<V, U> {
        self.shape.bounding_width()
    }

    /// 包围高度 / Bounding height
    pub fn height(&self) -> Quantity<V, U> {
        self.shape.bounding_height()
    }

    /// 实际占用面积 / Actually occupied area
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        self.shape.area()
    }

    /// 右上角 x 坐标 / Top-right x coordinate
    pub fn max_x(&self) -> Ret<Quantity<V, U>> {
        quantity_plus_safe(&self.x(), &self.width())
    }

    /// 右上角 y 坐标 / Top-right y coordinate
    pub fn max_y(&self) -> Ret<Quantity<V, U>> {
        quantity_plus_safe(&self.y(), &self.height())
    }
}
