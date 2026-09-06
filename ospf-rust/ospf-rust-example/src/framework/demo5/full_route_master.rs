//! 小实例全路线 master oracle / Full-route master oracle for small instances.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::solver::{LinearSolver, SolverStatus};
use ospf_rust_core::variable::Binary;
use ospf_rust_framework_network_scheduling::domain::route_generation::{
    PricingGraph, RouteGraphBuilder,
};
use ospf_rust_framework_network_scheduling::domain::vrp::{
    DistanceArcCostCalculator, DistanceAsTravelTimeCalculator, EuclideanDistanceCalculator,
    FixedPlusArcCostPolicy, PricingDuals, PricingPhase, Route, RouteStop, RouteValidator,
    VehicleTypeId, VrptwInstance,
};
use ospf_rust_quantities::Quantity;

const MAX_EXACT_ORACLE_CUSTOMERS: usize = 8;

/// 一次性全路线 master 的求解结果 / Result of the one-shot full-route master.
#[derive(Debug, Clone)]
pub struct FullRouteMasterResult {
    /// 全部可行路线数量 / Number of all feasible routes.
    pub route_count: usize,
    /// 最优目标值 / Optimal objective value.
    pub objective: f64,
    /// 被选中的路线 / Selected routes.
    pub selected_routes: Vec<Route<f64>>,
}

/// 枚举小实例的全部 elementary 可行路线 / Enumerate all elementary feasible routes of a small instance.
///
/// 该 oracle 只用于离线正确性验收，默认限制为八个客户，避免误用于生产规模。
/// This oracle is only for offline correctness gates and is capped at eight customers to avoid
/// accidental use at production scale.
pub fn enumerate_routes(instance: &VrptwInstance<f64>) -> Result<Vec<Route<f64>>, String> {
    if instance.customers.len() > MAX_EXACT_ORACLE_CUSTOMERS {
        return Err(format!(
            "全路线 oracle 仅支持最多 {} 个客户 / full-route oracle supports at most {} customers",
            MAX_EXACT_ORACLE_CUSTOMERS, MAX_EXACT_ORACLE_CUSTOMERS
        ));
    }

    let builder = RouteGraphBuilder::new(
        Arc::new(instance.clone()),
        EuclideanDistanceCalculator,
        DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
        DistanceArcCostCalculator,
        FixedPlusArcCostPolicy,
    );
    let duals = PricingDuals::new(PricingPhase::PhaseTwo, [], []);
    let mut routes = Vec::new();
    let mut signatures = BTreeSet::new();
    for vehicle_type in &instance.vehicle_types {
        let graph = builder
            .build(&vehicle_type.id, &duals, None)
            .map_err(|error| error.to_string())?;
        let start = graph.start_index();
        let start_time = graph
            .nodes
            .get(start)
            .ok_or_else(|| "pricing graph has no start node / 定价图缺少起点".to_owned())?
            .ready_time;
        let timeline = &instance.scheduling_window;
        let zero_load = Quantity::new(0.0, instance.units.load_unit.clone());
        let start_node = graph
            .nodes
            .get(start)
            .ok_or_else(|| "pricing graph start node is invalid / 定价图起点无效".to_owned())?;
        let start_stop = RouteStop {
            node_id: start_node.node_id.clone(),
            customer_id: None,
            arrival: timeline.instant_of(start_time),
            service_start: timeline.instant_of(start_time),
            departure: timeline.instant_of(start_time),
            accumulated_load: zero_load,
        };
        let mut visited = vec![false; instance.customers.len()];
        enumerate_graph_routes(
            &graph,
            instance,
            start,
            start_time,
            0.0,
            0.0,
            0.0,
            &mut visited,
            &mut Vec::new(),
            &mut vec![start_stop],
            &mut routes,
        )?;
        for route in routes
            .iter()
            .filter(|route| route.vehicle_type_id == vehicle_type.id)
        {
            signatures.insert(route.signature());
        }
    }

    let mut unique = BTreeMap::new();
    for route in routes {
        unique.insert(route.signature(), route);
    }
    let routes = unique.into_values().collect::<Vec<_>>();
    if routes.is_empty() && !signatures.is_empty() {
        return Err(
            "全路线 oracle 生成结果不一致 / full-route oracle route bookkeeping is inconsistent"
                .to_owned(),
        );
    }
    Ok(routes)
}

