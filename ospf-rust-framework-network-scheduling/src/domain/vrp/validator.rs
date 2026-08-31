//! VRPTW 实例与路线独立校验 / Independent VRPTW instance and route validation.

use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use time::OffsetDateTime;

use super::branching::BranchMask;
use super::model::{
    Customer, Depot, Route, ServiceTimeWindow, VehicleType, VehicleTypeId, VrpNode, VrptwArc,
    VrptwInstance, VrptwTolerances, VrptwUnits,
};
use super::policy::{ArcCostCalculator, DistanceCalculator, RouteCostPolicy, TravelTimeCalculator};
use super::policy::{ArcFeasibilityPolicy, DefaultArcFeasibilityPolicy};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::NetworkNodeId;
use ospf_rust_framework_gantt_scheduling::infrastructure::TimeWindow;
use time::Duration;

static NEXT_VRPTW_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

/// VRPTW 实例一致性校验器 / VRPTW instance-consistency validator.
pub struct VrptwValidator;

impl VrptwValidator {
    /// 校验输入并创建不可变实例 / Validate input and create an immutable instance.
    #[allow(clippy::too_many_arguments)]
    pub fn create<V>(
        name: String,
        start_depot: Depot<V>,
        end_depot: Depot<V>,
        customers: Vec<Customer<V>>,
        vehicle_types: Vec<VehicleType<V>>,
        scheduling_window: TimeWindow<V>,
        units: VrptwUnits,
        tolerances: VrptwTolerances,
    ) -> Result<VrptwInstance<V>>
    where
        V: SolveValue + UnitConversionValue,
    {
        Self::create_with_arcs(
            name,
            start_depot,
            end_depot,
            customers,
            vehicle_types,
            scheduling_window,
            units,
            tolerances,
            Vec::new(),
        )
    }

