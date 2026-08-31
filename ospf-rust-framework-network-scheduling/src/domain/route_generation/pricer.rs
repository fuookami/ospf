//! ESPPRC 精确定价器 / ESPPRC exact pricer.

use std::collections::VecDeque;

use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::UnitConversionValue;

use crate::domain::route_generation::model::{
    EspprcLabel, ForbiddenCustomers, LabelStatistics, PricingRequest, PricingResult,
    TruncationReason, VehiclePricingDiagnostic, VisitedCustomers,
};
use crate::domain::route_generation::policy::{
    DefaultLabelDominancePolicy, DefaultPricingColumnSelector, LabelDominancePolicy,
    PricingColumnSelector,
};
use crate::domain::vrp::{Route, RouteStop, RouteValidator};
use crate::error::{NetworkSchedulingError, Result};

use super::graph::{PricingGraph, RouteGraphBuilder};

/// ESPPRC 标签定价器 / ESPPRC label-based pricing service.
pub struct EspprcPricer<
    V,
    D = crate::domain::vrp::EuclideanDistanceCalculator,
    T = crate::domain::vrp::DistanceAsTravelTimeCalculator<D>,
    A = crate::domain::vrp::DistanceArcCostCalculator,
    C = crate::domain::vrp::FixedPlusArcCostPolicy,
    L = DefaultLabelDominancePolicy,
    S = DefaultPricingColumnSelector,
> where
    V: SolveValue + UnitConversionValue,
{
    /// 分支感知定价图构建器 / Branch-aware pricing-graph builder.
    pub graph_builder: RouteGraphBuilder<V, D, T, A, C>,
    /// 标签支配策略 / Label-dominance policy.
    pub dominance_policy: L,
    /// 负列选择策略 / Negative-column selection policy.
    pub column_selector: S,
}