/// 对全部可行路线建立并求解 master MILP / Build and solve a master MILP over all feasible routes.
pub fn solve<S>(instance: &VrptwInstance<f64>, solver: &S) -> Result<FullRouteMasterResult, String>
where
    S: LinearSolver,
{
    let routes = enumerate_routes(instance)?;
    if routes.is_empty() {
        return Err(
            "全路线 oracle 没有可行路线 / full-route oracle found no feasible routes".to_owned(),
        );
    }
    let mut model = MetaModel::<f64>::new("full_route_master_oracle");
    let variables = routes
        .iter()
        .enumerate()
        .map(|(index, _)| {
            model
                .register_auto_variable::<Binary>(&format!("oracle_route_{index}"))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    for customer in &instance.customers {
        let coefficients = routes
            .iter()
            .enumerate()
            .filter(|(_, route)| route.customer_ids().contains(&customer.id))
            .map(|(index, _)| (variables[index], 1.0))
            .collect::<Vec<_>>();
        model
            .add_linear_constraint(
                &coefficients,
                ConstraintRelation::Equal,
                1.0,
                &format!("oracle_coverage_{}", customer.id),
            )
            .map_err(|error| error.to_string())?;
    }
    for vehicle_type in &instance.vehicle_types {
        let coefficients = routes
            .iter()
            .enumerate()
            .filter(|(_, route)| route.vehicle_type_id == vehicle_type.id)
            .map(|(index, _)| (variables[index], 1.0))
            .collect::<Vec<_>>();
        model
            .add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                vehicle_type.amount as f64,
                &format!("oracle_fleet_{}", vehicle_type.id),
            )
            .map_err(|error| error.to_string())?;
    }
    model.set_objective_category(ObjectiveCategory::Minimum);
    model.add_linear_objective(
        &routes
            .iter()
            .enumerate()
            .map(|(index, route)| (variables[index], route.cost.value))
            .collect::<Vec<_>>(),
        "full_route_master_cost",
    );

    let linear_model = model
        .try_to_linear_triad_model()
        .map_err(|error| error.to_string())?;
    let output = solver
        .solve_linear(&linear_model)
        .map_err(|error| error.to_string())?;
    if output.status != SolverStatus::Optimal {
        return Err(format!(
            "全路线 master 未获得最优证书：{:?} / full-route master did not obtain an optimal certificate: {:?}",
            output.status, output.status
        ));
    }
    let objective = output
        .objective_value
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            "全路线 master 缺少有限目标值 / full-route master has no finite objective".to_owned()
        })?;
    let solution = output.solution.as_deref().ok_or_else(|| {
        "全路线 master 缺少解向量 / full-route master has no solution vector".to_owned()
    })?;
    let mut selected_routes = Vec::new();
    for (index, route) in routes.iter().enumerate() {
        let token = model.tokens().get(variables[index]).ok_or_else(|| {
            "全路线 master 变量索引无效 / full-route master variable index is invalid".to_owned()
        })?;
        let value = solution.get(token.solver_index).copied().ok_or_else(|| {
            "全路线 master 解向量长度不足 / full-route master solution is too short".to_owned()
        })?;
        if !value.is_finite() || (value - value.round()).abs() > 1e-6 {
            return Err(
                "全路线 master 解不是整数 / full-route master solution is fractional".to_owned(),
            );
        }
        if value >= 0.5 {
            selected_routes.push(route.clone());
        }
    }
    validate_master_selection(instance, &selected_routes)?;
    let recomputed = selected_routes
        .iter()
        .map(|route| route.cost.value)
        .sum::<f64>();
    if (recomputed - objective).abs() > 1e-6 {
        return Err(format!(
            "全路线 master 目标重算不一致：{} != {} / full-route master objective mismatch: {} != {}",
            objective, recomputed, objective, recomputed
        ));
    }
    Ok(FullRouteMasterResult {
        route_count: routes.len(),
        objective,
        selected_routes,
    })
}

