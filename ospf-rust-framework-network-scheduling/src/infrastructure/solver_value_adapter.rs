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
        value
            .to_f64_with_policy(self.policy)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })
    }

    /// 将 solver 值转换回领域值 / Convert a solver value back to a domain value.
    pub fn from_solver(&self, value: f64) -> Result<V> {
        V::from_f64_with_policy(value, self.policy).map_err(|error| {
            NetworkSchedulingError::Conversion {
                message: error.to_string(),
            }
        })
    }
}

impl<V> NetworkSchedulingSolverValueAdapter<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 将运行时单位物理量归一化到目标单位并转换为 solver 值 / Normalize a runtime quantity and convert it to solver value.
    pub fn normalize(&self, quantity: &Quantity<V, Unit>, target: &Unit) -> Result<f64> {
        let normalized =
            quantity
                .to_unit(target)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
        self.to_solver(&normalized.value)
    }
}
