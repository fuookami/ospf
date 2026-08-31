//! VRPTW 计算策略扩展点 / VRPTW calculation-policy extension points.

use std::collections::BTreeMap;

use time::Duration;

use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};

use super::{VehicleType, VrpNode, VrptwInstance};
use crate::error::{NetworkSchedulingError, Result};

/// 距离计算策略 / Distance-calculation policy.
pub trait DistanceCalculator<V>: Send + Sync
where
    V: SolveValue + UnitConversionValue,
{
    /// 计算两个节点之间的距离 / Calculate distance between two nodes.
    fn distance(
        &self,
        from: &VrpNode<V>,
        to: &VrpNode<V>,
        unit: &Unit,
    ) -> Result<Quantity<V, Unit>>;
}

/// 完整精度欧氏距离策略 / Full-precision Euclidean-distance policy.
#[derive(Debug, Clone)]
pub struct EuclideanDistanceCalculator;

impl EuclideanDistanceCalculator {
    /// 创建欧氏距离策略 / Create an Euclidean-distance policy.
    pub fn new() -> Self {
        Self
    }
}

impl Default for EuclideanDistanceCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> DistanceCalculator<V> for EuclideanDistanceCalculator
where
    V: SolveValue + UnitConversionValue,
{
    fn distance(
        &self,
        from: &VrpNode<V>,
        to: &VrpNode<V>,
        unit: &Unit,
    ) -> Result<Quantity<V, Unit>> {
        let lhs: BTreeMap<String, V> = from.payload.to_unit(unit)?;
        let rhs: BTreeMap<String, V> = to.payload.to_unit(unit)?;
        if lhs.keys().ne(rhs.keys()) {
            return Err(NetworkSchedulingError::validation(
                "坐标轴不一致 / coordinate axes do not match",
            ));
        }
        let squared = lhs
            .iter()
            .map(|(axis, value)| {
                let other = rhs.get(axis).ok_or_else(|| {
                    NetworkSchedulingError::validation(
                        "坐标轴不一致 / coordinate axes do not match",
                    )
                })?;
                let first = value
                    .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?;
                let second = other
                    .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?;
                Ok((first - second).powi(2))
            })
            .try_fold(0.0, |sum, value| value.map(|value| sum + value))?;
        let distance = squared.sqrt();
        let value = V::from_f64_with_policy(distance, SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        Ok(Quantity::new(value, unit.clone()))
    }
}

/// Solomon 一位小数距离策略 / Solomon one-decimal distance policy.
#[derive(Debug, Clone)]
pub struct SolomonDistanceCalculator;

impl<V> DistanceCalculator<V> for SolomonDistanceCalculator
where
    V: SolveValue + UnitConversionValue,
{
    fn distance(
        &self,
        from: &VrpNode<V>,
        to: &VrpNode<V>,
        unit: &Unit,
    ) -> Result<Quantity<V, Unit>> {
        let distance: Quantity<V, Unit> =
            EuclideanDistanceCalculator::new().distance(from, to, unit)?;
        let value = distance
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let rounded = (value * 10.0).round() / 10.0;
        Ok(Quantity::new(
            V::from_f64_with_policy(rounded, SolveValueConversionPolicy::AllowRounding).map_err(
                |error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                },
            )?,
            unit.clone(),
        ))
    }
}

/// 行驶时间计算策略 / Travel-time calculation policy.
pub trait TravelTimeCalculator<V>: Send + Sync
where
    V: SolveValue + UnitConversionValue,
{
    /// 计算弧行驶时间 / Calculate travel time for an arc.
    fn travel_time(
        &self,
        from: &VrpNode<V>,
        to: &VrpNode<V>,
        vehicle_type: &VehicleType<V>,
        instance: &VrptwInstance<V>,
    ) -> Result<Duration>;
}

/// 将距离值一比一映射为时间轴值 / Map distance values one-to-one to timeline duration.
#[derive(Debug, Clone)]
pub struct DistanceAsTravelTimeCalculator<D> {
    /// 距离策略 / Distance policy.
    pub distance_calculator: D,
}

impl<D> DistanceAsTravelTimeCalculator<D> {
    /// 创建距离到时间策略 / Create a distance-to-time policy.
    pub fn new(distance_calculator: D) -> Self {
        Self {
            distance_calculator,
        }
    }
}

