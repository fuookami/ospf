//! 物理量归一化保持在原数值域。 / Quantity normalization stays in the native value domain.

use ospf_rust_core::error::{ModelError, Result};
use ospf_rust_core::model::LinearModelValue;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue, UnitTrait};

/// 装箱物理量口径适配；不能隐式混算单位。 / Packing quantity adapter, without implicit mixed-unit arithmetic.
pub trait PackingValueAdapter<V, U: UnitTrait> {
    /// 转换到声明的模型单位，拒绝不支持的转换。 / Normalize to the declared model unit, rejecting unsupported conversions.
    fn quantity(&self, quantity: &Quantity<V, U>, target: &U) -> Result<V>;
}

/// 仅接受同量纲、同比例尺单位，无数值舍入。 / Accept equal dimensions/scales only, without numeric rounding.
#[derive(Debug, Clone, Copy, Default)]
pub struct SameUnitPackingValueAdapter;

impl<V: LinearModelValue, U: UnitTrait> PackingValueAdapter<V, U> for SameUnitPackingValueAdapter {
    fn quantity(&self, quantity: &Quantity<V, U>, target: &U) -> Result<V> {
        if quantity.unit.dimension_symbol() != target.dimension_symbol()
            || quantity.unit.scale_value() != target.scale_value()
            || !quantity.value.is_finite_value()
        {
            return Err(ModelError::InvalidConstraint("incompatible packing units or nonfinite value".into()).into());
        }
        Ok(quantity.value.clone())
    }
}

/// 运行时单位适配，委托 quantities 的显式转换。 / Runtime unit adapter delegating to quantities' explicit conversion.
#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimePackingValueAdapter;

impl<V: LinearModelValue + UnitConversionValue> PackingValueAdapter<V, Unit> for RuntimePackingValueAdapter {
    fn quantity(&self, quantity: &Quantity<V, Unit>, target: &Unit) -> Result<V> {
        let normalized = quantity.to_unit(target).map_err(|error|
            ModelError::InvalidConstraint(format!("packing unit conversion failed: {error}")))?;
        if !normalized.value.is_finite_value() {
            return Err(ModelError::InvalidConstraint("nonfinite normalized packing quantity".into()).into());
        }
        Ok(normalized.value)
    }
}
