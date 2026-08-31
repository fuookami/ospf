//! 容量、流量与节点供需值对象 / Capacity, flow, and node-balance value objects.

use std::cmp::Ordering;
use std::marker::PhantomData;

use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::Unit;
use ospf_rust_quantities::unit::UnitTrait;

use crate::error::{NetworkSchedulingError, Result};

/// 容量上下界 / Capacity lower and upper bounds.
#[derive(Debug, Clone)]
pub struct CapacityBounds<V, U: UnitTrait = Unit> {
    /// 下界 / Lower bound.
    pub lower: Quantity<V, U>,
    /// 上界 / Upper bound.
    pub upper: Quantity<V, U>,
}

impl<V, U> CapacityBounds<V, U>
where
    V: SolveValue,
    U: UnitTrait,
{
    /// 创建并验证 `0 <= lower <= upper` 的容量边界 / Create and validate `0 <= lower <= upper`.
    pub fn try_new(lower: Quantity<V, U>, upper: Quantity<V, U>) -> Result<Self> {
        let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if lower.value.partial_cmp(&zero).is_none()
            || lower.value.partial_cmp(&zero) == Some(Ordering::Less)
            || upper.value.partial_cmp(&zero).is_none()
            || lower.value.partial_cmp(&upper.value) == Some(Ordering::Greater)
        {
            return Err(NetworkSchedulingError::validation(
                "容量必须满足 0 <= lower <= upper / capacity must satisfy 0 <= lower <= upper",
            ));
        }
        Ok(Self { lower, upper })
    }
}

/// 非负流量值 / Non-negative flow value.
#[derive(Debug, Clone)]
pub struct Flow<V, U: UnitTrait = Unit> {
    /// 流量物理量 / Flow quantity.
    pub quantity: Quantity<V, U>,
}

impl<V, U> Flow<V, U>
where
    V: SolveValue,
    U: UnitTrait,
{
    /// 创建并验证非负流量 / Create and validate a non-negative flow.
    pub fn try_new(quantity: Quantity<V, U>) -> Result<Self> {
        let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if quantity
            .value
            .partial_cmp(&zero)
            .is_none_or(|ordering| ordering == Ordering::Less)
        {
            return Err(NetworkSchedulingError::validation(
                "流量不能为负 / flow cannot be negative",
            ));
        }
        Ok(Self { quantity })
    }
}

/// 节点供需平衡 / Node supply-demand balance.
#[derive(Debug, Clone)]
pub struct NodeBalance<V, U: UnitTrait = Unit> {
    /// 平衡量；正值表示供给，负值表示需求 / Balance; positive is supply and negative is demand.
    pub quantity: Quantity<V, U>,
}

impl<V, U> NodeBalance<V, U>
where
    U: UnitTrait,
{
    /// 创建节点平衡值 / Create a node balance.
    pub fn new(quantity: Quantity<V, U>) -> Self {
        Self { quantity }
    }
}

/// 保留单位类型参数的流值别名 / Flow value alias retaining the unit type parameter.
pub type FlowValue<V, U = Unit> = Quantity<V, U>;

/// `(commodity, arc)` 流变量索引 / `(commodity, arc)` flow-variable index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FlowIndex<C, A> {
    /// 商品 ID / Commodity ID.
    pub commodity: C,
    /// 弧 ID / Arc ID.
    pub arc: A,
    marker: PhantomData<fn() -> ()>,
}

impl<C, A> FlowIndex<C, A> {
    /// 创建流变量索引 / Create a flow-variable index.
    pub fn new(commodity: C, arc: A) -> Self {
        Self {
            commodity,
            arc,
            marker: PhantomData,
        }
    }
}