impl<V, D, T, A, C, L, S> EspprcPricer<V, D, T, A, C, L, S>
where
    V: SolveValue + UnitConversionValue,
    D: crate::domain::vrp::DistanceCalculator<V>,
    T: crate::domain::vrp::TravelTimeCalculator<V>,
    A: crate::domain::vrp::ArcCostCalculator<V>,
    C: crate::domain::vrp::RouteCostPolicy<V>,
    L: LabelDominancePolicy,
    S: PricingColumnSelector<V>,
{
    /// 使用策略创建定价器 / Create a pricer with explicit policies.
    pub fn with_policies(
        graph_builder: RouteGraphBuilder<V, D, T, A, C>,
        dominance_policy: L,
        column_selector: S,
    ) -> Self {
        Self {
            graph_builder,
            dominance_policy,
            column_selector,
        }
    }

    /// 使用当前策略对一张定价图执行精确定价 / Price one graph with the configured policies.
    pub(crate) fn price(
        &self,
        graph: &PricingGraph,
        request: &PricingRequest<V>,
    ) -> Result<PricingResult<V>> {
        if graph.vehicle_type_id != request.vehicle_type_id {
            return Err(NetworkSchedulingError::pricing(format!(
                "定价图车辆类型与请求不一致：{} / pricing graph vehicle type does not match request: {}",
                graph.vehicle_type_id, request.vehicle_type_id
            )));
        }
        if !graph.has_valid_integrity() {
            return Err(NetworkSchedulingError::pricing(
                "定价图公开快照完整性校验失败 / public pricing graph snapshot integrity check failed",
            ));
        }
        if graph.instance_identity != request.instance.instance_identity()
            || graph.instance_fingerprint != request.instance.pricing_fingerprint()
        {
            return Err(NetworkSchedulingError::pricing(
                "定价图实例快照与请求不一致 / pricing graph instance snapshot does not match the request",
            ));
        }
        if graph.branch_mask != request.branch_mask {
            return Err(NetworkSchedulingError::pricing(
                "定价图分支遮罩与请求不一致 / pricing graph branch mask does not match the request",
            ));
        }
        if graph.duals != request.duals {
            return Err(NetworkSchedulingError::pricing(
                "定价图对偶快照与请求不一致 / pricing graph dual snapshot does not match the request",
            ));
        }
        validate_reduced_cost_snapshot(graph, request)?;
        if graph.nodes.is_empty()
            || graph.start_index() >= graph.nodes.len()
            || graph.end_index() >= graph.nodes.len()
        {
            return Err(NetworkSchedulingError::structure(
                "定价图缺少起止仓库 / pricing graph is missing start or end depot",
            ));
        }

        let customer_count: usize = request.instance.customers.len();
        let start_index = graph.start_index();
        let end_index = graph.end_index();
        let start_node = graph.nodes.get(start_index).ok_or_else(|| {
            NetworkSchedulingError::structure("起始节点不存在 / start node does not exist")
        })?;
        let mut labels = Vec::new();
        let mut labels_at_node = vec![Vec::<usize>::new(); graph.nodes.len()];
        let mut dominated = Vec::new();
        let mut queue = VecDeque::new();
        let mut statistics = LabelStatistics::default();
        let mut negative_label_indices = Vec::new();
        let mut min_reduced_cost = f64::INFINITY;
        let mut interrupted = request.interrupted();
        let mut truncated = false;
        let mut truncation_reason = None;

        let root = EspprcLabel {
            reduced_cost: 0.0,
            time: start_node.ready_time,
            load: 0.0,
            current_node: start_index,
            visited: VisitedCustomers::empty(customer_count),
            forbidden: ForbiddenCustomers::empty(customer_count),
            predecessor: None,
            predecessor_arc: None,
        };
        labels.push(root);
        dominated.push(false);
        labels_at_node[start_index].push(0);
        queue.push_back(0);
        statistics.labels_created = 1;

        let max_depth: usize = request.max_depth.unwrap_or(customer_count);
        if max_depth > customer_count {
            return Err(NetworkSchedulingError::validation(
                "定价最大深度不能超过客户数 / pricing max depth cannot exceed customer count",
            ));
        }

        while let Some(label_index) = queue.pop_front() {
            if request.interrupted() {
                interrupted = true;
                break;
            }
            if dominated.get(label_index).copied().unwrap_or(true) {
                continue;
            }
            let label = labels.get(label_index).cloned().ok_or_else(|| {
                NetworkSchedulingError::contract(
                    "前驱标签索引无效 / predecessor label index is invalid",
                )
            })?;
            let outgoing = graph.outgoing.get(label.current_node).ok_or_else(|| {
                NetworkSchedulingError::structure(
                    "定价图出弧索引无效 / pricing graph outgoing index is invalid",
                )
            })?;

            for &arc_index in outgoing {
                if request.interrupted() {
                    interrupted = true;
                    break;
                }
                let arc = graph.arcs.get(arc_index).ok_or_else(|| {
                    NetworkSchedulingError::structure(
                        "定价图弧索引无效 / pricing graph arc index is invalid",
                    )
                })?;
                if arc.from != label.current_node {
                    return Err(NetworkSchedulingError::contract(
                        "出弧索引与弧起点不一致 / outgoing index disagrees with arc origin",
                    ));
                }
                let to_node = graph.nodes.get(arc.to).ok_or_else(|| {
                    NetworkSchedulingError::structure(
                        "定价图终点不存在 / pricing graph destination does not exist",
                    )
                })?;

                // 空路线不是 VRPTW 列，不能让 start->end 标签支配客户路线。
                // An empty route is not a VRPTW column and must not dominate customer routes.
                if arc.to == end_index && label.visited.count() == 0 {
                    continue;
                }

                if let Some(customer_index) = to_node.customer_index {
                    if label.visited.contains(customer_index)
                        || label.forbidden.contains(customer_index)
                    {
                        continue;
                    }
                    if label.visited.count() >= max_depth {
                        truncated = true;
                        truncation_reason.get_or_insert(TruncationReason::MaxDepth);
                        continue;
                    }
                }

                let travel_value: f64 = duration_value(request, arc.travel_time)?;
                let arrival_time: f64 = label.time + travel_value;
                let service_start: f64 = arrival_time.max(to_node.ready_time);
                if service_start > to_node.due_time + request.instance.tolerances.feasibility {
                    continue;
                }
                let new_load: f64 = label.load + to_node.demand;
                if new_load > graph.vehicle_capacity + request.instance.tolerances.feasibility {
                    continue;
                }
                let new_visited = to_node
                    .customer_index
                    .map_or_else(|| label.visited.clone(), |index| label.visited.add(index));
                let new_time = service_start + to_node.service_time;
                let (new_forbidden, unreachable_markings) = update_forbidden_customers(
                    &label.forbidden,
                    graph,
                    arc.to,
                    &new_visited,
                    new_time,
                    new_load,
                    request.instance.tolerances.feasibility,
                    request,
                )?;
                statistics.unreachable_markings += unreachable_markings;
                let new_label = EspprcLabel {
                    reduced_cost: label.reduced_cost + arc.reduced_cost,
                    time: new_time,
                    load: new_load,
                    current_node: arc.to,
                    visited: new_visited,
                    forbidden: new_forbidden,
                    predecessor: Some(label_index),
                    predecessor_arc: Some(arc_index),
                };

                statistics.extensions += 1;
                statistics.dominance_checks += labels_at_node[arc.to].len();
                if labels_at_node[arc.to].iter().any(|candidate_index| {
                    !dominated.get(*candidate_index).copied().unwrap_or(true)
                        && labels.get(*candidate_index).is_some_and(|candidate| {
                            self.dominance_policy.dominates(candidate, &new_label)
                        })
                }) {
                    statistics.dominated_labels += 1;
                    continue;
                }
                let dominated_indices = labels_at_node[arc.to]
                    .iter()
                    .copied()
                    .filter(|candidate_index| {
                        !dominated.get(*candidate_index).copied().unwrap_or(true)
                            && labels.get(*candidate_index).is_some_and(|candidate| {
                                self.dominance_policy.dominates(&new_label, candidate)
                            })
                    })
                    .collect::<Vec<_>>();
                for candidate_index in dominated_indices {
                    if let Some(value) = dominated.get_mut(candidate_index) {
                        *value = true;
                    }
                    statistics.dominated_labels += 1;
                }
                labels_at_node[arc.to].retain(|candidate_index| !dominated[*candidate_index]);

                if request
                    .max_labels
                    .is_some_and(|limit| labels.len() >= limit)
                {
                    truncated = true;
                    truncation_reason.get_or_insert(TruncationReason::MaxLabels);
                    break;
                }
                let new_label_index = labels.len();
                labels.push(new_label);
                dominated.push(false);
                labels_at_node[arc.to].push(new_label_index);
                queue.push_back(new_label_index);
                statistics.labels_created += 1;

                let reduced_cost = labels[new_label_index].reduced_cost;
                if arc.to == end_index && labels[new_label_index].visited.count() > 0 {
                    min_reduced_cost = min_reduced_cost.min(reduced_cost);
                    if reduced_cost < -request.pricing_tolerance {
                        negative_label_indices.push(new_label_index);
                    }
                }
            }
            if interrupted
                || truncated && matches!(truncation_reason, Some(TruncationReason::MaxLabels))
            {
                break;
            }
        }

        let mut routes = Vec::with_capacity(negative_label_indices.len());
        for label_index in negative_label_indices {
            routes.push(self.backtrack_route(graph, &labels, label_index, request)?);
        }
        routes.sort_by_key(Route::signature);
        let selected_routes = self
            .column_selector
            .select(&routes, request.max_columns_per_pricing);
        if selected_routes.len() < routes.len() {
            truncated = true;
            truncation_reason.get_or_insert(TruncationReason::MaxColumns);
        }
        if !min_reduced_cost.is_finite() {
            min_reduced_cost = 0.0;
        }

        let exact_pricing_complete = !(interrupted
            || truncated
                && matches!(
                    truncation_reason,
                    Some(TruncationReason::MaxLabels | TruncationReason::MaxDepth)
                ));
        let diagnostic = VehiclePricingDiagnostic {
            vehicle_type_id: request.vehicle_type_id.clone(),
            min_reduced_cost,
            negative_columns: routes.len(),
            exact_complete: exact_pricing_complete,
            statistics,
        };
        Ok(PricingResult {
            routes: selected_routes,
            min_reduced_cost,
            exact_pricing_complete,
            interrupted,
            truncated,
            truncation_reason,
            statistics,
            diagnostic: crate::domain::route_generation::model::PricingDiagnostic {
                vehicle_types: vec![diagnostic],
            },
        })
    }

    /// 构图并执行定价 / Build the graph and execute pricing.
    pub fn price_request(&self, request: &PricingRequest<V>) -> Result<PricingResult<V>> {
        let graph = self.graph_builder.build(
            &request.vehicle_type_id,
            &request.duals,
            request.branch_mask.as_ref(),
        )?;
        self.price(&graph, request)
    }

    fn backtrack_route(
        &self,
        graph: &PricingGraph,
        labels: &[EspprcLabel],
        end_label_index: usize,
        request: &PricingRequest<V>,
    ) -> Result<Route<V>> {
        let mut label_indices = Vec::new();
        let mut arc_indices = Vec::new();
        let mut current = Some(end_label_index);
        while let Some(label_index) = current {
            let label = labels.get(label_index).ok_or_else(|| {
                NetworkSchedulingError::contract(
                    "回溯标签索引无效 / invalid label index during backtracking",
                )
            })?;
            label_indices.push(label_index);
            if let Some(arc_index) = label.predecessor_arc {
                arc_indices.push(arc_index);
            }
            current = label.predecessor;
        }
        label_indices.reverse();
        arc_indices.reverse();
        if label_indices.len() != arc_indices.len() + 1 {
            return Err(NetworkSchedulingError::contract(
                "前驱链长度不一致 / predecessor chain length is inconsistent",
            ));
        }

        let timeline = &request.instance.scheduling_window;
        let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let mut load = 0.0;
        let mut total_distance = 0.0;
        let mut total_cost = 0.0;
        let mut stops = Vec::with_capacity(label_indices.len());
        for (position, label_index) in label_indices.iter().copied().enumerate() {
            let label = labels.get(label_index).ok_or_else(|| {
                NetworkSchedulingError::contract("路线标签不存在 / route label does not exist")
            })?;
            let node = graph.nodes.get(label.current_node).ok_or_else(|| {
                NetworkSchedulingError::structure("路线节点不存在 / route node does not exist")
            })?;
            let arrival = if position == 0 {
                label.time
            } else {
                let previous = labels.get(label_indices[position - 1]).ok_or_else(|| {
                    NetworkSchedulingError::contract(
                        "路线前驱标签不存在 / route predecessor label does not exist",
                    )
                })?;
                let arc = graph.arcs.get(arc_indices[position - 1]).ok_or_else(|| {
                    NetworkSchedulingError::structure(
                        "路线前驱弧不存在 / route predecessor arc does not exist",
                    )
                })?;
                previous.time + duration_value(request, arc.travel_time)?
            };
            let service_start = arrival.max(node.ready_time);
            if let Some(customer_index) = node.customer_index {
                load += node.demand;
                if let Some(customer) = request.instance.customers.get(customer_index) {
                    let _ = customer;
                } else {
                    return Err(NetworkSchedulingError::structure(
                        "路线客户索引不存在 / route customer index does not exist",
                    ));
                }
            }
            if position > 0 {
                let arc = graph.arcs.get(arc_indices[position - 1]).ok_or_else(|| {
                    NetworkSchedulingError::structure("路线弧不存在 / route arc does not exist")
                })?;
                total_distance += arc.distance;
                total_cost += arc.route_cost;
            }
            let time_value = |value: f64| {
                V::from_f64_with_policy(value, SolveValueConversionPolicy::AllowRounding).map_err(
                    |error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    },
                )
            };
            let accumulated_load =
                Quantity::new(time_value(load)?, request.instance.units.load_unit.clone());
            stops.push(RouteStop {
                node_id: node.node_id.clone(),
                customer_id: node
                    .customer_index
                    .and_then(|index| request.instance.customers.get(index))
                    .map(|customer| customer.id.clone()),
                arrival: timeline.instant_of(time_value(arrival)?),
                service_start: timeline.instant_of(time_value(service_start)?),
                departure: timeline.instant_of(time_value(label.time)?),
                accumulated_load,
            });
        }
        let route = Route::with_arc_ids(
            request.vehicle_type_id.clone(),
            stops,
            arc_indices
                .iter()
                .map(|index| graph.arcs[*index].arc_id.clone())
                .collect(),
            Quantity::new(
                V::from_f64_with_policy(total_distance, SolveValueConversionPolicy::AllowRounding)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?,
                request.instance.units.distance_unit.clone(),
            ),
            Quantity::new(
                V::from_f64_with_policy(total_cost, SolveValueConversionPolicy::AllowRounding)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?,
                request.instance.units.cost_unit.clone(),
            ),
        )?;
        if route
            .stops
            .first()
            .is_some_and(|stop| stop.accumulated_load.value != zero)
        {
            return Err(NetworkSchedulingError::contract(
                "路线根节点负载不为零 / route root load is not zero",
            ));
        }
        RouteValidator::validate(
            &request.instance,
            &route,
            &self.graph_builder.distance_calculator,
            &self.graph_builder.travel_time_calculator,
            &self.graph_builder.arc_cost_calculator,
            &self.graph_builder.route_cost_policy,
            request.branch_mask.as_ref(),
        )?;
        Ok(route)
    }
}