    /// 校验输入、弧目录并创建不可变实例 / Validate input and arc catalog, then create an immutable instance.
    #[allow(clippy::too_many_arguments)]
    pub fn create_with_arcs<V>(
        name: String,
        start_depot: Depot<V>,
        end_depot: Depot<V>,
        customers: Vec<Customer<V>>,
        vehicle_types: Vec<VehicleType<V>>,
        scheduling_window: TimeWindow<V>,
        units: VrptwUnits,
        tolerances: VrptwTolerances,
        arcs: Vec<VrptwArc<V>>,
    ) -> Result<VrptwInstance<V>>
    where
        V: SolveValue + UnitConversionValue,
    {
        if name.trim().is_empty() || customers.is_empty() || vehicle_types.is_empty() {
            return Err(NetworkSchedulingError::validation(
                "实例名称、客户和车辆类型不能为空 / instance name, customers, and vehicle types are required",
            ));
        }
        if start_depot.node.id == end_depot.node.id {
            return Err(NetworkSchedulingError::validation(
                "起止仓库必须使用不同节点 ID / start and end depots must use different node IDs",
            ));
        }
        if start_depot.node.id.as_str().is_empty() || end_depot.node.id.as_str().is_empty() {
            return Err(NetworkSchedulingError::validation(
                "仓库节点 ID 不能为空 / depot node IDs cannot be empty",
            ));
        }
        if scheduling_window.window.is_empty() {
            return Err(NetworkSchedulingError::validation(
                "业务时间轴不能为空 / business scheduling window cannot be empty",
            ));
        }

        let mut customer_ids = HashSet::new();
        let mut vehicle_type_ids = HashSet::new();
        let mut node_ids = HashSet::new();
        node_ids.insert(start_depot.node.id.clone());
        node_ids.insert(end_depot.node.id.clone());
        let axes = start_depot
            .node
            .payload
            .axis_names()
            .cloned()
            .collect::<Vec<_>>();
        if axes.is_empty() {
            return Err(NetworkSchedulingError::validation(
                "所有节点必须有非空坐标轴 / every node must have non-empty coordinate axes",
            ));
        }
        if end_depot
            .node
            .payload
            .axis_names()
            .cloned()
            .collect::<Vec<_>>()
            != axes
        {
            return Err(NetworkSchedulingError::validation(
                "起止仓库必须具有相同坐标轴 / start and end depots must have the same coordinate axes",
            ));
        }
        let window_start = scheduling_window.window.start;
        let window_end = scheduling_window.window.end;
        for node_window in [start_depot.time_window, end_depot.time_window] {
            validate_service_window(node_window, window_start, window_end)?;
        }
        validate_node_units(&start_depot.node, &units.distance_unit)?;
        validate_node_units(&end_depot.node, &units.distance_unit)?;

        for customer in &customers {
            if !customer_ids.insert(customer.id.clone()) {
                return Err(NetworkSchedulingError::validation(format!(
                    "客户 ID 重复：{} / duplicate customer ID: {}",
                    customer.id, customer.id
                )));
            }
            if !node_ids.insert(customer.node.id.clone()) {
                return Err(NetworkSchedulingError::validation(format!(
                    "网络节点 ID 重复：{} / duplicate network node ID: {}",
                    customer.node.id, customer.node.id
                )));
            }
            if customer
                .node
                .payload
                .axis_names()
                .cloned()
                .collect::<Vec<_>>()
                != axes
            {
                return Err(NetworkSchedulingError::validation(
                    "所有节点必须具有相同坐标轴 / all nodes must have the same coordinate axes",
                ));
            }
            validate_node_units(&customer.node, &units.distance_unit)?;
            validate_service_window(customer.time_window, window_start, window_end)?;
            if customer.service_time < Duration::ZERO {
                return Err(NetworkSchedulingError::validation(
                    "服务时长不能为负 / service duration cannot be negative",
                ));
            }
            let demand = finite_quantity_value(&customer.demand, &units.load_unit, "客户需求")?;
            if demand < 0.0 {
                return Err(NetworkSchedulingError::validation(
                    "客户需求不能为负 / customer demand cannot be negative",
                ));
            }
        }
        for vehicle_type in &vehicle_types {
            if !vehicle_type_ids.insert(vehicle_type.id.clone()) {
                return Err(NetworkSchedulingError::validation(format!(
                    "车辆类型 ID 重复：{} / duplicate vehicle-type ID: {}",
                    vehicle_type.id, vehicle_type.id
                )));
            }
            let capacity =
                finite_quantity_value(&vehicle_type.capacity, &units.load_unit, "车辆容量")?;
            let fixed_cost =
                finite_quantity_value(&vehicle_type.fixed_cost, &units.cost_unit, "车辆固定成本")?;
            if capacity <= 0.0 || fixed_cost < 0.0 {
                return Err(NetworkSchedulingError::validation(
                    "车辆容量或固定成本无效 / vehicle capacity or fixed cost is invalid",
                ));
            }
        }
        let mut arc_ids = HashSet::new();
        for arc in &arcs {
            if !arc_ids.insert(arc.id.clone()) {
                return Err(NetworkSchedulingError::validation(format!(
                    "基础网络弧 ID 重复：{} / duplicate base-network arc ID: {}",
                    arc.id, arc.id
                )));
            }
            if !node_ids.contains(&arc.from) || !node_ids.contains(&arc.to) {
                return Err(NetworkSchedulingError::structure(format!(
                    "基础网络弧 {} 的端点不存在 / endpoint of base-network arc {} does not exist",
                    arc.id, arc.id
                )));
            }
            if arc.from == arc.to {
                return Err(NetworkSchedulingError::validation(
                    "基础网络弧不允许自环 / base-network arcs cannot be self-loops",
                ));
            }
            let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            if arc.distance.value < zero {
                return Err(NetworkSchedulingError::validation(
                    "基础网络弧距离不能为负 / base-network arc distance cannot be negative",
                ));
            }
            let distance = finite_quantity_value(&arc.distance, &units.distance_unit, "弧距离")?;
            let _cost = finite_quantity_value(&arc.cost, &units.cost_unit, "弧成本")?;
            if distance < 0.0 {
                return Err(NetworkSchedulingError::validation(
                    "基础网络弧距离不能为负 / base-network arc distance cannot be negative",
                ));
            }
            if arc.travel_time.duration < Duration::ZERO {
                return Err(NetworkSchedulingError::validation(
                    "行驶时间不能为负 / travel time cannot be negative",
                ));
            }
        }

        let customer_by_id = customers
            .iter()
            .enumerate()
            .map(|(index, customer)| (customer.id.clone(), index))
            .collect();
        let vehicle_type_by_id = vehicle_types
            .iter()
            .enumerate()
            .map(|(index, vehicle_type)| (vehicle_type.id.clone(), index))
            .collect();
        Ok(VrptwInstance {
            name,
            start_depot,
            end_depot,
            customers,
            vehicle_types,
            arcs,
            scheduling_window,
            units,
            tolerances,
            customer_by_id,
            vehicle_type_by_id,
            instance_identity: NEXT_VRPTW_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
        })
    }

