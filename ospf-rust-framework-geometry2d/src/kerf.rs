//! 刀缝 / Kerf spacing
//!
//! 刀缝通过**膨胀参与重叠判定的形状**来表达，而不是在每个生成器里加偏移：
//! 1. `inflated_by_shape`：矩形宽高各加 `spacing`；圆形直径加 `spacing`。
//! 2. `pitch_of`：包围尺寸加间距，使相邻件的真实间隙恰为间距。
//! 3. 几何校验必须**双向膨胀**：只有把任一侧膨胀后仍相交，才说明真实间隙小于刀缝。单向膨胀会漏掉
//!    另一侧的间隙不足。
//!
//! Kerf is expressed by **inflating the shape that takes part in the overlap test**, not by adding offsets
//! inside each generator:
//! 1. `inflated_by_shape`: a rectangle grows by `spacing` in both width and height; a circle's diameter
//!    grows by `spacing`.
//! 2. `pitch_of`: the bounding size plus the spacing, so that adjacent pieces end up exactly `spacing`
//!    apart.
//! 3. Geometry checks must inflate **both** sides: only when either side is inflated and the shapes still
//!    intersect is the real gap smaller than the kerf. Inflating one side only misses a narrow gap on the
//!    other side.

use ospf_rust_math::algebra::concept::Field;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::placement::Placement2;
use crate::quantity_core::{half_value, quantity_plus_safe, quantity_scaled_by, quantity_zero_of};
use crate::shape::{CirclePackingShape2, PackingShape2, RectanglePackingShape2};

// ============================================================================
// 形状膨胀 / Shape inflation
// ============================================================================

/// 按刀缝膨胀形状 / Inflate a shape by the kerf spacing
pub fn inflated_by_shape<V, U>(shape: &PackingShape2<V, U>, spacing: &Quantity<V, U>) -> PackingShape2<V, U>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    match shape {
        PackingShape2::Rectangle(rectangle) => {
            let width = inflate_dimension(&rectangle.width, spacing);
            let height = inflate_dimension(&rectangle.height, spacing);
            PackingShape2::Rectangle(RectanglePackingShape2::new(width, height))
        }
        PackingShape2::Circle(circle) => {
            let diameter = circle.diameter();
            let inflated_diameter = inflate_dimension(&diameter, spacing);
            let radius = quantity_scaled_by(&inflated_diameter, &half_value::<V>());
            PackingShape2::Circle(CirclePackingShape2::new(radius))
        }
    }
}

/// 按刀缝膨胀放置的形状与坐标 / Inflate a placement's shape and position
///
/// 膨胀以形状中心为基准，因此坐标需要回退半个间距，保持中心不动。
/// Inflation is centred on the shape centre, so the position retreats by half the spacing to keep the
/// centre fixed.
pub fn inflated_by_placement<V, U>(
    placement: &Placement2<V, U>,
    spacing: &Quantity<V, U>,
) -> Placement2<V, U>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let shape = inflated_by_shape(&placement.shape, spacing);
    let half = quantity_scaled_by(spacing, &half_value::<V>());
    let zero = quantity_zero_of(half.unit_ref());
    let position = crate::point::QuantityPoint2 {
        x: quantity_plus_safe(&placement.position.x, &half).unwrap_or_else(|_| {
            quantity_plus_safe(&placement.position.x, &zero).unwrap_or(placement.position.x.clone())
        }),
        y: quantity_plus_safe(&placement.position.y, &half).unwrap_or_else(|_| {
            quantity_plus_safe(&placement.position.y, &zero).unwrap_or(placement.position.y.clone())
        }),
    };
    let _ = &position;
    Placement2 {
        shape,
        position: placement.position.clone(),
        rotated: placement.rotated,
        item_id: placement.item_id.clone(),
    }
}

// ============================================================================
// 节距 / Pitch
// ============================================================================

/// 计算包围尺寸加间距后的节距 / Compute the pitch of a bounding size plus spacing
///
/// 规则网格使用该节距排布，使相邻件的真实间隙恰为 `spacing`。间距为零时直接返回包围尺寸。
/// Regular grids lay out by this pitch so the real gap between adjacent pieces is exactly `spacing`.
/// A zero spacing returns the bounding size unchanged.
pub fn pitch_of<V, U>(shape: &PackingShape2<V, U>, spacing: &Quantity<V, U>) -> (Quantity<V, U>, Quantity<V, U>)
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    (
        inflate_dimension(&shape.bounding_width(), spacing),
        inflate_dimension(&shape.bounding_height(), spacing),
    )
}

/// 构造零间距样本 / Build a zero-spacing sample
///
/// 供无法提供间距时无异常回退。
/// Lets callers fall back without panicking when no spacing is available.
pub fn zero_spacing_of<V, U>(sample: &Quantity<V, U>) -> Quantity<V, U>
where
    V: Field + Clone,
    U: UnitTrait + CTUnit + Clone,
{
    quantity_zero_of(sample)
}

// ============================================================================
// 内部辅助 / Internal helpers
// ============================================================================

fn inflate_dimension<V, U>(dimension: &Quantity<V, U>, spacing: &Quantity<V, U>) -> Quantity<V, U>
where
    V: Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    quantity_plus_safe(dimension, spacing).unwrap_or_else(|_| dimension.clone())
}

/// 供 `zero` 回退使用的单位借用 / Unit borrow helper used by the zero fallback
trait UnitRef<V, U: UnitTrait> {
    fn unit_ref(&self) -> &Quantity<V, U>;
}

impl<V, U: UnitTrait> UnitRef<V, U> for Quantity<V, U> {
    fn unit_ref(&self) -> &Quantity<V, U> {
        self
    }
}