fn validate_reduced_cost_snapshot<V>(
    graph: &PricingGraph,
    request: &PricingRequest<V>,
) -> Result<()>
where
    V: SolveValue + UnitConversionValue,
{
    for (index, arc) in graph.arcs.iter().enumerate() {
        let from = graph.nodes.get(arc.from).ok_or_else(|| {
            NetworkSchedulingError::structure(
                "定价图弧起点索引无效 / pricing graph arc origin index is invalid",
            )
        })?;
        let to = graph.nodes.get(arc.to).ok_or_else(|| {
            NetworkSchedulingError::structure(
                "定价图弧终点索引无效 / pricing graph arc destination index is invalid",
            )
        })?;
        let customer_dual = to
            .customer_index
            .and_then(|customer_index| request.instance.customers.get(customer_index))
            .map(|customer| graph.duals.customer_dual(&customer.id))
            .unwrap_or(0.0);
        let fleet_dual = if from.is_start_depot {
            graph.duals.fleet_dual(&request.vehicle_type_id)
        } else {
            0.0
        };
        let phase_cost = match graph.duals.phase {
            crate::domain::vrp::PricingPhase::PhaseOne => 0.0,
            crate::domain::vrp::PricingPhase::PhaseTwo => arc.route_cost,
        };
        let expected = phase_cost - customer_dual - fleet_dual;
        if !arc.reduced_cost.is_finite() || arc.reduced_cost != expected {
            return Err(NetworkSchedulingError::pricing(format!(
                "定价图弧 {} 的 reduced cost 快照无效 / reduced-cost snapshot for pricing arc {} is invalid",
                index, index
            )));
        }
    }
    Ok(())
}