#[allow(clippy::too_many_arguments)]
fn enumerate_graph_routes(
    graph: &PricingGraph,
    instance: &VrptwInstance<f64>,
    current_node: usize,
    current_time: f64,
    current_load: f64,
    total_distance: f64,
    total_cost: f64,
    visited: &mut [bool],
    arc_indices: &mut Vec<usize>,
    stops: &mut Vec<RouteStop<f64>>,
    routes: &mut Vec<Route<f64>>,
) -> Result<(), String> {
    let end_index = graph.end_index();
    let outgoing = graph
        .outgoing
        .get(current_node)
        .ok_or_else(|| "pricing graph outgoing index is invalid / 定价图出弧索引无效".to_owned())?
        .clone();
    for arc_index in outgoing {
        let arc =
            graph.arcs.get(arc_index).cloned().ok_or_else(|| {
                "pricing graph arc index is invalid / 定价图弧索引无效".to_owned()
            })?;
        let node =
            graph.nodes.get(arc.to).cloned().ok_or_else(|| {
                "pricing graph destination is invalid / 定价图终点无效".to_owned()
            })?;
        let customer_index = node.customer_index;
        if customer_index.is_some_and(|index| visited[index]) {
            continue;
        }
        let travel = instance
            .scheduling_window
            .value_of_duration(arc.travel_time);
        let arrival = current_time + travel;
        let service_start = arrival.max(node.ready_time);
        if service_start > node.due_time + instance.tolerances.feasibility {
            continue;
        }
        let next_load = current_load + node.demand;
        if next_load > graph.vehicle_capacity + instance.tolerances.feasibility {
            continue;
        }
        let departure = service_start + node.service_time;
        if let Some(index) = customer_index {
            visited[index] = true;
        }
        arc_indices.push(arc_index);
        stops.push(RouteStop {
            node_id: node.node_id.clone(),
            customer_id: customer_index
                .and_then(|index| instance.customers.get(index))
                .map(|customer| customer.id.clone()),
            arrival: instance.scheduling_window.instant_of(arrival),
            service_start: instance.scheduling_window.instant_of(service_start),
            departure: instance.scheduling_window.instant_of(departure),
            accumulated_load: Quantity::new(next_load, instance.units.load_unit.clone()),
        });
        let next_distance = total_distance + arc.distance;
        let next_cost = total_cost + arc.route_cost;
        if arc.to == end_index {
            if !visited.iter().all(|visited| !*visited) {
                let route = Route::with_arc_ids(
                    graph.vehicle_type_id.clone(),
                    stops.clone(),
                    arc_indices
                        .iter()
                        .filter_map(|index| graph.arcs.get(*index))
                        .map(|arc| arc.arc_id.clone())
                        .collect(),
                    Quantity::new(next_distance, instance.units.distance_unit.clone()),
                    Quantity::new(next_cost, instance.units.cost_unit.clone()),
                )
                .map_err(|error| error.to_string())?;
                RouteValidator::validate(
                    instance,
                    &route,
                    &EuclideanDistanceCalculator,
                    &DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator),
                    &DistanceArcCostCalculator,
                    &FixedPlusArcCostPolicy,
                    None,
                )
                .map_err(|error| error.to_string())?;
                routes.push(route);
            }
        } else {
            enumerate_graph_routes(
                graph,
                instance,
                arc.to,
                departure,
                next_load,
                next_distance,
                next_cost,
                visited,
                arc_indices,
                stops,
                routes,
            )?;
        }
        stops.pop();
        arc_indices.pop();
        if let Some(index) = customer_index {
            visited[index] = false;
        }
    }
    Ok(())
}

fn validate_master_selection(
    instance: &VrptwInstance<f64>,
    routes: &[Route<f64>],
) -> Result<(), String> {
    let mut covered = BTreeSet::new();
    let mut fleet = BTreeMap::<VehicleTypeId, usize>::new();
    for route in routes {
        let vehicle_type = instance
            .vehicle_type(&route.vehicle_type_id)
            .ok_or_else(|| {
                "master selected an unknown vehicle type / master 选择了未知车辆类型".to_owned()
            })?;
        let count = fleet.entry(route.vehicle_type_id.clone()).or_default();
        *count += 1;
        if *count > vehicle_type.amount {
            return Err("master selection exceeds fleet size / master 选择超过车队上限".to_owned());
        }
        for customer in route.customer_ids() {
            if !covered.insert(customer) {
                return Err(
                    "master selection overlaps customers / master 选择路线存在客户重叠".to_owned(),
                );
            }
        }
    }
    let expected = instance
        .customers
        .iter()
        .map(|customer| customer.id.clone())
        .collect::<BTreeSet<_>>();
    if covered != expected {
        return Err(
            "master selection does not cover customers exactly once / master 未精确覆盖客户"
                .to_owned(),
        );
    }
    Ok(())
}
