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
            // ID 空值必须由校验器自己拦截：`CustomerId` 等类型的内部字段可被结构体字面量
            // 直接构造，构造器检查不构成边界保证。
            //
            // Blank IDs must be rejected by the validator itself: the inner field of types
            // such as `CustomerId` can be built through a struct literal, so constructor
            // checks are not a boundary guarantee.
            if customer.id.as_str().trim().is_empty() {
                return Err(NetworkSchedulingError::validation(
                    "客户 ID 不能为空 / customer ID cannot be blank",
                ));
            }
            if customer.node.id.as_str().trim().is_empty() {
                return Err(NetworkSchedulingError::validation(
                    "客户节点 ID 不能为空 / customer node ID cannot be blank",
                ));
            }
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
            if vehicle_type.id.as_str().trim().is_empty() {
                return Err(NetworkSchedulingError::validation(
                    "车辆类型 ID 不能为空 / vehicle-type ID cannot be blank",
                ));
            }
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
        // 需求与容量的交叉校验必须在客户与车辆类型都校验完成后进行。
        // 若某客户的需求超过所有车型的容量，该实例在构造上就必然不可行——没有任何
        // 车辆能服务它。这种脏数据应在此边界被拒绝，而不是拖到路线重放或定价阶段才暴露。
        //
        // The demand/capacity cross-check must run after both customer and vehicle-type
        // validation. A customer whose demand exceeds every vehicle capacity makes the
        // instance infeasible by construction — no vehicle can serve it. Such input belongs
        // rejected at this boundary rather than surfacing later in replay or pricing.
        for customer in &customers {
            let demand = finite_quantity_value(&customer.demand, &units.load_unit, "客户需求")?;
            let mut supported = false;
            for vehicle_type in &vehicle_types {
                let capacity =
                    finite_quantity_value(&vehicle_type.capacity, &units.load_unit, "车辆容量")?;
                if demand <= capacity {
                    supported = true;
                    break;
                }
            }
            if !supported {
                return Err(NetworkSchedulingError::validation(format!(
                    "客户 {} 的需求超过所有车辆容量 / demand of customer {} exceeds every vehicle capacity",
                    customer.id, customer.id
                )));
            }
        }
        let mut arc_ids = HashSet::new();
        for arc in &arcs {
            if arc.id.as_str().trim().is_empty() {
                return Err(NetworkSchedulingError::validation(
                    "基础网络弧 ID 不能为空 / base-network arc ID cannot be blank",
                ));
            }
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
    // 校验器必须自足：`ServiceTimeWindow` 的字段是 pub，任何以结构体字面量构造的窗口都能
    // 绕过 `new` 的 ready<=due 检查。因此这里复核窗口自身的不变量，而不是依赖构造器。
    //
    // The validator must be self-sufficient: `ServiceTimeWindow`'s fields are `pub`, so any
    // window built from a struct literal bypasses the `ready <= due` check in `new`. The
    // window's own invariant is therefore re-checked here rather than trusted from the
    // constructor.
    if window.due_time < window.ready_time {
        return Err(NetworkSchedulingError::validation(
            "服务时间窗的截止时刻早于起始时刻 / service time window due time is before its ready time",
        ));
    }
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

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    #![allow(clippy::borrow_interior_mutable_const)]

    use std::collections::{BTreeMap, BTreeSet};

    use ospf_rust_quantities::unit::{CTUnit, Kilogram, Meter, Second};

    use ospf_rust_framework_gantt_scheduling::infrastructure::{TimeRange, TimeWindow};

    use super::*;
    use crate::domain::vrp::{
        Coordinate, CustomerId, DistanceArcCostCalculator, DistanceAsTravelTimeCalculator,
        EuclideanDistanceCalculator, FixedPlusArcCostPolicy, RouteStop, TravelTime, VehicleTypeId,
        coordinate_node, default_arc_id,
    };
    use crate::infrastructure::{NetworkArcId, NetworkNode};

    // ========================================================================
    // 固定装置 / Fixtures
    // ========================================================================

    fn cost_unit() -> Unit {
        VrptwUnits::default().cost_unit
    }

    fn scheduling_window_fixture() -> TimeWindow<f64> {
        let start = OffsetDateTime::UNIX_EPOCH;
        let end = start + Duration::hours(24);
        TimeWindow::seconds(TimeRange::new(start, end), 0.0, false, 1.0)
    }

    fn service_window_fixture() -> ServiceTimeWindow {
        let window = scheduling_window_fixture().window;
        ServiceTimeWindow::new(window.start, window.end)
            .expect("合法的服务时间窗 / valid service window")
    }

    fn node_fixture(id: &str, x: f64, y: f64) -> VrpNode<f64> {
        coordinate_node(id, x, y, Meter::INSTANT.clone()).expect("合法的二维节点 / valid 2-D node")
    }

    fn axes_fixture(names: &[&str]) -> BTreeMap<String, Quantity<f64, Unit>> {
        names
            .iter()
            .map(|name| ((*name).to_owned(), Quantity::new(0.0, Meter::INSTANT.clone())))
            .collect()
    }

    fn node_with_axes(id: &str, axes: BTreeMap<String, Quantity<f64, Unit>>) -> VrpNode<f64> {
        NetworkNode::new(id, Coordinate { axes })
    }

    fn depot_fixture(id: &str) -> Depot<f64> {
        Depot {
            node: node_fixture(id, 0.0, 0.0),
            time_window: service_window_fixture(),
        }
    }

    /// 跳过构造器校验直接构造客户，用于覆盖校验器的拒绝路径。
    ///
    /// `Customer` 的字段是 `pub`，因此外部代码可以用结构体字面量绕过 `Customer::new`。
    /// 校验器必须能独立拦住这类输入，本辅助函数就是用来复现该情形的。
    ///
    /// Build a customer past its constructor validation to reach the validator's rejection
    /// paths. `Customer`'s fields are `pub`, so outside code can bypass `Customer::new`
    /// through a struct literal; the validator must reject such input on its own, and this
    /// helper reproduces exactly that situation.
    fn raw_customer(id: &str, node_id: &str, demand: f64) -> Customer<f64> {
        Customer {
            id: CustomerId::from(id),
            node: node_fixture(node_id, 1.0, 0.0),
            demand: Quantity::new(demand, Kilogram::INSTANT.clone()),
            time_window: service_window_fixture(),
            service_time: Duration::ZERO,
        }
    }

    /// 跳过构造器校验直接构造车辆类型，理由同 `raw_customer`。
    /// Build a vehicle type past its constructor validation; see `raw_customer`.
    fn raw_vehicle_type(id: &str, capacity: f64, fixed_cost: f64) -> VehicleType<f64> {
        VehicleType {
            id: VehicleTypeId::from(id),
            capacity: Quantity::new(capacity, Kilogram::INSTANT.clone()),
            fixed_cost: Quantity::new(fixed_cost, cost_unit()),
            amount: 1,
        }
    }

    fn customer_fixture(id: &str, node_id: &str, demand: f64) -> Customer<f64> {
        Customer::new(
            CustomerId::from(id),
            node_fixture(node_id, 1.0, 0.0),
            Quantity::new(demand, Kilogram::INSTANT.clone()),
            service_window_fixture(),
            Duration::ZERO,
        )
        .expect("合法的客户 / valid customer")
    }

    fn vehicle_type_fixture(id: &str, capacity: f64, fixed_cost: f64) -> VehicleType<f64> {
        VehicleType::new(
            VehicleTypeId::from(id),
            Quantity::new(capacity, Kilogram::INSTANT.clone()),
            Quantity::new(fixed_cost, cost_unit()),
            1,
        )
        .expect("合法的车辆类型 / valid vehicle type")
    }

    fn arc_fixture(id: &str, from: &str, to: &str, distance: f64) -> VrptwArc<f64> {
        VrptwArc::new(
            NetworkArcId::from(id),
            NetworkNodeId::from(from),
            NetworkNodeId::from(to),
            Quantity::new(distance, Meter::INSTANT.clone()),
            TravelTime::new(Duration::seconds(1)).expect("合法的行驶时间 / valid travel time"),
            Quantity::new(distance, cost_unit()),
        )
        .expect("合法的弧 / valid arc")
    }

    /// 跳过构造器校验直接构造弧，用于覆盖校验器的拒绝路径。
    /// Build an arc past its constructor validation to reach the validator rejection paths.
    fn raw_arc(id: &str, from: &str, to: &str, distance: f64, travel_seconds: i64) -> VrptwArc<f64> {
        VrptwArc {
            id: NetworkArcId::from(id),
            from: NetworkNodeId::from(from),
            to: NetworkNodeId::from(to),
            distance: Quantity::new(distance, Meter::INSTANT.clone()),
            travel_time: TravelTime {
                duration: Duration::seconds(travel_seconds),
            },
            cost: Quantity::new(distance.abs(), cost_unit()),
            feasible: true,
        }
    }

    /// 单客户实例装置；每个字段可通过 `with_*` 单独覆写。
    /// One-customer instance fixture; each field can be overridden through `with_*`.
    struct InstanceFixture {
        name: String,
        start_depot: Depot<f64>,
        end_depot: Depot<f64>,
        customers: Vec<Customer<f64>>,
        vehicle_types: Vec<VehicleType<f64>>,
        scheduling_window: TimeWindow<f64>,
        units: VrptwUnits,
        arcs: Vec<VrptwArc<f64>>,
    }

    impl InstanceFixture {
        fn new() -> Self {
            Self {
                name: "valid-instance".to_owned(),
                start_depot: depot_fixture("start"),
                end_depot: depot_fixture("end"),
                customers: vec![customer_fixture("c1", "c1-node", 1.0)],
                vehicle_types: vec![vehicle_type_fixture("v1", 10.0, 1.0)],
                scheduling_window: scheduling_window_fixture(),
                units: VrptwUnits::default(),
                arcs: Vec::new(),
            }
        }

        fn with_name(mut self, name: impl Into<String>) -> Self {
            self.name = name.into();
            self
        }

        fn with_start_depot(mut self, depot: Depot<f64>) -> Self {
            self.start_depot = depot;
            self
        }

        fn with_end_depot(mut self, depot: Depot<f64>) -> Self {
            self.end_depot = depot;
            self
        }

        fn with_customers(mut self, customers: Vec<Customer<f64>>) -> Self {
            self.customers = customers;
            self
        }

        fn with_vehicle_types(mut self, vehicle_types: Vec<VehicleType<f64>>) -> Self {
            self.vehicle_types = vehicle_types;
            self
        }

        fn with_scheduling_window(mut self, scheduling_window: TimeWindow<f64>) -> Self {
            self.scheduling_window = scheduling_window;
            self
        }

        fn with_units(mut self, units: VrptwUnits) -> Self {
            self.units = units;
            self
        }

        fn with_arcs(mut self, arcs: Vec<VrptwArc<f64>>) -> Self {
            self.arcs = arcs;
            self
        }

        fn build(self) -> Result<VrptwInstance<f64>> {
            VrptwValidator::create_with_arcs(
                self.name,
                self.start_depot,
                self.end_depot,
                self.customers,
                self.vehicle_types,
                self.scheduling_window,
                self.units,
                VrptwTolerances::default(),
                self.arcs,
            )
        }
    }

    fn assert_validation_error(error: &NetworkSchedulingError, fragment: &str) {
        assert!(
            matches!(error, NetworkSchedulingError::Validation { .. }),
            "期望 Validation 错误，实际为 {error} / expected a Validation error, got {error}"
        );
        assert!(
            error.to_string().contains(fragment),
            "错误消息应包含 {fragment}，实际为 {error} / error message should contain {fragment}, got {error}"
        );
    }

    fn assert_structure_error(error: &NetworkSchedulingError, fragment: &str) {
        assert!(
            matches!(error, NetworkSchedulingError::Structure { .. }),
            "期望 Structure 错误，实际为 {error} / expected a Structure error, got {error}"
        );
        assert!(
            error.to_string().contains(fragment),
            "错误消息应包含 {fragment}，实际为 {error} / error message should contain {fragment}, got {error}"
        );
    }

    fn assert_conversion_error(error: &NetworkSchedulingError) {
        assert!(
            matches!(error, NetworkSchedulingError::Conversion { .. }),
            "期望 Conversion 错误，实际为 {error} / expected a Conversion error, got {error}"
        );
    }

    // ========================================================================
    // 合法输入 / Accepted input
    // ========================================================================

    #[test]
    fn valid_instance_is_created_and_revalidated() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");

        assert_eq!(instance.name, "valid-instance");
        assert_eq!(instance.customers.len(), 1);
        assert_eq!(instance.vehicle_types.len(), 1);
        assert_eq!(instance.arcs.len(), 0);
        assert!(instance.arcs.is_empty());
        assert!(instance.customer(&CustomerId::from("c1")).is_some());
        assert!(instance.vehicle_type(&VehicleTypeId::from("v1")).is_some());
        assert!(instance.customer(&CustomerId::from("missing")).is_none());
        assert!(instance.instance_identity() > 0);
        VrptwValidator::validate(&instance).expect("合法实例可重复校验 / the instance revalidates");
    }

    #[test]
    fn create_without_arcs_keeps_an_empty_arc_catalog() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let copied = VrptwValidator::create(
            instance.name.clone(),
            instance.start_depot.clone(),
            instance.end_depot.clone(),
            instance.customers.clone(),
            instance.vehicle_types.clone(),
            instance.scheduling_window.clone(),
            instance.units.clone(),
            instance.tolerances,
        )
        .expect("create 入口合法 / the create entry point succeeds");

        assert!(copied.arcs.is_empty());
        assert_ne!(copied.instance_identity(), instance.instance_identity());
    }

    #[test]
    fn created_instances_receive_distinct_identities_and_clones_keep_them() {
        let first = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let second = InstanceFixture::new().build().expect("合法实例 / valid instance");

        assert_ne!(first.instance_identity(), second.instance_identity());
        assert_eq!(first.clone().instance_identity(), first.instance_identity());
    }

    #[test]
    fn valid_arc_catalog_is_retained() {
        let instance = InstanceFixture::new()
            .with_arcs(vec![
                arc_fixture("a-start-c1", "start", "c1-node", 1.0),
                arc_fixture("a-c1-end", "c1-node", "end", 1.0),
            ])
            .build()
            .expect("合法弧目录 / valid arc catalog");

        assert_eq!(instance.arcs.len(), 2);
        assert!(instance.arc(&NetworkArcId::from("a-start-c1")).is_some());
        VrptwValidator::validate(&instance).expect("合法实例可重复校验 / the instance revalidates");
    }

    // ========================================================================
    // 实例级必填项与仓库校验 / Required fields and depot checks
    // ========================================================================

    #[test]
    fn service_window_with_ready_after_due_is_rejected_by_the_validator() {
        // 回归：字段是 pub，因此可以绕过 `ServiceTimeWindow::new` 造出倒挂窗口。
        // 校验器必须自己复核该不变量，而不是信任构造器。
        //
        // Regression: the fields are `pub`, so an inverted window can be built past
        // `ServiceTimeWindow::new`. The validator must re-check the invariant rather than
        // trusting the constructor.
        let window = scheduling_window_fixture().window;
        let inverted = ServiceTimeWindow {
            ready_time: window.end,
            due_time: window.start,
        };
        let mut start_depot = depot_fixture("start");
        start_depot.time_window = inverted;

        let error = InstanceFixture::new()
            .with_start_depot(start_depot)
            .build()
            .expect_err("倒挂的服务时间窗必须被拒绝 / an inverted service window must be rejected");
        assert_validation_error(&error, "截止时刻早于起始时刻");
    }

    #[test]
    fn customer_service_window_with_ready_after_due_is_rejected_by_the_validator() {
        // 同上，但作用于客户而非仓库，确保每个节点的窗口都被复核。
        // As above but on a customer, ensuring every node's window is re-checked.
        let window = scheduling_window_fixture().window;
        let mut customer = customer_fixture("c1", "c1-node", 1.0);
        customer.time_window = ServiceTimeWindow {
            ready_time: window.end,
            due_time: window.start,
        };

        let error = InstanceFixture::new()
            .with_customers(vec![customer])
            .build()
            .expect_err("客户倒挂时间窗必须被拒绝 / an inverted customer window must be rejected");
        assert_validation_error(&error, "截止时刻早于起始时刻");
    }

    #[test]
    fn blank_customer_id_is_rejected() {
        // 空客户 ID 必须被校验器拦截，不能只依赖 `Customer::new`。
        // A blank customer ID must be rejected by the validator, not only by `Customer::new`.
        let error = InstanceFixture::new()
            .with_customers(vec![raw_customer("", "c1-node", 1.0)])
            .build()
            .expect_err("空客户 ID 必须被拒绝 / a blank customer ID must be rejected");
        assert_validation_error(&error, "客户 ID 不能为空");
    }

    #[test]
    fn whitespace_only_customer_id_is_rejected() {
        // 纯空白 ID 与空 ID 同等对待；否则会产生无法用日志区分的"隐形"标识。
        // A whitespace-only ID is treated like a blank one; otherwise it yields an
        // "invisible" identifier that cannot be told apart in logs.
        let error = InstanceFixture::new()
            .with_customers(vec![raw_customer("   ", "c1-node", 1.0)])
            .build()
            .expect_err("纯空白客户 ID 必须被拒绝 / a whitespace-only customer ID must be rejected");
        assert_validation_error(&error, "客户 ID 不能为空");
    }

    #[test]
    fn blank_customer_node_id_is_rejected() {
        // 客户节点 ID 为空也必须被拒绝 / A blank customer node ID must also be rejected.
        let error = InstanceFixture::new()
            .with_customers(vec![raw_customer("c1", "", 1.0)])
            .build()
            .expect_err("空客户节点 ID 必须被拒绝 / a blank customer node ID must be rejected");
        assert_validation_error(&error, "客户节点 ID 不能为空");
    }

    #[test]
    fn blank_vehicle_type_id_is_rejected() {
        // 空车辆类型 ID 必须被拒绝 / A blank vehicle-type ID must be rejected.
        let error = InstanceFixture::new()
            .with_vehicle_types(vec![raw_vehicle_type("", 10.0, 1.0)])
            .build()
            .expect_err("空车辆类型 ID 必须被拒绝 / a blank vehicle-type ID must be rejected");
        assert_validation_error(&error, "车辆类型 ID 不能为空");
    }

    #[test]
    fn blank_arc_id_is_rejected() {
        // 空弧 ID 必须被拒绝 / A blank base-network arc ID must be rejected.
        let error = InstanceFixture::new()
            .with_arcs(vec![raw_arc("", "start", "c1-node", 1.0, 1)])
            .build()
            .expect_err("空弧 ID 必须被拒绝 / a blank arc ID must be rejected");
        assert_validation_error(&error, "基础网络弧 ID 不能为空");
    }

    #[test]
    fn valid_identifiers_are_still_accepted() {
        // 对照：非空 ID 与合法时间窗不得被新增校验误伤。
        // Control: non-blank IDs and a valid service window must not be caught by the new checks.
        let instance = InstanceFixture::new()
            .build()
            .expect("合法实例必须继续被接受 / a valid instance must still be accepted");

        assert_eq!(instance.customers.len(), 1);
        assert_eq!(instance.vehicle_types.len(), 1);
    }

    #[test]
    fn blank_instance_name_is_rejected() {
        let error = InstanceFixture::new()
            .with_name("   ")
            .build()
            .expect_err("空实例名必须被拒绝 / a blank instance name must be rejected");
        assert_validation_error(&error, "实例名称");
    }

    #[test]
    fn empty_customer_list_is_rejected() {
        let error = InstanceFixture::new()
            .with_customers(Vec::new())
            .build()
            .expect_err("空客户列表必须被拒绝 / an empty customer list must be rejected");
        assert_validation_error(&error, "客户");
    }

    #[test]
    fn empty_vehicle_type_list_is_rejected() {
        let error = InstanceFixture::new()
            .with_vehicle_types(Vec::new())
            .build()
            .expect_err("空车辆类型列表必须被拒绝 / an empty vehicle-type list must be rejected");
        assert_validation_error(&error, "车辆类型");
    }

    #[test]
    fn identical_depot_node_ids_are_rejected() {
        let error = InstanceFixture::new()
            .with_end_depot(depot_fixture("start"))
            .build()
            .expect_err("起止仓库同 ID 必须被拒绝 / identical depot node IDs must be rejected");
        assert_validation_error(&error, "起止仓库必须使用不同节点 ID");
    }

    #[test]
    fn empty_start_depot_node_id_is_rejected() {
        let error = InstanceFixture::new()
            .with_start_depot(depot_fixture(""))
            .build()
            .expect_err("空仓库节点 ID 必须被拒绝 / an empty depot node ID must be rejected");
        assert_validation_error(&error, "仓库节点 ID 不能为空");
    }

    #[test]
    fn empty_end_depot_node_id_is_rejected() {
        let error = InstanceFixture::new()
            .with_end_depot(depot_fixture(""))
            .build()
            .expect_err("空仓库节点 ID 必须被拒绝 / an empty depot node ID must be rejected");
        assert_validation_error(&error, "仓库节点 ID 不能为空");
    }

    #[test]
    fn empty_business_scheduling_window_is_rejected() {
        let start = OffsetDateTime::UNIX_EPOCH;
        let empty_window: TimeWindow<f64> =
            TimeWindow::seconds(TimeRange::new(start, start), 0.0, false, 1.0);

        let error = InstanceFixture::new()
            .with_scheduling_window(empty_window)
            .build()
            .expect_err("空业务时间轴必须被拒绝 / an empty scheduling window must be rejected");
        assert_validation_error(&error, "业务时间轴不能为空");
    }

    // ========================================================================
    // 唯一性校验 / Uniqueness checks
    // ========================================================================

    #[test]
    fn duplicate_customer_id_is_rejected() {
        let original = customer_fixture("c1", "c1-node", 1.0);
        let duplicate = customer_fixture("c1", "c2-node", 1.0);

        let error = InstanceFixture::new()
            .with_customers(vec![original, duplicate])
            .build()
            .expect_err("重复客户 ID 必须被拒绝 / a duplicate customer ID must be rejected");
        assert_validation_error(&error, "客户 ID 重复");
    }

    #[test]
    fn duplicate_network_node_id_between_customers_is_rejected() {
        let first = customer_fixture("c1", "shared-node", 1.0);
        let second = customer_fixture("c2", "shared-node", 1.0);

        let error = InstanceFixture::new()
            .with_customers(vec![first, second])
            .build()
            .expect_err("重复网络节点 ID 必须被拒绝 / a duplicate network node ID must be rejected");
        assert_validation_error(&error, "网络节点 ID 重复");
    }

    #[test]
    fn customer_reusing_a_depot_node_id_is_rejected() {
        let customer = customer_fixture("c1", "start", 1.0);

        let error = InstanceFixture::new()
            .with_customers(vec![customer])
            .build()
            .expect_err("客户复用仓库节点 ID 必须被拒绝 / a customer reusing a depot node ID must be rejected");
        assert_validation_error(&error, "网络节点 ID 重复");
    }

    #[test]
    fn duplicate_vehicle_type_id_is_rejected() {
        let error = InstanceFixture::new()
            .with_vehicle_types(vec![
                vehicle_type_fixture("v1", 10.0, 1.0),
                vehicle_type_fixture("v1", 20.0, 2.0),
            ])
            .build()
            .expect_err("重复车辆类型 ID 必须被拒绝 / a duplicate vehicle-type ID must be rejected");
        assert_validation_error(&error, "车辆类型 ID 重复");
    }

    // ========================================================================
    // 坐标轴与节点单位校验 / Coordinate axes and node units
    // ========================================================================

    #[test]
    fn depot_with_mismatched_coordinate_axes_is_rejected() {
        let mismatched = Depot {
            node: node_with_axes("end", axes_fixture(&["x", "z"])),
            time_window: service_window_fixture(),
        };

        let error = InstanceFixture::new()
            .with_end_depot(mismatched)
            .build()
            .expect_err("起止仓库坐标轴不一致必须被拒绝 / mismatched depot axes must be rejected");
        assert_validation_error(&error, "起止仓库必须具有相同坐标轴");
    }

    #[test]
    fn customer_with_mismatched_coordinate_axes_is_rejected() {
        let mut customer = customer_fixture("c1", "c1-node", 1.0);
        customer.node = node_with_axes("c1-node", axes_fixture(&["x", "z"]));

        let error = InstanceFixture::new()
            .with_customers(vec![customer])
            .build()
            .expect_err("客户坐标轴不一致必须被拒绝 / mismatched customer axes must be rejected");
        assert_validation_error(&error, "所有节点必须具有相同坐标轴");
    }

    #[test]
    fn depot_without_coordinate_axes_is_rejected() {
        let empty_axes = Depot {
            node: node_with_axes("start", BTreeMap::new()),
            time_window: service_window_fixture(),
        };

        let error = InstanceFixture::new()
            .with_start_depot(empty_axes)
            .build()
            .expect_err("空坐标轴必须被拒绝 / empty coordinate axes must be rejected");
        assert_validation_error(&error, "所有节点必须有非空坐标轴");
    }

    #[test]
    fn non_finite_node_coordinate_is_rejected() {
        let invalid = Depot {
            node: node_fixture("start", f64::NAN, 0.0),
            time_window: service_window_fixture(),
        };

        let error = InstanceFixture::new()
            .with_start_depot(invalid)
            .build()
            .expect_err("非有限坐标必须被拒绝 / a non-finite coordinate must be rejected");
        assert_validation_error(&error, "节点坐标必须是有限值");
    }

    #[test]
    fn node_coordinate_unit_mismatch_is_rejected() {
        let invalid = Depot {
            node: coordinate_node("start", 0.0, 0.0, Second::INSTANT.clone())
                .expect("时间轴节点构造成功 / the time-unit node is constructible"),
            time_window: service_window_fixture(),
        };

        let error = InstanceFixture::new()
            .with_start_depot(invalid)
            .build()
            .expect_err("坐标单位量纲不匹配必须被拒绝 / a dimensional coordinate mismatch must be rejected");
        assert_conversion_error(&error);
    }

    #[test]
    fn instance_distance_unit_mismatch_is_rejected() {
        let units = VrptwUnits {
            distance_unit: Second::INSTANT.clone(),
            load_unit: Kilogram::INSTANT.clone(),
            cost_unit: cost_unit(),
        };

        let error = InstanceFixture::new()
            .with_units(units)
            .build()
            .expect_err("距离单位口径量纲不匹配必须被拒绝 / a dimensional distance-unit contract must be rejected");
        assert_conversion_error(&error);
    }

    // ========================================================================
    // 客户时间与需求校验 / Customer time and demand checks
    // ========================================================================

    #[test]
    fn negative_customer_demand_is_rejected() {
        let mut customer = customer_fixture("c1", "c1-node", 1.0);
        customer.demand = Quantity::new(-1.0, Kilogram::INSTANT.clone());

        let error = InstanceFixture::new()
            .with_customers(vec![customer])
            .build()
            .expect_err("负需求必须被拒绝 / a negative demand must be rejected");
        assert_validation_error(&error, "客户需求不能为负");
    }

    #[test]
    fn non_finite_customer_demand_is_rejected() {
        let mut customer = customer_fixture("c1", "c1-node", 1.0);
        customer.demand = Quantity::new(f64::INFINITY, Kilogram::INSTANT.clone());

        let error = InstanceFixture::new()
            .with_customers(vec![customer])
            .build()
            .expect_err("非有限需求必须被拒绝 / a non-finite demand must be rejected");
        assert_conversion_error(&error);
    }

    #[test]
    fn customer_demand_unit_mismatch_is_rejected() {
        let mut customer = customer_fixture("c1", "c1-node", 1.0);
        customer.demand = Quantity::new(1.0, Second::INSTANT.clone());

        let error = InstanceFixture::new()
            .with_customers(vec![customer])
            .build()
            .expect_err("需求单位量纲不匹配必须被拒绝 / a dimensional demand mismatch must be rejected");
        assert_conversion_error(&error);
    }

    #[test]
    fn negative_customer_service_time_is_rejected() {
        let mut customer = customer_fixture("c1", "c1-node", 1.0);
        customer.service_time = Duration::seconds(-1);

        let error = InstanceFixture::new()
            .with_customers(vec![customer])
            .build()
            .expect_err("负服务时长必须被拒绝 / a negative service duration must be rejected");
        assert_validation_error(&error, "服务时长不能为负");
    }

    #[test]
    fn service_time_window_with_ready_time_after_due_time_is_rejected() {
        let window = scheduling_window_fixture().window;
        let result = ServiceTimeWindow::new(window.start + Duration::seconds(10), window.start);

        let error = result.expect_err("ready > due 必须被拒绝 / ready greater than due must be rejected");
        assert_validation_error(&error, "截止时刻早于起始时刻");
    }

    #[test]
    fn customer_service_window_outside_business_timeline_is_rejected() {
        let window = scheduling_window_fixture().window;
        let outside = ServiceTimeWindow::new(window.start, window.end + Duration::seconds(1))
            .expect("闭区间本身合法 / the closed window itself is valid");
        let mut customer = customer_fixture("c1", "c1-node", 1.0);
        customer.time_window = outside;

        let error = InstanceFixture::new()
            .with_customers(vec![customer])
            .build()
            .expect_err("越出业务时间轴必须被拒绝 / a window outside the timeline must be rejected");
        assert_validation_error(&error, "服务时间窗超出业务时间轴");
    }

    #[test]
    fn depot_service_window_outside_business_timeline_is_rejected() {
        let window = scheduling_window_fixture().window;
        let outside = Depot {
            node: node_fixture("start", 0.0, 0.0),
            time_window: ServiceTimeWindow::new(window.start - Duration::seconds(1), window.end)
                .expect("闭区间本身合法 / the closed window itself is valid"),
        };

        let error = InstanceFixture::new()
            .with_start_depot(outside)
            .build()
            .expect_err("仓库时间窗越出业务时间轴必须被拒绝 / a depot window outside the timeline must be rejected");
        assert_validation_error(&error, "服务时间窗超出业务时间轴");
    }

    // ========================================================================
    // 车辆类型校验 / Vehicle-type checks
    // ========================================================================

    #[test]
    fn non_positive_vehicle_capacity_is_rejected() {
        for capacity in [0.0, -1.0] {
            let invalid = VehicleType {
                id: VehicleTypeId::from("v1"),
                capacity: Quantity::new(capacity, Kilogram::INSTANT.clone()),
                fixed_cost: Quantity::new(1.0, cost_unit()),
                amount: 1,
            };
            let error = InstanceFixture::new()
                .with_vehicle_types(vec![invalid])
                .build()
                .expect_err("非正容量必须被拒绝 / a non-positive capacity must be rejected");
            assert_validation_error(&error, "车辆容量或固定成本无效");
        }
    }

    #[test]
    fn negative_vehicle_fixed_cost_is_rejected() {
        let invalid = VehicleType {
            id: VehicleTypeId::from("v1"),
            capacity: Quantity::new(10.0, Kilogram::INSTANT.clone()),
            fixed_cost: Quantity::new(-1.0, cost_unit()),
            amount: 1,
        };

        let error = InstanceFixture::new()
            .with_vehicle_types(vec![invalid])
            .build()
            .expect_err("负固定成本必须被拒绝 / a negative fixed cost must be rejected");
        assert_validation_error(&error, "车辆容量或固定成本无效");
    }

    #[test]
    fn demand_above_every_vehicle_capacity_is_rejected() {
        // 与 Kotlin `instanceShouldRejectDemandAboveEveryVehicleCapacity` 对齐：
        // 需求超过所有车型容量时实例在构造上必然不可行，必须在创建边界被拒绝，
        // 而不是拖到路线重放或定价阶段才暴露。
        //
        // Mirrors Kotlin `instanceShouldRejectDemandAboveEveryVehicleCapacity`: when a
        // customer's demand exceeds every vehicle capacity the instance is infeasible by
        // construction and must be rejected at the creation boundary rather than
        // surfacing later during replay or pricing.
        let error = InstanceFixture::new()
            .with_customers(vec![customer_fixture("c1", "c1-node", 999.0)])
            .with_vehicle_types(vec![vehicle_type_fixture("v1", 10.0, 1.0)])
            .build()
            .expect_err("需求超容必须被拒绝 / demand above capacity must be rejected");
        assert_validation_error(&error, "超过所有车辆容量");
    }

    #[test]
    fn demand_equal_to_the_largest_capacity_is_accepted() {
        // 边界必须包含等号：需求恰好等于最大容量时可被服务，不得被拒绝。
        // The boundary is inclusive: a demand exactly equal to the largest capacity is
        // servable and must not be rejected.
        let instance = InstanceFixture::new()
            .with_customers(vec![customer_fixture("c1", "c1-node", 10.0)])
            .with_vehicle_types(vec![vehicle_type_fixture("v1", 10.0, 1.0)])
            .build()
            .expect("需求等于容量必须被接受 / demand equal to capacity must be accepted");

        assert_eq!(instance.customers.len(), 1);
    }

    #[test]
    fn one_sufficient_vehicle_type_is_enough_to_accept_the_demand() {
        // 只要存在任一车型可服务该客户，实例就应被接受——校验针对"所有"车型，而非最小容量。
        // A single sufficient vehicle type is enough: the check asks whether *any* type can
        // serve the customer, not whether the smallest one can.
        let instance = InstanceFixture::new()
            .with_customers(vec![customer_fixture("c1", "c1-node", 5.0)])
            .with_vehicle_types(vec![
                vehicle_type_fixture("v-small", 1.0, 1.0),
                vehicle_type_fixture("v-large", 50.0, 1.0),
            ])
            .build()
            .expect("存在足够车型时必须接受 / a sufficient vehicle type must be accepted");

        assert_eq!(instance.vehicle_types.len(), 2);
    }

    #[test]
    fn every_customer_is_checked_against_capacity() {
        // 交叉校验必须遍历全部客户，不能只检查第一个。
        // The cross-check must cover every customer, not just the first.
        let error = InstanceFixture::new()
            .with_customers(vec![
                customer_fixture("c-ok", "c-ok-node", 1.0),
                customer_fixture("c-heavy", "c-heavy-node", 999.0),
            ])
            .with_vehicle_types(vec![vehicle_type_fixture("v1", 10.0, 1.0)])
            .build()
            .expect_err("任一客户超容都必须被拒绝 / any over-capacity customer must be rejected");
        assert_validation_error(&error, "c-heavy");
    }

    #[test]
    fn vehicle_capacity_unit_mismatch_is_rejected() {
        let invalid = VehicleType {
            id: VehicleTypeId::from("v1"),
            capacity: Quantity::new(10.0, Second::INSTANT.clone()),
            fixed_cost: Quantity::new(1.0, cost_unit()),
            amount: 1,
        };

        let error = InstanceFixture::new()
            .with_vehicle_types(vec![invalid])
            .build()
            .expect_err("容量单位量纲不匹配必须被拒绝 / a dimensional capacity mismatch must be rejected");
        assert_conversion_error(&error);
    }

    // ========================================================================
    // 基础网络弧校验 / Base-network arc checks
    // ========================================================================

    #[test]
    fn duplicate_base_arc_id_is_rejected() {
        let error = InstanceFixture::new()
            .with_arcs(vec![
                arc_fixture("a1", "start", "c1-node", 1.0),
                arc_fixture("a1", "c1-node", "end", 1.0),
            ])
            .build()
            .expect_err("重复弧 ID 必须被拒绝 / a duplicate arc ID must be rejected");
        assert_validation_error(&error, "基础网络弧 ID 重复");
    }

    #[test]
    fn base_arc_with_missing_endpoint_is_rejected() {
        let error = InstanceFixture::new()
            .with_arcs(vec![arc_fixture("a1", "start", "ghost", 1.0)])
            .build()
            .expect_err("弧端点不存在必须被拒绝 / a missing arc endpoint must be rejected");
        assert_structure_error(&error, "的端点不存在");
    }

    #[test]
    fn base_arc_self_loop_is_rejected() {
        let error = InstanceFixture::new()
            .with_arcs(vec![raw_arc("a1", "start", "start", 1.0, 1)])
            .build()
            .expect_err("自环必须被拒绝 / a self-loop must be rejected");
        assert_validation_error(&error, "基础网络弧不允许自环");
    }

    #[test]
    fn negative_base_arc_distance_is_rejected() {
        let error = InstanceFixture::new()
            .with_arcs(vec![raw_arc("a1", "start", "c1-node", -1.0, 1)])
            .build()
            .expect_err("负弧距离必须被拒绝 / a negative arc distance must be rejected");
        assert_validation_error(&error, "基础网络弧距离不能为负");
    }

    #[test]
    fn negative_base_arc_travel_time_is_rejected() {
        let error = InstanceFixture::new()
            .with_arcs(vec![raw_arc("a1", "start", "c1-node", 1.0, -1)])
            .build()
            .expect_err("负行驶时间必须被拒绝 / a negative travel time must be rejected");
        assert_validation_error(&error, "行驶时间不能为负");
    }

    #[test]
    fn base_arc_distance_unit_mismatch_is_rejected() {
        let mut arc = arc_fixture("a1", "start", "c1-node", 1.0);
        arc.distance = Quantity::new(1.0, Second::INSTANT.clone());

        let error = InstanceFixture::new()
            .with_arcs(vec![arc])
            .build()
            .expect_err("弧距离单位量纲不匹配必须被拒绝 / a dimensional arc-distance mismatch must be rejected");
        assert_conversion_error(&error);
    }

    #[test]
    fn base_arc_cost_unit_mismatch_is_rejected() {
        let mut arc = arc_fixture("a1", "start", "c1-node", 1.0);
        arc.cost = Quantity::new(1.0, Second::INSTANT.clone());

        let error = InstanceFixture::new()
            .with_arcs(vec![arc])
            .build()
            .expect_err("弧成本单位量纲不匹配必须被拒绝 / a dimensional arc-cost mismatch must be rejected");
        assert_conversion_error(&error);
    }

    // ========================================================================
    // 重复校验入口 / Revalidation entry point
    // ========================================================================

    #[test]
    fn revalidation_rejects_a_mutated_instance() {
        let mut instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        instance.customers.push(instance.customers[0].clone());

        let error = VrptwValidator::validate(&instance)
            .expect_err("篡改后的实例必须被拒绝 / a mutated instance must be rejected");
        assert_validation_error(&error, "客户 ID 重复");
    }

    // ========================================================================
    // 路线重放固定装置 / Route-replay fixtures
    // ========================================================================

    /// 与单客户实例一致的合法路线：仓库 → 客户 → 仓库，1 米/秒。
    /// A valid route matching the one-customer instance: depot → customer → depot at 1 m/s.
    fn route_fixture(instance: &VrptwInstance<f64>, vehicle_type_id: &str) -> Route<f64> {
        let start = instance.scheduling_window.window.start;
        let instant = |seconds: i64| start + Duration::seconds(seconds);
        let load = |value: f64| Quantity::new(value, instance.units.load_unit.clone());
        let stops = vec![
            RouteStop {
                node_id: instance.start_depot.node.id.clone(),
                customer_id: None,
                arrival: instant(0),
                service_start: instant(0),
                departure: instant(0),
                accumulated_load: load(0.0),
            },
            RouteStop {
                node_id: instance.customers[0].node.id.clone(),
                customer_id: Some(instance.customers[0].id.clone()),
                arrival: instant(1),
                service_start: instant(1),
                departure: instant(1),
                accumulated_load: load(1.0),
            },
            RouteStop {
                node_id: instance.end_depot.node.id.clone(),
                customer_id: None,
                arrival: instant(2),
                service_start: instant(2),
                departure: instant(2),
                accumulated_load: load(1.0),
            },
        ];
        Route::new(
            vehicle_type_id,
            stops,
            Quantity::new(2.0, instance.units.distance_unit.clone()),
            Quantity::new(3.0, instance.units.cost_unit.clone()),
        )
        .expect("合法的路线 / valid route")
    }

    fn validate_route(
        instance: &VrptwInstance<f64>,
        route: &Route<f64>,
        branch_mask: Option<&BranchMask<VehicleTypeId>>,
    ) -> Result<()> {
        RouteValidator::validate(
            instance,
            route,
            &EuclideanDistanceCalculator,
            &DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
            &DistanceArcCostCalculator,
            &FixedPlusArcCostPolicy,
            branch_mask,
        )
    }

    // ========================================================================
    // 路线重放校验 / Route-replay validation
    // ========================================================================

    #[test]
    fn valid_route_passes_route_validator() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let route = route_fixture(&instance, "v1");

        validate_route(&instance, &route, None).expect("合法路线必须通过 / a valid route must pass");
    }

    #[test]
    fn route_with_unknown_vehicle_type_is_rejected() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let route = route_fixture(&instance, "ghost");

        let error = validate_route(&instance, &route, None)
            .expect_err("未知车辆类型必须被拒绝 / an unknown vehicle type must be rejected");
        assert_validation_error(&error, "路线车辆类型不存在");
    }

    #[test]
    fn route_without_customer_stop_is_rejected() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let mut route = route_fixture(&instance, "v1");
        route.stops.remove(1);

        let error = validate_route(&instance, &route, None)
            .expect_err("缺少客户停靠点必须被拒绝 / a missing customer stop must be rejected");
        assert_validation_error(&error, "路线至少需要一个客户停靠点");
    }

    #[test]
    fn route_not_starting_at_the_start_depot_is_rejected() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let mut route = route_fixture(&instance, "v1");
        route.stops.swap(0, 1);

        let error = validate_route(&instance, &route, None)
            .expect_err("起终点错误必须被拒绝 / wrong route endpoints must be rejected");
        assert_validation_error(&error, "路线必须从起始仓库到结束仓库");
    }

    #[test]
    fn route_referencing_a_missing_customer_is_rejected() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let mut route = route_fixture(&instance, "v1");
        route.stops[1].customer_id = Some(CustomerId::from("ghost"));

        let error = validate_route(&instance, &route, None)
            .expect_err("未知客户必须被拒绝 / a missing customer must be rejected");
        assert_validation_error(&error, "路线引用不存在客户");
    }

    #[test]
    fn route_repeating_a_customer_is_rejected() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let mut route = route_fixture(&instance, "v1");
        let repeated = route.stops[1].clone();
        route.stops.insert(2, repeated);

        let error = validate_route(&instance, &route, None)
            .expect_err("重复客户必须被拒绝 / a repeated customer must be rejected");
        assert_validation_error(&error, "客户缺失、重复或节点不匹配");
    }

    #[test]
    fn route_with_mismatched_customer_node_is_rejected() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let mut route = route_fixture(&instance, "v1");
        route.stops[1].node_id = NetworkNodeId::from("c1-node-other");

        let error = validate_route(&instance, &route, None)
            .expect_err("客户节点不匹配必须被拒绝 / a mismatched customer node must be rejected");
        assert_validation_error(&error, "客户缺失、重复或节点不匹配");
    }

    #[test]
    fn route_incompatible_with_the_branch_mask_is_rejected() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let route = route_fixture(&instance, "v1");
        let mask = BranchMask::new(
            instance.start_depot.node.id.clone(),
            instance.end_depot.node.id.clone(),
            BTreeMap::from([(
                VehicleTypeId::from("v1"),
                BTreeSet::from([instance.customers[0].node.id.clone()]),
            )]),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .expect("合法的分支遮罩 / valid branch mask");

        let error = validate_route(&instance, &route, Some(&mask))
            .expect_err("分支遮罩禁止的路线必须被拒绝 / a mask-rejected route must be rejected");
        assert_validation_error(&error, "路线不兼容当前分支遮罩");
    }

    #[test]
    fn route_with_inconsistent_arrival_time_is_rejected() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let mut route = route_fixture(&instance, "v1");
        route.stops[1].arrival = instance.scheduling_window.window.start + Duration::seconds(5);

        let error = validate_route(&instance, &route, None)
            .expect_err("到达时刻不递推必须被拒绝 / a broken arrival recurrence must be rejected");
        assert_validation_error(&error, "到达时刻与行驶时间递推不一致");
    }

    #[test]
    fn route_with_load_beyond_capacity_is_rejected() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let mut route = route_fixture(&instance, "v1");
        route.stops[1].accumulated_load = Quantity::new(100.0, instance.units.load_unit.clone());
        route.stops[2].accumulated_load = Quantity::new(100.0, instance.units.load_unit.clone());

        let error = validate_route(&instance, &route, None)
            .expect_err("超出容量必须被拒绝 / a capacity overflow must be rejected");
        assert_validation_error(&error, "累计负载递推或容量约束失败");
    }

    #[test]
    fn route_with_mismatched_total_distance_is_rejected() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let mut route = route_fixture(&instance, "v1");
        route.distance = Quantity::new(99.0, instance.units.distance_unit.clone());

        let error = validate_route(&instance, &route, None)
            .expect_err("总距离不一致必须被拒绝 / a mismatched total distance must be rejected");
        assert_validation_error(&error, "总距离与重算结果不一致");
    }

    #[test]
    fn route_with_mismatched_total_cost_is_rejected() {
        let instance = InstanceFixture::new().build().expect("合法实例 / valid instance");
        let mut route = route_fixture(&instance, "v1");
        route.cost = Quantity::new(99.0, instance.units.cost_unit.clone());

        let error = validate_route(&instance, &route, None)
            .expect_err("总成本不一致必须被拒绝 / a mismatched total cost must be rejected");
        assert_validation_error(&error, "总成本与重算结果不一致");
    }

    fn base_arc_catalog(feasible: bool) -> Vec<VrptwArc<f64>> {
        let start = NetworkNodeId::from("start");
        let customer = NetworkNodeId::from("c1-node");
        let end = NetworkNodeId::from("end");
        vec![
            arc_fixture(
                default_arc_id(&start, &customer).as_str(),
                "start",
                "c1-node",
                1.0,
            )
            .with_feasibility(feasible),
            arc_fixture(default_arc_id(&customer, &end).as_str(), "c1-node", "end", 1.0),
        ]
    }

    #[test]
    fn route_with_explicit_base_arcs_is_accepted() {
        let instance = InstanceFixture::new()
            .with_arcs(base_arc_catalog(true))
            .build()
            .expect("合法弧目录 / valid arc catalog");
        let route = route_fixture(&instance, "v1");

        validate_route(&instance, &route, None)
            .expect("显式弧目录下的合法路线必须通过 / a valid route over explicit arcs must pass");
    }

    #[test]
    fn route_using_an_infeasible_base_arc_is_rejected() {
        let instance = InstanceFixture::new()
            .with_arcs(base_arc_catalog(false))
            .build()
            .expect("合法弧目录 / valid arc catalog");
        let route = route_fixture(&instance, "v1");

        let error = validate_route(&instance, &route, None)
            .expect_err("不可行弧必须被拒绝 / an infeasible base arc must be rejected");
        assert_validation_error(&error, "路线使用了不可行基础网络弧");
    }
}