    /// 重新校验已有实例 / Revalidate an existing instance.
    pub fn validate<V>(instance: &VrptwInstance<V>) -> Result<()>
    where
        V: SolveValue + UnitConversionValue,
    {
        let copy = Self::create_with_arcs(
            instance.name.clone(),
            instance.start_depot.clone(),
            instance.end_depot.clone(),
            instance.customers.clone(),
            instance.vehicle_types.clone(),
            instance.scheduling_window.clone(),
            instance.units.clone(),
            instance.tolerances,
            instance.arcs.clone(),
        )?;
        let _ = copy;
        Ok(())
    }
}

fn validate_service_window(
    window: ServiceTimeWindow,
    start: OffsetDateTime,
    end: OffsetDateTime,
) -> Result<()> {
    if window.ready_time < start || window.due_time > end {
        return Err(NetworkSchedulingError::validation(
            "服务时间窗超出业务时间轴 / service time window is outside the business scheduling window",
        ));
    }
    Ok(())
}

fn validate_node_units<V>(node: &VrpNode<V>, unit: &Unit) -> Result<()>
where
    V: SolveValue + UnitConversionValue,
{
    let values = node.payload.to_unit(unit)?;
    if values.values().any(|value| {
        value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_or(true, |value| !value.is_finite())
    }) {
        return Err(NetworkSchedulingError::validation(
            "节点坐标必须是有限值 / node coordinates must be finite",
        ));
    }
    Ok(())
}

fn finite_quantity_value<V>(quantity: &Quantity<V, Unit>, unit: &Unit, name: &str) -> Result<f64>
where
    V: SolveValue + UnitConversionValue,
{
    let value = quantity
        .to_unit(unit)
        .map_err(|error| NetworkSchedulingError::Conversion {
            message: error.to_string(),
        })?
        .value
        .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
        .map_err(|error| NetworkSchedulingError::Conversion {
            message: error.to_string(),
        })?;
    if !value.is_finite() {
        return Err(NetworkSchedulingError::validation(format!(
            "{}必须是有限值 / {} must be finite",
            name, name
        )));
    }
    Ok(value)
}

/// 路线完整复核策略集合 / Complete route-validation policy bundle.
///
/// 将路线复核所需的计算策略和分支上下文聚合为一个配置对象，避免在公共入口中传递
/// 多个相互关联的策略参数。
/// A bundle of calculation policies and branch context used to replay a route without
/// passing several related policy arguments through the public entry point.
pub struct RouteValidationPolicy<'a, V, D, T, A, C>
where
    V: SolveValue + UnitConversionValue,
{
    /// 距离计算策略 / Distance-calculation policy.
    pub distance_calculator: &'a D,
    /// 行驶时间计算策略 / Travel-time calculation policy.
    pub travel_time_calculator: &'a T,
    /// 弧成本计算策略 / Arc-cost calculation policy.
    pub arc_cost_calculator: &'a A,
    /// 路线成本汇总策略 / Route-cost aggregation policy.
    pub route_cost_policy: &'a C,
    /// 弧可行性策略 / Arc-feasibility policy.
    pub arc_feasibility_policy: &'a dyn ArcFeasibilityPolicy<V>,
    /// 当前分支掩码 / Active branch mask.
    pub branch_mask: Option<&'a BranchMask<VehicleTypeId>>,
}

