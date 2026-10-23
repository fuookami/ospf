//! ESPPRC 精确定价器 / ESPPRC exact pricer.

use std::cell::Cell;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use crate::domain::path::{PathProblem, PathSearchBudget, PathSearchCompletion, PathSearchContext};
use crate::domain::route_generation::graph::{PricingGraph, RouteGraphBuilder};
use crate::domain::route_generation::model::{
    EspprcLabel, ForbiddenCustomers, LabelStatistics, PricingRequest, PricingResult,
    TruncationReason, VehiclePricingDiagnostic, VisitedCustomers,
};
use crate::domain::route_generation::path_search::{self, PricingLabel, PricingPathPolicy};
use crate::domain::route_generation::policy::{
    DefaultLabelDominancePolicy, DefaultPricingColumnSelector, LabelDominancePolicy,
    PricingColumnSelector,
};
use crate::domain::vrp::{Route, RoutePricingArc, RouteStop, RouteValidator};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::NetworkSchedulingSolverValueAdapter;

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
        self.graph_builder.pricing_rows.validate_dual_keys(&request.duals.extension)?;
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        for arc in &graph.arcs {
            let coefficients: std::collections::BTreeMap<_, _> = self.graph_builder.pricing_rows.arc_coefficients(&RoutePricingArc {
                instance: &request.instance, vehicle_type: &request.vehicle_type_id, arc: &arc.arc_id,
                from: &graph.nodes[arc.from].node_id, to: &graph.nodes[arc.to].node_id,
            })?.into_iter().map(|(id, value)| Ok((id, adapter.to_solver(&value)?))).collect::<Result<_>>()?;
            if coefficients != arc.row_coefficients {
                return Err(NetworkSchedulingError::pricing("扩展行提供器与图快照不一致 / extension-row provider disagrees with graph snapshot"));
            }
        }
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
        let max_depth = request.max_depth.unwrap_or(customer_count);
        if max_depth > customer_count {
            return Err(NetworkSchedulingError::validation(
                "定价最大深度不能超过客户数 / pricing max depth cannot exceed customer count",
            ));
        }
        let depth_limited = Cell::new(false);
        let unreachable = Cell::new(0);
        let minimum = Cell::new(f64::INFINITY);
        let root = EspprcLabel {
            reduced_cost: 0.0, time: start_node.ready_time, load: 0.0, current_node: start_index,
            visited: VisitedCustomers::empty(customer_count), forbidden: ForbiddenCustomers::empty(customer_count),
            predecessor: None, predecessor_arc: None,
        };
        let extend = |index: usize, label: &EspprcLabel, arc_index: usize, _: usize| -> Result<Option<EspprcLabel>> {
            let arc = &graph.arcs[arc_index];
            let node = &graph.nodes[arc.to];
            if arc.to == end_index && label.visited.count() == 0 { return Ok(None); }
            if let Some(customer) = node.customer_index {
                if label.visited.contains(customer) || label.forbidden.contains(customer) { return Ok(None); }
                if label.visited.count() >= max_depth {
                    depth_limited.set(true);
                    return Ok(None);
                }
            }
            let arrival = label.time + duration_value(request, arc.travel_time)?;
            let service_start = arrival.max(node.ready_time);
            let load = label.load + node.demand;
            if service_start > node.due_time + request.instance.tolerances.feasibility
                || load > graph.vehicle_capacity + request.instance.tolerances.feasibility
            { return Ok(None); }
            let visited = node.customer_index.map_or_else(|| label.visited.clone(), |index| label.visited.add(index));
            let (forbidden, markings) = update_forbidden_customers(&label.forbidden, graph,
                arc.to, &visited, load, request.instance.tolerances.feasibility);
            unreachable.set(unreachable.get() + markings);
            Ok(Some(EspprcLabel {
                reduced_cost: label.reduced_cost + arc.reduced_cost,
                time: service_start + node.service_time, load, current_node: arc.to, visited, forbidden,
                predecessor: Some(index), predecessor_arc: Some(arc_index),
            }))
        };
        let finish = |labels: &[EspprcLabel]| -> Result<Option<(EspprcLabel, Route<V>)>> {
            let route = self.backtrack_route(graph, labels, labels.len() - 1, request)?;
            let cost = match request.duals.phase {
                crate::domain::vrp::PricingPhase::PhaseOne => 0.0,
                crate::domain::vrp::PricingPhase::PhaseTwo => route.cost.value
                    .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                    .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?,
            };
            let coefficients = self.graph_builder.pricing_rows.route_coefficients(&route, &request.instance)?;
            let row_price = coefficients.iter().map(|(id, value)| Ok(adapter.to_solver(value)? * request.duals.extension[id]))
                .collect::<Result<Vec<_>>>()?.into_iter().sum::<f64>();
            let reduced_cost = cost - request.duals.fleet_dual(&request.vehicle_type_id)
                - route.customer_ids().iter().map(|id| request.duals.customer_dual(id)).sum::<f64>() - row_price;
            if !cost.is_finite() || !reduced_cost.is_finite() {
                return Err(NetworkSchedulingError::pricing("完整路径费用必须有限 / complete-path costs must be finite"));
            }
            minimum.set(minimum.get().min(reduced_cost));
            Ok((reduced_cost < -request.pricing_tolerance).then(|| {
                let mut label = labels[labels.len() - 1].clone();
                label.reduced_cost = reduced_cost;
                (label, route)
            }))
        };
        let dominates = |left: &EspprcLabel, right: &EspprcLabel| self.dominance_policy.dominates(left, right);
        let compare = |left: &EspprcLabel, a: &Route<V>, right: &EspprcLabel, b: &Route<V>|
            left.reduced_cost.total_cmp(&right.reduced_cost).then_with(|| a.signature().cmp(&b.signature()));
        let use_dominance = self.graph_builder.route_cost_policy.additive_label_cost_is_exact()
            && self.dominance_policy.is_certified() && self.graph_builder.pricing_rows.additive_decomposition_is_exact();
        let graph_view = path_search::graph(graph.nodes.len(), graph.arcs.iter().map(|arc| (arc.arc_id.clone(), arc.from, arc.to)))?;
        let policy = PricingPathPolicy { root, extend: &extend, finish: &finish,
            dominates: use_dominance.then_some(&dominates), compare: &compare };
        let mut problem = PathProblem::new(&graph_view, policy);
        problem.starts = vec![path_search::node_id(start_index)];
        problem.terminals = [path_search::node_id(end_index)].into();
        problem.options.maximum_steps = Some(customer_count + 1);
        let interrupt = || request.interrupted();
        let search = PathSearchContext.search(&problem, &PathSearchBudget {
            maximum_labels: request.max_labels, interrupted: Some(&interrupt), ..Default::default()
        })?;
        let interrupted = search.completion == PathSearchCompletion::Interrupted;
        let exact_pricing_complete = search.complete() && !depth_limited.get();
        let statistics = LabelStatistics {
            labels_created: search.diagnostics.generated_labels, extensions: search.diagnostics.extensions,
            dominance_checks: search.diagnostics.dominance_checks, dominated_labels: search.diagnostics.dominated_labels,
            unreachable_markings: unreachable.get(),
        };
        let routes: Vec<_> = search.paths.into_iter().filter_map(|candidate| candidate.state.route).collect();
        let selected_routes = self.column_selector.select(&routes, request.max_columns_per_pricing);
        let truncation_reason = if search.completion == PathSearchCompletion::BudgetLimit { Some(TruncationReason::MaxLabels) }
            else if depth_limited.get() { Some(TruncationReason::MaxDepth) }
            else if selected_routes.len() < routes.len() { Some(TruncationReason::MaxColumns) }
            else { None };
        let truncated = truncation_reason.is_some();
        let min_reduced_cost = if minimum.get().is_finite() { minimum.get() } else { 0.0 };
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
        let mut arc_costs = Vec::new();
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
                arc_costs.push(Quantity::new(
                    V::from_f64_with_policy(arc.arc_cost, SolveValueConversionPolicy::AllowRounding)
                        .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?,
                    request.instance.units.cost_unit.clone(),
                ));
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
        let vehicle = request.instance.vehicle_type(&request.vehicle_type_id).ok_or_else(||
            NetworkSchedulingError::structure("车辆类型不存在 / missing vehicle type"))?;
        let policy_cost = self.graph_builder.route_cost_policy.cost(vehicle, &arc_costs, &request.instance)?;
        crate::infrastructure::validate_linear_units([&policy_cost.unit, &request.instance.units.cost_unit])?;
        let complete_cost = policy_cost.to_unit(&request.instance.units.cost_unit)
            .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
        crate::infrastructure::NetworkSchedulingSolverValueAdapter::<V>::default().to_solver(&complete_cost.value)?;
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
            complete_cost,
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
        if arc.row_coefficients.len() != request.duals.extension.len() {
            return Err(NetworkSchedulingError::pricing("图扩展行与对偶域不一致 / graph extension rows disagree with dual domain"));
        }
        let mut row_price = 0.0;
        for (id, value) in &arc.row_coefficients {
            let price = request.duals.extension.get(id).ok_or_else(|| NetworkSchedulingError::pricing(
                "图扩展行缺少对偶 / graph extension row lacks a dual"))?;
            row_price += value * price;
        }
        let expected = phase_cost - customer_dual - fleet_dual - row_price;
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
fn update_forbidden_customers(
    current_forbidden: &ForbiddenCustomers,
    graph: &PricingGraph,
    current_node: usize,
    visited: &VisitedCustomers,
    load: f64,
    tolerance: f64,
) -> (ForbiddenCustomers, usize) {
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
            // 直达失败不代表经其他客户后仍失败；这里只保留单调容量排除。 / Direct travel failure does not rule out an indirect path; retain only monotone capacity exclusion.
            if load + candidate.demand > graph.vehicle_capacity + tolerance {
                forbidden = forbidden.add(index);
                markings += 1;
            }
        }
    }
    (forbidden, markings)
}

impl PricingLabel for EspprcLabel {
    fn set_predecessor(&mut self, predecessor: Option<usize>, arc: Option<usize>) {
        self.predecessor = predecessor;
        self.predecessor_arc = arc;
    }
}