impl<V, D, T, A, C>
    EspprcPricer<V, D, T, A, C, DefaultLabelDominancePolicy, DefaultPricingColumnSelector>
where
    V: SolveValue + UnitConversionValue,
    D: crate::domain::vrp::DistanceCalculator<V>,
    T: crate::domain::vrp::TravelTimeCalculator<V>,
    A: crate::domain::vrp::ArcCostCalculator<V>,
    C: crate::domain::vrp::RouteCostPolicy<V>,
{
    /// 使用默认支配与列选择策略创建定价器 / Create a pricer with default dominance and selection policies.
    pub fn new(graph_builder: RouteGraphBuilder<V, D, T, A, C>) -> Self {
        Self::with_policies(
            graph_builder,
            DefaultLabelDominancePolicy,
            DefaultPricingColumnSelector,
        )
    }
}

fn duration_value<V>(request: &PricingRequest<V>, duration: time::Duration) -> Result<f64>
where
    V: SolveValue + UnitConversionValue,
{
    request
        .instance
        .scheduling_window
        .value_of_duration(duration)
        .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
        .map_err(|error| NetworkSchedulingError::Conversion {
            message: error.to_string(),
        })
}

#[allow(clippy::too_many_arguments)]
fn update_forbidden_customers<V>(
    current_forbidden: &ForbiddenCustomers,
    graph: &PricingGraph,
    current_node: usize,
    visited: &VisitedCustomers,
    departure_time: f64,
    load: f64,
    tolerance: f64,
    request: &PricingRequest<V>,
) -> Result<(ForbiddenCustomers, usize)>
where
    V: SolveValue + UnitConversionValue,
{
    let mut forbidden = current_forbidden.clone();
    let mut markings = 0;
    if let Some(outgoing) = graph.outgoing.get(current_node) {
        for arc_index in outgoing {
            let Some(arc) = graph.arcs.get(*arc_index) else {
                continue;
            };
            let Some(candidate) = graph.nodes.get(arc.to) else {
                continue;
            };
            let Some(index) = candidate.customer_index else {
                continue;
            };
            if visited.contains(index) || forbidden.contains(index) {
                continue;
            }
            let arrival = departure_time + duration_value(request, arc.travel_time)?;
            let service_start = arrival.max(candidate.ready_time);
            if service_start > candidate.due_time + tolerance
                || load + candidate.demand > graph.vehicle_capacity + tolerance
            {
                forbidden = forbidden.add(index);
                markings += 1;
            }
        }
    }
    Ok((forbidden, markings))
}