/// 独立路线资源递推与成本校验器 / Independent route resource-recurrence and cost validator.
pub struct RouteValidator;

impl RouteValidator {
    /// 重放完整路线并检查所有 VRPTW 资源 / Replay a complete route and check all VRPTW resources.
    pub fn validate<V, D, T, A, C>(
        instance: &VrptwInstance<V>,
        route: &Route<V>,
        distance_calculator: &D,
        travel_time_calculator: &T,
        arc_cost_calculator: &A,
        route_cost_policy: &C,
        branch_mask: Option<&BranchMask<VehicleTypeId>>,
    ) -> Result<()>
    where
        V: SolveValue + UnitConversionValue,
        D: DistanceCalculator<V>,
        T: TravelTimeCalculator<V>,
        A: ArcCostCalculator<V>,
        C: RouteCostPolicy<V>,
    {
        Self::validate_with_policy(
            instance,
            route,
            RouteValidationPolicy {
                distance_calculator,
                travel_time_calculator,
                arc_cost_calculator,
                route_cost_policy,
                arc_feasibility_policy: &DefaultArcFeasibilityPolicy,
                branch_mask,
            },
        )
    }

    /// 使用指定弧可行性策略重放完整路线 / Replay a complete route with an arc-feasibility policy.
    pub fn validate_with_policy<V, D, T, A, C>(
        instance: &VrptwInstance<V>,
        route: &Route<V>,
        policy: RouteValidationPolicy<'_, V, D, T, A, C>,
    ) -> Result<()>
    where
        V: SolveValue + UnitConversionValue,
        D: DistanceCalculator<V>,
        T: TravelTimeCalculator<V>,
        A: ArcCostCalculator<V>,
        C: RouteCostPolicy<V>,
    {
        let RouteValidationPolicy {
            distance_calculator,
            travel_time_calculator,
            arc_cost_calculator,
            route_cost_policy,
            arc_feasibility_policy,
            branch_mask,
        } = policy;
        let vehicle_type = instance
            .vehicle_type(&route.vehicle_type_id)
            .ok_or_else(|| {
                NetworkSchedulingError::validation(
                    "路线车辆类型不存在 / route vehicle type does not exist",
                )
            })?;
        instance.validate_route_arc_ids(route)?;
        if route.stops.len() < 3 {
            return Err(NetworkSchedulingError::validation(
                "路线至少需要一个客户停靠点 / route must contain at least one customer stop",
            ));
        }
        let first = route.stops.first().ok_or_else(|| {
            NetworkSchedulingError::structure("路线不能为空 / route cannot be empty")
        })?;
        let last = route.stops.last().ok_or_else(|| {
            NetworkSchedulingError::structure("路线不能为空 / route cannot be empty")
        })?;
        if first.node_id != instance.start_depot.node.id
            || first.customer_id.is_some()
            || last.node_id != instance.end_depot.node.id
            || last.customer_id.is_some()
        {
            return Err(NetworkSchedulingError::validation(
                "路线必须从起始仓库到结束仓库 / route must start at start depot and end at end depot",
            ));
        }
        let customer_stops = &route.stops[1..route.stops.len() - 1];
        let mut seen = HashSet::new();
        for stop in customer_stops {
            let customer_id = stop.customer_id.as_ref().ok_or_else(|| {
                NetworkSchedulingError::validation(
                    "客户停靠点缺少客户 ID / customer stop is missing customer ID",
                )
            })?;
            let customer = instance.customer(customer_id).ok_or_else(|| {
                NetworkSchedulingError::validation(
                    "路线引用不存在客户 / route references a missing customer",
                )
            })?;
            if !seen.insert(customer_id.clone()) || customer.node.id != stop.node_id {
                return Err(NetworkSchedulingError::validation(
                    "客户缺失、重复或节点不匹配 / customer is missing, repeated, or mismatched",
                ));
            }
        }
        if let Some(mask) = branch_mask
            && !mask.is_route_compatible(
                &route.vehicle_type_id,
                &route
                    .stops
                    .iter()
                    .map(|stop| stop.node_id.clone())
                    .collect::<Vec<_>>(),
                &route.effective_arc_ids(),
            )
        {
            return Err(NetworkSchedulingError::validation(
                "路线不兼容当前分支遮罩 / route is incompatible with the branch mask",
            ));
        }
        let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let start_window = instance.start_depot.time_window;
        let first_load = first
            .accumulated_load
            .to_unit(&instance.units.load_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let capacity = vehicle_type
            .capacity
            .to_unit(&instance.units.load_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let capacity_f64 = capacity
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if !start_window.contains(first.service_start)
            || first.arrival > first.service_start
            || first.service_start != first.departure
            || !first_load.is_finite()
            || first_load < -instance.tolerances.feasibility
            || first_load.abs() > instance.tolerances.feasibility
            || !capacity_f64.is_finite()
            || capacity_f64 < 0.0
        {
            return Err(NetworkSchedulingError::validation(
                "起始仓库资源状态无效 / start depot resource state is invalid",
            ));
        }
        let mut total_distance = 0.0;
        let mut arc_costs = Vec::new();
        let arc_ids = route.effective_arc_ids();
        for index in 1..route.stops.len() {
            let previous = &route.stops[index - 1];
            let current = &route.stops[index];
            let from = node_of(instance, &previous.node_id).ok_or_else(|| {
                NetworkSchedulingError::validation(
                    "路线起点节点不存在 / route origin node does not exist",
                )
            })?;
            let to = node_of(instance, &current.node_id).ok_or_else(|| {
                NetworkSchedulingError::validation(
                    "路线终点节点不存在 / route destination node does not exist",
                )
            })?;
            if !arc_feasibility_policy.is_feasible(from, to, vehicle_type)? {
                return Err(NetworkSchedulingError::validation(
                    "路线使用了策略禁止的弧 / route uses an arc rejected by the feasibility policy",
                ));
            }
            let arc_id = arc_ids.get(index - 1).ok_or_else(|| {
                NetworkSchedulingError::contract(
                    "路线弧索引长度不一致 / route arc-index length is inconsistent",
                )
            })?;
            let base_arc = instance.arc_for_route(arc_id, &previous.node_id, &current.node_id);
            if !instance.arcs.is_empty() && base_arc.is_none() {
                return Err(NetworkSchedulingError::validation(
                    "路线引用不存在的基础网络弧 / route references a missing base-network arc",
                ));
            }
            let (travel_time, distance, arc_cost) = if let Some(base_arc) = base_arc {
                if !base_arc.feasible {
                    return Err(NetworkSchedulingError::validation(
                        "路线使用了不可行基础网络弧 / route uses an infeasible base-network arc",
                    ));
                }
                let distance = base_arc
                    .distance
                    .to_unit(&instance.units.distance_unit)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?;
                let arc_cost =
                    base_arc
                        .cost
                        .to_unit(&instance.units.cost_unit)
                        .map_err(|error| NetworkSchedulingError::Conversion {
                            message: error.to_string(),
                        })?;
                (base_arc.travel_time.duration, distance, arc_cost)
            } else {
                let travel_time =
                    travel_time_calculator.travel_time(from, to, vehicle_type, instance)?;
                let distance =
                    distance_calculator.distance(from, to, &instance.units.distance_unit)?;
                let arc_cost = arc_cost_calculator.cost(
                    from,
                    to,
                    &distance,
                    travel_time,
                    vehicle_type,
                    instance,
                )?;
                (travel_time, distance, arc_cost)
            };
            if travel_time < Duration::ZERO {
                return Err(NetworkSchedulingError::validation(
                    "行驶时间不能为负 / travel time cannot be negative",
                ));
            }
            if current.arrival != previous.departure + travel_time {
                return Err(NetworkSchedulingError::validation(
                    "到达时刻与行驶时间递推不一致 / arrival time does not match travel-time recurrence",
                ));
            }
            let window = instance.time_window_of(&current.node_id).ok_or_else(|| {
                NetworkSchedulingError::validation(
                    "路线节点时间窗不存在 / route node time window does not exist",
                )
            })?;
            let expected_service_start = current.arrival.max(window.ready_time);
            if current.service_start != expected_service_start
                || !window.contains(current.service_start)
            {
                return Err(NetworkSchedulingError::validation(
                    "服务开始时间窗或等待状态无效 / service start or waiting state is invalid",
                ));
            }
            if current.departure
                != current.service_start + instance.service_time_of(&current.node_id)
            {
                return Err(NetworkSchedulingError::validation(
                    "离开时刻与服务时长递推不一致 / departure time does not match service-time recurrence",
                ));
            }
            let previous_load = previous
                .accumulated_load
                .to_unit(&instance.units.load_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            let demand = instance
                .demand_of(&current.node_id)
                .map(|value| value.to_unit(&instance.units.load_unit))
                .transpose()
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?
                .unwrap_or_else(|| Quantity::new(zero.clone(), instance.units.load_unit.clone()));
            let demand_f64 = demand
                .value
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            let expected_load = previous_load.value.clone() + demand.value;
            let actual_load = current
                .accumulated_load
                .to_unit(&instance.units.load_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            let expected_load_f64 = expected_load
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            let actual_load_f64 = actual_load
                .value
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            if !actual_load_f64.is_finite()
                || !expected_load_f64.is_finite()
                || !demand_f64.is_finite()
                || demand_f64 < -instance.tolerances.feasibility
                || actual_load_f64 < -instance.tolerances.feasibility
                || (actual_load_f64 - expected_load_f64).abs() > instance.tolerances.feasibility
                || actual_load_f64 > capacity_f64 + instance.tolerances.feasibility
            {
                return Err(NetworkSchedulingError::validation(
                    "累计负载递推或容量约束失败 / accumulated load recurrence or capacity failed",
                ));
            }
            let distance_value = distance
                .value
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            if !distance_value.is_finite() || distance_value < -instance.tolerances.cost_validation
            {
                return Err(NetworkSchedulingError::validation(
                    "弧距离不是有限非负值 / arc distance is not finite and non-negative",
                ));
            }
            let arc_cost_value = arc_cost
                .to_unit(&instance.units.cost_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?
                .value
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            if !arc_cost_value.is_finite() {
                return Err(NetworkSchedulingError::validation(
                    "弧成本不是有限值 / arc cost is not finite",
                ));
            }
            total_distance += distance_value;
            arc_costs.push(arc_cost);
        }
        let route_distance = route
            .distance
            .to_unit(&instance.units.distance_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if !route_distance.is_finite()
            || !total_distance.is_finite()
            || (route_distance - total_distance).abs() > instance.tolerances.cost_validation
        {
            return Err(NetworkSchedulingError::validation(
                "总距离与重算结果不一致 / total distance does not match recomputation",
            ));
        }
        let expected_cost = route_cost_policy.cost(vehicle_type, &arc_costs, instance)?;
        let expected_cost = expected_cost
            .to_unit(&instance.units.cost_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let route_cost = route
            .cost
            .to_unit(&instance.units.cost_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if !expected_cost.is_finite()
            || !route_cost.is_finite()
            || route_cost < -instance.tolerances.cost_validation
            || (route_cost - expected_cost).abs() > instance.tolerances.cost_validation
        {
            return Err(NetworkSchedulingError::validation(
                "总成本与重算结果不一致 / total cost does not match recomputation",
            ));
        }
        Ok(())
    }
}

fn node_of<'a, V>(instance: &'a VrptwInstance<V>, id: &NetworkNodeId) -> Option<&'a VrpNode<V>>
where
    V: SolveValue + UnitConversionValue,
{
    if id == &instance.start_depot.node.id {
        return Some(&instance.start_depot.node);
    }
    if id == &instance.end_depot.node.id {
        return Some(&instance.end_depot.node);
    }
    instance
        .customers
        .iter()
        .find(|customer| &customer.node.id == id)
        .map(|customer| &customer.node)
}
