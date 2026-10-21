//! 块装载模型 / Block loading models
//!
//! 二维空间区域 `Space2` 与块 `Block2`，以及同量纲加减辅助。
//! The 2D space region `Space2`, the block `Block2`, and same-dimension add/subtract helpers.

pub mod space;

pub use space::{
    Block2, Space2, placement_inside_space, space_is_usable,
};

use ospf_rust_base::error::Ret;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;

use crate::infrastructure::validation_error;

/// 同量纲加法 / Same-dimension addition
///
/// 区域坐标与节距推进都走这里，量纲不兼容时返回错误而不是 panic。
/// Region coordinates and pitch advancement both go through here; incompatible dimensions surface as an
/// error rather than panicking.
pub fn add_quantity<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> Quantity<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone,
    U: UnitTrait + Clone,
{
    Quantity::new(lhs.value.clone() + rhs.value.clone(), lhs.unit.clone())
}

/// 同量纲减法（失败可传播）/ Same-dimension subtraction with propagatable failure
pub fn subtract_quantity<V, U>(
    lhs: &Quantity<V, U>,
    rhs: &Quantity<V, U>,
) -> Option<Quantity<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone,
    U: UnitTrait + Clone,
{
    if lhs.unit.dimension_symbol() != rhs.unit.dimension_symbol() {
        return None;
    }
    Some(Quantity::new(
        lhs.value.clone() - rhs.value.clone(),
        lhs.unit.clone(),
    ))
}

/// 校验区域尺寸为正 / Validate that region dimensions are positive
pub fn require_usable_space<V, U>(space: &Space2<V, U>) -> Ret<()>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
{
    if space.is_degenerate() {
        return Err(validation_error(
            "空间区域已退化，宽或高非正 / The space region is degenerate: width or height is non-positive",
        ));
    }
    Ok(())
}
