//! 旋转守卫 / Rotation guard
//!
//! 与 BPP3D 的能力守卫对应：生成旋转候选前必须确认物品允许旋转，否则返回失败而不是默默生成非法候选。
//! 圆形件旋转不改变形状，视为始终允许。
//! Mirrors the BPP3D capability guard: before generating a rotated candidate the item must allow rotation,
//! otherwise the call fails instead of silently producing an illegal candidate. Circle items are unchanged
//! by rotation and are always allowed.

use ospf_rust_base::error::Ret;
use ospf_rust_quantities::unit::concept::UnitTrait;

use crate::domain::item::model::AbstractItem;
use crate::infrastructure::validation_error;

/// 校验物品是否允许进入旋转候选生成 / Validate that an item may enter rotated candidate generation
///
/// 允许时返回 `Ok`，否则返回带物品编号的失败信息。
/// Returns `Ok` when allowed, otherwise a failure carrying the item identifier.
pub fn require_rotation_allowed<V, U, I>(item: &I) -> Ret<()>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
    I: AbstractItem<V, U>,
{
    if item.shape().is_circle() {
        return Ok(());
    }
    if !item.allow_rotate() {
        return Err(validation_error(format!(
            "物品 {} 未允许旋转，不能生成旋转候选 / Item {} does not allow rotation; rotated candidates are rejected",
            item.id(),
            item.id()
        )));
    }
    Ok(())
}
