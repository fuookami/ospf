//! 求解器数值与物理量边界适配 / Solver-value and physical-quantity boundary adapter.

use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};

use crate::error::{NetworkSchedulingError, Result};

/// 将领域值安全转换为 solver `f64` 的适配器 / Adapter converting domain values to solver `f64`.
#[derive(Debug, Clone, Copy)]
pub struct NetworkSchedulingSolverValueAdapter<V> {
    policy: SolveValueConversionPolicy,
    marker: std::marker::PhantomData<fn() -> V>,
}

impl<V> Default for NetworkSchedulingSolverValueAdapter<V> {
    fn default() -> Self {
        Self::new(SolveValueConversionPolicy::AllowRounding)
    }
}

impl<V> NetworkSchedulingSolverValueAdapter<V> {
    /// 创建适配器 / Create an adapter.
    pub fn new(policy: SolveValueConversionPolicy) -> Self {
        Self {
            policy,
            marker: std::marker::PhantomData,
        }
    }

    /// 获取转换策略 / Get conversion policy.
    pub fn policy(&self) -> SolveValueConversionPolicy {
        self.policy
    }
}

impl<V> NetworkSchedulingSolverValueAdapter<V>
where
    V: SolveValue,
{
    /// 将领域值转换为 solver 值 / Convert a domain value to a solver value.
    pub fn to_solver(&self, value: &V) -> Result<f64> {
        let converted = value
            .to_f64_with_policy(self.policy)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if !converted.is_finite() {
            return Err(NetworkSchedulingError::validation("solver 值必须有限 / solver value must be finite"));
        }
        Ok(converted)
    }

    /// 将 solver 值转换回领域值 / Convert a solver value back to a domain value.
    pub fn from_solver(&self, value: f64) -> Result<V> {
        if !value.is_finite() {
            return Err(NetworkSchedulingError::validation("solver 值必须有限 / solver value must be finite"));
        }
        let converted = V::from_f64_with_policy(value, self.policy).map_err(|error| {
            NetworkSchedulingError::Conversion {
                message: error.to_string(),
            }
        })?;
        self.to_solver(&converted)?;
        Ok(converted)
    }
}

impl<V> NetworkSchedulingSolverValueAdapter<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 将运行时单位物理量归一化到目标单位并转换为 solver 值 / Normalize a runtime quantity and convert it to solver value.
    pub fn normalize(&self, quantity: &Quantity<V, Unit>, target: &Unit) -> Result<f64> {
        validate_linear_units([&quantity.unit, target])?;
        let normalized =
            quantity
                .to_unit(target)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
        self.to_solver(&normalized.value)
    }
}

/// 在任何单位乘除之前拒绝仿射单位。 / Reject affine units before any unit multiplication or division.
pub fn validate_linear_units<'a>(units: impl IntoIterator<Item = &'a Unit>) -> Result<()> {
    if units.into_iter().any(|unit| !unit.is_linear()) {
        return Err(NetworkSchedulingError::validation("线性模型不支持仿射单位 / linear models do not support affine units"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_quantities::unit::{CTUnit, Celsius, Kelvin};

    #[derive(Debug, Clone, PartialEq, PartialOrd)]
    struct NonFiniteValue;

    impl SolveValue for NonFiniteValue {
        fn type_name() -> &'static str { "NonFiniteValue" }

        fn from_f64_with_policy(_value: f64, _policy: SolveValueConversionPolicy) -> ospf_rust_core::error::Result<Self> {
            Ok(Self)
        }

        fn to_f64_with_policy(&self, _policy: SolveValueConversionPolicy) -> ospf_rust_core::error::Result<f64> {
            Ok(f64::NAN)
        }
    }

    #[test]
    fn custom_value_conversion_cannot_smuggle_nonfinite_values_in_either_direction() {
        let adapter = NetworkSchedulingSolverValueAdapter::<NonFiniteValue>::default();
        assert!(adapter.to_solver(&NonFiniteValue).is_err());
        assert!(adapter.from_solver(1.0).is_err());
    }

    #[test]
    fn affine_units_are_rejected_before_arithmetic_and_linear_units_work() {
        let adapter = NetworkSchedulingSolverValueAdapter::<f64>::default();
        let celsius = Celsius::INSTANT;
        let kelvin = Kelvin::INSTANT;
        let affine = Quantity::new(10.0, celsius.clone());
        assert!(adapter.normalize(&affine, &kelvin).is_err());
        let linear = Quantity::new(10.0, kelvin.clone());
        assert!(adapter.normalize(&linear, &celsius).is_err());
        assert_eq!(adapter.normalize(&linear, &kelvin).unwrap(), 10.0);
        assert!(adapter.from_solver(f64::NAN).is_err());
        assert!(adapter.to_solver(&f64::INFINITY).is_err());
        assert!(adapter.from_solver(f64::NEG_INFINITY).is_err());
    }
}
