//! 二维量纲点与向量 / 2D quantity point and vector
//!
//! 坐标分量必须携带长度量纲，运算在量纲不兼容时以错误暴露而不是 panic。
//! Coordinate components must carry a length dimension; operations surface incompatible dimensions as errors
//! instead of panicking.

use std::cmp::Ordering;

use ospf_rust_base::error::Ret;
use ospf_rust_math::algebra::concept::Field;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::quantity_core::{quantity_ord_or_null, quantity_ord_safe};

// ============================================================================
// QuantityVector2 - 二维量纲向量 / 2D quantity vector
// ============================================================================

/// 二维量纲向量 / 2D quantity vector
#[derive(Debug, Clone)]
pub struct QuantityVector2<V, U: UnitTrait> {
    /// X 分量 / X component
    pub x: Quantity<V, U>,
    /// Y 分量 / Y component
    pub y: Quantity<V, U>,
}

impl<V, U> QuantityVector2<V, U>
where
    V: Field + Clone,
    U: UnitTrait + CTUnit + Clone,
{
    /// 向量加法 / Vector addition
    pub fn plus(&self, rhs: &Self) -> Ret<Self> {
        let x = crate::quantity_core::quantity_plus_safe(&self.x, &rhs.x)?;
        let y = crate::quantity_core::quantity_plus_safe(&self.y, &rhs.y)?;
        Ok(Self { x, y })
    }

    /// 向量减法 / Vector subtraction
    pub fn minus(&self, rhs: &Self) -> Ret<Self> {
        let x = crate::quantity_core::quantity_minus_safe(&self.x, &rhs.x)?;
        let y = crate::quantity_core::quantity_minus_safe(&self.y, &rhs.y)?;
        Ok(Self { x, y })
    }
}

// ============================================================================
// QuantityPoint2 - 二维量纲点 / 2D quantity point
// ============================================================================

/// 二维量纲点 / 2D quantity point
#[derive(Debug, Clone)]
pub struct QuantityPoint2<V, U: UnitTrait> {
    /// X 坐标 / X coordinate
    pub x: Quantity<V, U>,
    /// Y 坐标 / Y coordinate
    pub y: Quantity<V, U>,
}

impl<V, U> QuantityPoint2<V, U>
where
    V: Field + Clone,
    U: UnitTrait + CTUnit + Clone,
{
    /// 加上向量偏移 / Add a vector offset
    pub fn plus(&self, offset: &QuantityVector2<V, U>) -> Ret<Self> {
        let x = crate::quantity_core::quantity_plus_safe(&self.x, &offset.x)?;
        let y = crate::quantity_core::quantity_plus_safe(&self.y, &offset.y)?;
        Ok(Self { x, y })
    }

    /// 减去向量偏移 / Subtract a vector offset
    pub fn minus(&self, offset: &QuantityVector2<V, U>) -> Ret<Self> {
        let x = crate::quantity_core::quantity_minus_safe(&self.x, &offset.x)?;
        let y = crate::quantity_core::quantity_minus_safe(&self.y, &offset.y)?;
        Ok(Self { x, y })
    }
}

impl<V, U> QuantityPoint2<V, U>
where
    V: PartialOrd,
    U: UnitTrait,
{
    /// 比较两点排序：先 y 后 x，不可比较时返回 `None` / Compare two points by y then x; `None` if incomparable
    pub fn ord_or_null(&self, rhs: &Self) -> Option<Ordering> {
        match quantity_ord_or_null(&self.y, &rhs.y)? {
            Ordering::Equal => {}
            other => return Some(other),
        }
        quantity_ord_or_null(&self.x, &rhs.x)
    }

    /// 比较两点排序，不可比较时返回错误 / Compare two points, failing when incomparable
    pub fn ord_safe(&self, rhs: &Self, axis: &str) -> Ret<Ordering> {
        quantity_ord_safe(&self.y, &rhs.y, axis)?;
        quantity_ord_safe(&self.x, &rhs.x, axis)
    }
}

// ============================================================================
// 构造与转换 / Construction and conversion
// ============================================================================

/// 构造二维量纲点 / Build a 2D quantity point
pub fn point2_of<V, U>(x: Quantity<V, U>, y: Quantity<V, U>) -> QuantityPoint2<V, U>
where
    U: UnitTrait,
{
    QuantityPoint2 { x, y }
}

/// 构造二维量纲向量 / Build a 2D quantity vector
pub fn vector2_of<V, U>(x: Quantity<V, U>, y: Quantity<V, U>) -> QuantityVector2<V, U>
where
    U: UnitTrait,
{
    QuantityVector2 { x, y }
}