impl<V, D> TravelTimeCalculator<V> for DistanceAsTravelTimeCalculator<D>
where
    V: SolveValue + UnitConversionValue,
    D: DistanceCalculator<V>,
{
    fn travel_time(
        &self,
        from: &VrpNode<V>,
        to: &VrpNode<V>,
        _vehicle_type: &VehicleType<V>,
        instance: &VrptwInstance<V>,
    ) -> Result<Duration> {
        let distance =
            self.distance_calculator
                .distance(from, to, &instance.units.distance_unit)?;
        let value = distance
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        Ok(instance.scheduling_window.duration_of(
            V::from_f64_with_policy(value, SolveValueConversionPolicy::AllowRounding).map_err(
                |error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                },
            )?,
        ))
    }
}

/// 弧成本计算策略 / Arc-cost calculation policy.
pub trait ArcCostCalculator<V>: Send + Sync
where
    V: SolveValue + UnitConversionValue,
{
    /// 计算弧成本 / Calculate arc cost.
    fn cost(
        &self,
        from: &VrpNode<V>,
        to: &VrpNode<V>,
        distance: &Quantity<V, Unit>,
        travel_time: Duration,
        vehicle_type: &VehicleType<V>,
        instance: &VrptwInstance<V>,
    ) -> Result<Quantity<V, Unit>>;
}

/// 将距离映射到成本单位 / Map distance into the cost unit.
#[derive(Debug, Clone, Copy, Default)]
pub struct DistanceArcCostCalculator;

impl<V> ArcCostCalculator<V> for DistanceArcCostCalculator
where
    V: SolveValue + UnitConversionValue,
{
    fn cost(
        &self,
        _from: &VrpNode<V>,
        _to: &VrpNode<V>,
        distance: &Quantity<V, Unit>,
        _travel_time: Duration,
        _vehicle_type: &VehicleType<V>,
        instance: &VrptwInstance<V>,
    ) -> Result<Quantity<V, Unit>> {
        let normalized = distance
            .to_unit(&instance.units.distance_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let value = normalized.value;
        let solver_value = value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let cost = V::from_f64_with_policy(solver_value, SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        Ok(Quantity::new(cost, instance.units.cost_unit.clone()))
    }
}

/// 路线成本策略 / Route-cost policy.
pub trait RouteCostPolicy<V>: Send + Sync
where
    V: SolveValue + UnitConversionValue,
{
    /// 汇总固定成本与弧成本 / Aggregate fixed cost and arc costs.
    fn cost(
        &self,
        vehicle_type: &VehicleType<V>,
        arc_costs: &[Quantity<V, Unit>],
        instance: &VrptwInstance<V>,
    ) -> Result<Quantity<V, Unit>>;
}

/// 固定成本加弧成本策略 / Fixed-cost-plus-arc-cost policy.
#[derive(Debug, Clone, Copy, Default)]
pub struct FixedPlusArcCostPolicy;

impl<V> RouteCostPolicy<V> for FixedPlusArcCostPolicy
where
    V: SolveValue + UnitConversionValue,
{
    fn cost(
        &self,
        vehicle_type: &VehicleType<V>,
        arc_costs: &[Quantity<V, Unit>],
        instance: &VrptwInstance<V>,
    ) -> Result<Quantity<V, Unit>> {
        let mut total = vehicle_type
            .fixed_cost
            .to_unit(&instance.units.cost_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value;
        for arc_cost in arc_costs {
            let normalized = arc_cost
                .to_unit(&instance.units.cost_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            total = total + normalized.value;
        }
        Ok(Quantity::new(total, instance.units.cost_unit.clone()))
    }
}

/// Demo17 成本策略别名 / Demo17 cost-policy alias.
pub type Demo17CostPolicy = FixedPlusArcCostPolicy;

/// 静态弧可行性策略 / Static arc-feasibility policy.
pub trait ArcFeasibilityPolicy<V>: Send + Sync
where
    V: SolveValue + UnitConversionValue,
{
    /// 判断弧是否可用 / Check whether an arc is feasible.
    fn is_feasible(
        &self,
        from: &VrpNode<V>,
        to: &VrpNode<V>,
        vehicle_type: &VehicleType<V>,
    ) -> Result<bool>;
}

/// 默认仅禁止自环 / Default policy rejecting only self-loops.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultArcFeasibilityPolicy;

impl<V> ArcFeasibilityPolicy<V> for DefaultArcFeasibilityPolicy
where
    V: SolveValue + UnitConversionValue,
{
    fn is_feasible(
        &self,
        from: &VrpNode<V>,
        to: &VrpNode<V>,
        _vehicle_type: &VehicleType<V>,
    ) -> Result<bool> {
        Ok(from.id != to.id)
    }
}
