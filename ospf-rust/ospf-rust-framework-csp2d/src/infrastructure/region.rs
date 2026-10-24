//! 二维裁切区域 / 2D cutting region
//!
//! 以左下角与宽高表示。刀缝是一条被切掉的窄带：竖刀占据 `[position, position + kerf)`，横刀同理，
//! 因此右侧（或上侧）区域的起点是 `position + kerf` 而不是 `position`。
//! Expressed by its lower-left corner and size. The kerf is a removed narrow strip: a vertical cut occupies
//! `[position, position + kerf)` and likewise for a horizontal cut, so the start of the right (or upper)
//! region is `position + kerf`, not `position`.

use ospf_rust_base::error::Ret;
use ospf_rust_framework_geometry2d::{
    Box2Geometry, quantity_greater_or_equal, quantity_is_positive, quantity_less_or_equal,
    quantity_multiply, quantity_zero_of,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::infrastructure::error::validation_error;

// ============================================================================
// CutOrientation2 - 刀线方向 / Cut line orientation
// ============================================================================

/// 刀线方向 / Cut line orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CutOrientation2 {
    /// 竖刀：刀线平行于 y 轴，把料分成左右两块 / Vertical cut: parallel to the y axis, splitting left and right
    Vertical,
    /// 横刀：刀线平行于 x 轴，把料分成上下两块 / Horizontal cut: parallel to the x axis, splitting bottom and top
    Horizontal,
}

// ============================================================================
// Region2 - 二维裁切区域 / 2D cutting region
// ============================================================================

/// 二维裁切区域 / 2D cutting region
#[derive(Debug, Clone)]
pub struct Region2<V, U: UnitTrait> {
    /// 左下角 x 坐标 / Lower-left x coordinate
    pub x: Quantity<V, U>,
    /// 左下角 y 坐标 / Lower-left y coordinate
    pub y: Quantity<V, U>,
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
}

impl<V, U> Region2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建区域 / Create a region
    pub fn new(
        x: Quantity<V, U>,
        y: Quantity<V, U>,
        width: Quantity<V, U>,
        height: Quantity<V, U>,
    ) -> Self {
        Self { x, y, width, height }
    }

    /// 面积 / Area
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        quantity_multiply(&self.width, &self.height)
    }

    /// 右边界 x 坐标 / Right edge x coordinate
    pub fn max_x(&self) -> Quantity<V, U> {
        add(&self.x, &self.width)
    }

    /// 上边界 y 坐标 / Top edge y coordinate
    pub fn max_y(&self) -> Quantity<V, U> {
        add(&self.y, &self.height)
    }

    /// 是否退化（宽或高非正）/ Whether the region is degenerate
    pub fn is_degenerate(&self) -> bool {
        !quantity_is_positive(&self.width) || !quantity_is_positive(&self.height)
    }

    /// 是否完全包含另一个区域 / Whether this region fully contains another region
    pub fn contains(&self, rhs: &Self) -> bool {
        quantity_less_or_equal(&self.x, &rhs.x)
            && quantity_less_or_equal(&self.y, &rhs.y)
            && quantity_greater_or_equal(&self.max_x(), &rhs.max_x())
            && quantity_greater_or_equal(&self.max_y(), &rhs.max_y())
    }

    /// 转换为几何包围盒，复用共享几何模块的重叠判定 / Convert into a geometric box to reuse shared overlap
    /// tests
    pub fn to_box(&self) -> Box2Geometry<V, U> {
        Box2Geometry::of_rectangle(
            self.x.clone(),
            self.y.clone(),
            self.width.clone(),
            self.height.clone(),
        )
    }

    /// 按刀线把区域切成两块并扣除刀缝 / Split the region into two by a cut line, removing the kerf
    ///
    /// 返回顺序为「左/下」与「右/上」。任一侧退化即返回错误，不产出零面积区域。
    /// Returns left/bottom then right/top. A degenerate side is an error rather than a zero-area region.
    pub fn cut(
        &self,
        orientation: CutOrientation2,
        position: &Quantity<V, U>,
        kerf: &Quantity<V, U>,
    ) -> Ret<(Self, Self)> {
        let effective_kerf = if quantity_is_positive(kerf) {
            kerf.clone()
        } else {
            quantity_zero_of(&self.width)
        };
        match orientation {
            CutOrientation2::Vertical => {
                let lower_start = self.x.clone();
                let lower_end = position.clone();
                let upper_start = add(position, &effective_kerf);
                let upper_end = self.max_x();
                if !is_strictly_greater(&lower_end, &lower_start)
                    || !is_strictly_greater(&upper_end, &upper_start)
                {
                    return Err(validation_error(format!(
                        "竖刀位置非法：position={:?} 无法把区域切成两块非退化部分 / Invalid vertical cut position {}",
                        position.value,
                        position.value.to_f64().unwrap_or(f64::NAN)
                    )));
                }
                Ok((
                    Self::new(
                        lower_start.clone(),
                        self.y.clone(),
                        subtract(&lower_end, &lower_start),
                        self.height.clone(),
                    ),
                    Self::new(
                        upper_start.clone(),
                        self.y.clone(),
                        subtract(&upper_end, &upper_start),
                        self.height.clone(),
                    ),
                ))
            }
            CutOrientation2::Horizontal => {
                let lower_start = self.y.clone();
                let lower_end = position.clone();
                let upper_start = add(position, &effective_kerf);
                let upper_end = self.max_y();
                if !is_strictly_greater(&lower_end, &lower_start)
                    || !is_strictly_greater(&upper_end, &upper_start)
                {
                    return Err(validation_error(format!(
                        "横刀位置非法：position={:?} 无法把区域切成两块非退化部分 / Invalid horizontal cut position {}",
                        position.value,
                        position.value.to_f64().unwrap_or(f64::NAN)
                    )));
                }
                Ok((
                    Self::new(
                        self.x.clone(),
                        lower_start.clone(),
                        self.width.clone(),
                        subtract(&lower_end, &lower_start),
                    ),
                    Self::new(
                        self.x.clone(),
                        upper_start.clone(),
                        self.width.clone(),
                        subtract(&upper_end, &upper_start),
                    ),
                ))
            }
        }
    }
}

// ============================================================================
// 构造器 / Constructor
// ============================================================================

/// 由坐标与尺寸构造裁切区域 / Build a cutting region from a corner and a size
pub fn region2_of<V, U>(
    x: Quantity<V, U>,
    y: Quantity<V, U>,
    width: Quantity<V, U>,
    height: Quantity<V, U>,
) -> Region2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    Region2::new(x, y, width, height)
}

// ============================================================================
// 同量纲算术 / Same-dimension arithmetic
// ============================================================================

fn add<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> Quantity<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone,
    U: UnitTrait + Clone,
{
    Quantity::new(lhs.value.clone() + rhs.value.clone(), lhs.unit.clone())
}

fn subtract<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> Quantity<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone,
    U: UnitTrait + Clone,
{
    Quantity::new(lhs.value.clone() - rhs.value.clone(), lhs.unit.clone())
}

fn is_strictly_greater<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> bool
where
    V: PartialOrd,
    U: UnitTrait,
{
    lhs.value > rhs.value
}
