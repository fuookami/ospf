//! BigDecimal 精确定价路径 / BigDecimal exact pricing path.
//!
//! 该实现只接受显式基础弧。显式弧的距离、成本和时间由实例直接提供，避免默认
//! 欧氏距离以及 `TimeWindow` 的 f64 适配器破坏精度。solver 对偶仍由调用方以
//! BigDecimal 提供，整个 ESPPRC 标签扩展不经过 f64。
//! This implementation accepts explicit base arcs only. Distances, costs, and travel
//! times come directly from the instance, avoiding the f64 adapters used by default
//! Euclidean and `TimeWindow` policies. Solver duals are supplied as BigDecimal and
//! the complete ESPPRC label extension stays outside f64.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;

use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use ospf_rust_framework_gantt_scheduling::infrastructure::DurationUnit;
use ospf_rust_quantities::Quantity;
use time::{Duration, OffsetDateTime};

use crate::domain::route_generation::{
    CancellationToken, ForbiddenCustomers, LabelStatistics, TruncationReason,
};
use crate::domain::vrp::{
    ArcFeasibilityPolicy, BranchMask, CustomerId, DefaultArcFeasibilityPolicy,
    DistanceArcCostCalculator, DistanceAsTravelTimeCalculator, EuclideanDistanceCalculator,
    FixedPlusArcCostPolicy, PricingPhase, Route, RouteStop, RouteValidationPolicy, RouteValidator,
    VehicleTypeId, VrptwInstance,
};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkNodeId};

/// BigDecimal 精确定价图节点 / BigDecimal exact pricing-graph node.
#[derive(Debug, Clone)]
pub struct ExactBigDecimalPricingNode {
    /// 节点 ID / Node ID.
    pub node_id: NetworkNodeId,
    /// 客户原始索引；仓库为 `None` / Original customer index; `None` for depots.
    pub customer_index: Option<usize>,
    /// 是否起始仓库 / Whether this is the start depot.
    pub is_start_depot: bool,
    /// 是否结束仓库 / Whether this is the end depot.
    pub is_end_depot: bool,
    /// 最早服务开始时刻 / Earliest service-start value.
    pub ready_time: BigDecimal,
    /// 最晚服务开始时刻 / Latest service-start value.
    pub due_time: BigDecimal,
    /// 服务时长 / Service duration.
    pub service_time: BigDecimal,
    /// 客户需求 / Customer demand.
    pub demand: BigDecimal,
}

/// BigDecimal 精确定价图弧 / BigDecimal exact pricing-graph arc.
#[derive(Debug, Clone)]
pub struct ExactBigDecimalPricingArc {
    /// 稳定弧 ID / Stable arc ID.
    pub arc_id: NetworkArcId,
    /// 起点索引 / Origin index.
    pub from: usize,
    /// 终点索引 / Destination index.
    pub to: usize,
    /// 距离 / Distance.
    pub distance: BigDecimal,
    /// 行驶时间 / Travel duration.
    pub travel_time: Duration,
    /// 弧成本 / Arc cost.
    pub arc_cost: BigDecimal,
    /// 计入固定成本后的路线成本增量 / Route-cost increment including start fixed cost.
    pub route_cost: BigDecimal,
    /// reduced cost 增量 / Reduced-cost increment.
    pub reduced_cost: BigDecimal,
}

/// BigDecimal 精确定价图 / BigDecimal exact pricing graph.
#[derive(Debug, Clone)]
pub struct ExactBigDecimalPricingGraph {
    /// 节点 / Nodes.
    pub nodes: Vec<ExactBigDecimalPricingNode>,
    /// 弧 / Arcs.
    pub arcs: Vec<ExactBigDecimalPricingArc>,
    /// 出弧索引 / Outgoing arc indices.
    pub outgoing: Vec<Vec<usize>>,
    /// 车辆类型 / Vehicle type.
    pub vehicle_type_id: VehicleTypeId,
    /// 车辆容量 / Vehicle capacity.
    pub vehicle_capacity: BigDecimal,
    /// 构图时使用的分支遮罩 / Branch mask used while building the graph.
    pub branch_mask: Option<BranchMask<VehicleTypeId>>,
    /// 构图时使用的对偶快照 / Dual snapshot used while building the graph.
    pub duals: ExactBigDecimalPricingDuals,
    /// 构图时使用的实例身份 / Instance identity used while building the graph.
    pub instance_identity: u64,
    /// 构图时使用的实例内容指纹 / Instance-content fingerprint used while building the graph.
    pub instance_fingerprint: u64,
    /// 图快照完整性指纹；外部调用方不能伪造 / Graph-snapshot integrity fingerprint; callers outside this crate cannot forge it.
    pub(crate) integrity_fingerprint: u64,
}

impl ExactBigDecimalPricingGraph {
    /// 获取起始仓库索引 / Get the start-depot index.
    pub fn start_index(&self) -> Option<usize> {
        self.nodes.iter().position(|node| node.is_start_depot)
    }

    /// 获取结束仓库索引 / Get the end-depot index.
    pub fn end_index(&self) -> Option<usize> {
        self.nodes.iter().position(|node| node.is_end_depot)
    }

    /// 检查公开图数据是否仍与构图快照一致 / Check whether public graph data still matches the build snapshot.
    pub(crate) fn has_valid_integrity(&self) -> bool {
        self.integrity_fingerprint == self.calculate_integrity_fingerprint()
    }

    fn calculate_integrity_fingerprint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        format!(
            "{:?}",
            (
                &self.nodes,
                &self.arcs,
                &self.outgoing,
                &self.vehicle_type_id,
                &self.vehicle_capacity,
                &self.branch_mask,
                &self.duals,
                self.instance_identity,
                self.instance_fingerprint,
            )
        )
        .hash(&mut hasher);
        hasher.finish()
    }
}

/// BigDecimal 定价对偶快照 / BigDecimal pricing-dual snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactBigDecimalPricingDuals {
    /// 定价阶段 / Pricing phase.
    pub phase: PricingPhase,
    /// 客户覆盖对偶 / Customer-cover duals.
    pub customer: BTreeMap<CustomerId, BigDecimal>,
    /// 车辆类型上界对偶 / Fleet upper-bound duals.
    pub fleet: BTreeMap<VehicleTypeId, BigDecimal>,
}

impl ExactBigDecimalPricingDuals {
    /// 创建 BigDecimal 对偶快照 / Create a BigDecimal dual snapshot.
    pub fn new(
        phase: PricingPhase,
        customer: impl IntoIterator<Item = (CustomerId, BigDecimal)>,
        fleet: impl IntoIterator<Item = (VehicleTypeId, BigDecimal)>,
    ) -> Self {
        Self {
            phase,
            customer: customer.into_iter().collect(),
            fleet: fleet.into_iter().collect(),
        }
    }

    fn customer_dual(&self, id: &CustomerId) -> BigDecimal {
        self.customer
            .get(id)
            .cloned()
            .unwrap_or_else(BigDecimal::zero)
    }

    fn fleet_dual(&self, id: &VehicleTypeId) -> BigDecimal {
        self.fleet.get(id).cloned().unwrap_or_else(BigDecimal::zero)
    }
}

/// BigDecimal 精确定价请求 / BigDecimal exact pricing request.
#[derive(Debug, Clone)]
pub struct ExactBigDecimalPricingRequest {
    /// VRPTW 实例 / VRPTW instance.
    pub instance: Arc<VrptwInstance<BigDecimal>>,
    /// BigDecimal 对偶 / BigDecimal duals.
    pub duals: ExactBigDecimalPricingDuals,
    /// 当前分支遮罩 / Current branch mask.
    pub branch_mask: Option<BranchMask<VehicleTypeId>>,
    /// 负列判定容差 / Negative-column tolerance.
    pub pricing_tolerance: BigDecimal,
    /// 单次最多返回的列数 / Maximum returned columns.
    pub max_columns_per_pricing: usize,
    /// 当前车辆类型 / Current vehicle type.
    pub vehicle_type_id: VehicleTypeId,
    /// 取消令牌 / Cancellation token.
    pub cancellation: Option<CancellationToken>,
    /// 截止时间 / Deadline.
    pub deadline: Option<Instant>,
    /// 标签上限 / Label limit.
    pub max_labels: Option<usize>,
    /// 路径客户数上限 / Maximum customers per path.
    pub max_depth: Option<usize>,
}

impl ExactBigDecimalPricingRequest {
    /// 创建精确定价请求 / Create an exact pricing request.
    pub fn new(
        instance: Arc<VrptwInstance<BigDecimal>>,
        duals: ExactBigDecimalPricingDuals,
        vehicle_type_id: VehicleTypeId,
    ) -> Self {
        Self {
            instance,
            duals,
            branch_mask: None,
            pricing_tolerance: BigDecimal::zero(),
            max_columns_per_pricing: usize::MAX,
            vehicle_type_id,
            cancellation: None,
            deadline: None,
            max_labels: None,
            max_depth: None,
        }
    }

    fn interrupted(&self) -> bool {
        self.cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
            || self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
    }
}

/// BigDecimal 精确定价结果 / BigDecimal exact pricing result.
#[derive(Debug, Clone)]
pub struct ExactBigDecimalPricingResult {
    /// 发现的负 reduced-cost 路线 / Negative reduced-cost routes.
    pub routes: Vec<Route<BigDecimal>>,
    /// 最小 reduced cost / Minimum reduced cost.
    pub min_reduced_cost: BigDecimal,
    /// 是否完成精确定价 / Whether exact pricing completed.
    pub exact_pricing_complete: bool,
    /// 是否中断 / Whether interrupted.
    pub interrupted: bool,
    /// 是否截断 / Whether truncated.
    pub truncated: bool,
    /// 截断原因 / Truncation reason.
    pub truncation_reason: Option<TruncationReason>,
    /// 标签统计 / Label statistics.
    pub statistics: LabelStatistics,
}

/// 只基于显式基础弧的 BigDecimal 图构建器 / BigDecimal graph builder based on explicit base arcs.
pub struct ExactBigDecimalRouteGraphBuilder {
    /// VRPTW 实例 / VRPTW instance.
    pub instance: Arc<VrptwInstance<BigDecimal>>,
}

impl ExactBigDecimalRouteGraphBuilder {
    /// 创建精确图构建器 / Create an exact graph builder.
    pub fn new(instance: Arc<VrptwInstance<BigDecimal>>) -> Self {
        Self { instance }
    }

    /// 构建精确定价图 / Build an exact pricing graph.
    pub fn build(
        &self,
        vehicle_type_id: &VehicleTypeId,
        duals: &ExactBigDecimalPricingDuals,
        branch_mask: Option<&BranchMask<VehicleTypeId>>,
    ) -> Result<ExactBigDecimalPricingGraph> {
        if self.instance.arcs.is_empty() {
            return Err(NetworkSchedulingError::validation(
                "BigDecimal 精确定价要求显式基础弧；隐式完全图策略可能经过 f64 / BigDecimal exact pricing requires explicit base arcs; implicit complete-graph policies may pass through f64",
            ));
        }
        let vehicle = self.instance.vehicle_type(vehicle_type_id).ok_or_else(|| {
            NetworkSchedulingError::validation(format!(
                "车辆类型不存在：{} / vehicle type does not exist: {}",
                vehicle_type_id, vehicle_type_id
            ))
        })?;
        let mut nodes = Vec::with_capacity(self.instance.customers.len() + 2);
        nodes.push(self.node_for_depot(&self.instance.start_depot, true, false)?);
        for (index, customer) in self.instance.customers.iter().enumerate() {
            if branch_mask.is_some_and(|mask| !mask.allows_node(vehicle_type_id, &customer.node.id))
            {
                continue;
            }
            let time_window = customer.time_window;
            nodes.push(ExactBigDecimalPricingNode {
                node_id: customer.node.id.clone(),
                customer_index: Some(index),
                is_start_depot: false,
                is_end_depot: false,
                ready_time: instant_value(&self.instance, time_window.ready_time)?,
                due_time: instant_value(&self.instance, time_window.due_time)?,
                service_time: duration_value(&self.instance, customer.service_time)?,
                demand: customer
                    .demand
                    .to_unit(&self.instance.units.load_unit)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?
                    .value,
            });
        }
        nodes.push(self.node_for_depot(&self.instance.end_depot, false, true)?);

        let node_indices = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.node_id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let capacity = vehicle
            .capacity
            .to_unit(&self.instance.units.load_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value;
        let fixed_cost = vehicle
            .fixed_cost
            .to_unit(&self.instance.units.cost_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value;
        let mut arcs = Vec::new();
        let mut outgoing = vec![Vec::new(); nodes.len()];
        for base_arc in &self.instance.arcs {
            let Some(&from) = node_indices.get(&base_arc.from) else {
                continue;
            };
            let Some(&to) = node_indices.get(&base_arc.to) else {
                continue;
            };
            if nodes[from].is_end_depot
                || nodes[to].is_start_depot
                || from == to
                || !base_arc.feasible
            {
                continue;
            }
            if base_arc.travel_time.duration < Duration::ZERO {
                return Err(NetworkSchedulingError::validation(
                    "行驶时间不能为负 / travel time cannot be negative",
                ));
            }
            if base_arc.distance.value < BigDecimal::zero() {
                return Err(NetworkSchedulingError::validation(
                    "弧距离不能为负 / arc distance cannot be negative",
                ));
            }
            if branch_mask.is_some_and(|mask| {
                !mask.allows_arc(vehicle_type_id, &base_arc.id, &base_arc.from, &base_arc.to)
            }) {
                continue;
            }
            let distance = base_arc
                .distance
                .to_unit(&self.instance.units.distance_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?
                .value;
            let arc_cost = base_arc
                .cost
                .to_unit(&self.instance.units.cost_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?
                .value;
            let route_cost = if nodes[from].is_start_depot {
                &arc_cost + &fixed_cost
            } else {
                arc_cost.clone()
            };
            let customer_dual = nodes[to]
                .customer_index
                .and_then(|index| self.instance.customers.get(index))
                .map(|customer| duals.customer_dual(&customer.id))
                .unwrap_or_else(BigDecimal::zero);
            let fleet_dual = if nodes[from].is_start_depot {
                duals.fleet_dual(vehicle_type_id)
            } else {
                BigDecimal::zero()
            };
            let phase_cost = match duals.phase {
                PricingPhase::PhaseOne => BigDecimal::zero(),
                PricingPhase::PhaseTwo => route_cost.clone(),
            };
            let index = arcs.len();
            arcs.push(ExactBigDecimalPricingArc {
                arc_id: base_arc.id.clone(),
                from,
                to,
                distance,
                travel_time: base_arc.travel_time.duration,
                arc_cost,
                route_cost,
                reduced_cost: phase_cost - customer_dual - fleet_dual,
            });
            outgoing[from].push(index);
        }
        let mut graph = ExactBigDecimalPricingGraph {
            nodes,
            arcs,
            outgoing,
            vehicle_type_id: vehicle_type_id.clone(),
            vehicle_capacity: capacity,
            branch_mask: branch_mask.cloned(),
            duals: duals.clone(),
            instance_identity: self.instance.instance_identity(),
            instance_fingerprint: self.instance.pricing_fingerprint(),
            integrity_fingerprint: 0,
        };
        graph.integrity_fingerprint = graph.calculate_integrity_fingerprint();
        Ok(graph)
    }

    fn node_for_depot(
        &self,
        depot: &crate::domain::vrp::Depot<BigDecimal>,
        is_start_depot: bool,
        is_end_depot: bool,
    ) -> Result<ExactBigDecimalPricingNode> {
        Ok(ExactBigDecimalPricingNode {
            node_id: depot.node.id.clone(),
            customer_index: None,
            is_start_depot,
            is_end_depot,
            ready_time: instant_value(&self.instance, depot.time_window.ready_time)?,
            due_time: instant_value(&self.instance, depot.time_window.due_time)?,
            service_time: BigDecimal::zero(),
            demand: BigDecimal::zero(),
        })
    }
}

/// BigDecimal elementary ESPPRC 定价器 / BigDecimal elementary ESPPRC pricer.
pub struct ExactBigDecimalEspprcPricer {
    /// 显式弧图构建器 / Explicit-arc graph builder.
    pub graph_builder: ExactBigDecimalRouteGraphBuilder,
}

impl ExactBigDecimalEspprcPricer {
    /// 创建精确定价器 / Create an exact pricer.
    pub fn new(graph_builder: ExactBigDecimalRouteGraphBuilder) -> Self {
        Self { graph_builder }
    }

    /// 构图并执行精确定价 / Build the graph and execute exact pricing.
    pub fn price_request(
        &self,
        request: &ExactBigDecimalPricingRequest,
    ) -> Result<ExactBigDecimalPricingResult> {
        let graph = self.graph_builder.build(
            &request.vehicle_type_id,
            &request.duals,
            request.branch_mask.as_ref(),
        )?;
        self.price(&graph, request)
    }

    /// 在已构建图上执行精确定价 / Price on an already-built graph.
    pub fn price(
        &self,
        graph: &ExactBigDecimalPricingGraph,
        request: &ExactBigDecimalPricingRequest,
    ) -> Result<ExactBigDecimalPricingResult> {
        if graph.vehicle_type_id != request.vehicle_type_id {
            return Err(NetworkSchedulingError::pricing(
                "BigDecimal 定价图车辆类型与请求不一致 / BigDecimal pricing graph vehicle type does not match the request",
            ));
        }
        if !graph.has_valid_integrity() {
            return Err(NetworkSchedulingError::pricing(
                "BigDecimal 定价图公开快照完整性校验失败 / BigDecimal public pricing graph snapshot integrity check failed",
            ));
        }
        if graph.branch_mask != request.branch_mask {
            return Err(NetworkSchedulingError::pricing(
                "BigDecimal 定价图分支遮罩与请求不一致 / BigDecimal pricing graph branch mask does not match the request",
            ));
        }
        if graph.instance_identity != request.instance.instance_identity()
            || graph.instance_fingerprint != request.instance.pricing_fingerprint()
        {
            return Err(NetworkSchedulingError::pricing(
                "BigDecimal 定价图实例快照与请求不一致 / BigDecimal pricing graph instance snapshot does not match the request",
            ));
        }
        if graph.duals != request.duals {
            return Err(NetworkSchedulingError::pricing(
                "BigDecimal 定价图对偶快照与请求不一致 / BigDecimal pricing graph dual snapshot does not match the request",
            ));
        }
        validate_reduced_cost_snapshot(graph, request)?;
        let start_index = graph.start_index().ok_or_else(|| {
            NetworkSchedulingError::structure(
                "BigDecimal 定价图缺少起点 / BigDecimal pricing graph has no start depot",
            )
        })?;
        let end_index = graph.end_index().ok_or_else(|| {
            NetworkSchedulingError::structure(
                "BigDecimal 定价图缺少终点 / BigDecimal pricing graph has no end depot",
            )
        })?;
        let customer_count = request.instance.customers.len();
        let max_depth = request.max_depth.unwrap_or(customer_count);
        if max_depth > customer_count {
            return Err(NetworkSchedulingError::validation(
                "定价最大深度不能超过客户数 / pricing max depth cannot exceed customer count",
            ));
        }

        let root = ExactLabel {
            reduced_cost: BigDecimal::zero(),
            time: graph.nodes[start_index].ready_time.clone(),
            load: BigDecimal::zero(),
            current_node: start_index,
            visited: crate::domain::route_generation::VisitedCustomers::empty(customer_count),
            forbidden: ForbiddenCustomers::empty(customer_count),
            predecessor: None,
            predecessor_arc: None,
        };
        let mut labels = vec![root];
        let mut dominated = vec![false];
        let mut labels_at_node = vec![Vec::<usize>::new(); graph.nodes.len()];
        labels_at_node[start_index].push(0);
        let mut queue = VecDeque::from([0usize]);
        let mut statistics = LabelStatistics {
            labels_created: 1,
            ..LabelStatistics::default()
        };
        let mut negative_label_indices = Vec::new();
        let mut min_reduced_cost: Option<BigDecimal> = None;
        let mut interrupted = request.interrupted();
        let mut truncated = false;
        let mut truncation_reason = None;

        while let Some(label_index) = queue.pop_front() {
            if request.interrupted() {
                interrupted = true;
                break;
            }
            if dominated[label_index] {
                continue;
            }
            let label = labels.get(label_index).cloned().ok_or_else(|| {
                NetworkSchedulingError::contract(
                    "BigDecimal 前驱标签索引无效 / BigDecimal predecessor label index is invalid",
                )
            })?;
            for &arc_index in graph.outgoing.get(label.current_node).ok_or_else(|| {
                NetworkSchedulingError::structure(
                    "BigDecimal 出弧索引无效 / BigDecimal outgoing arc index is invalid",
                )
            })? {
                if request.interrupted() {
                    interrupted = true;
                    break;
                }
                let arc = graph.arcs.get(arc_index).ok_or_else(|| {
                    NetworkSchedulingError::structure(
                        "BigDecimal 定价弧索引无效 / BigDecimal pricing arc index is invalid",
                    )
                })?;
                if arc.from != label.current_node {
                    return Err(NetworkSchedulingError::contract(
                        "BigDecimal 出弧索引与弧起点不一致 / BigDecimal outgoing index disagrees with arc origin",
                    ));
                }
                let node = graph.nodes.get(arc.to).ok_or_else(|| {
                    NetworkSchedulingError::structure(
                        "BigDecimal 定价终点不存在 / BigDecimal pricing destination does not exist",
                    )
                })?;
                // 空路线不是 VRPTW 列，不能让 start->end 标签支配客户路线。
                // An empty route is not a VRPTW column and must not dominate customer routes.
                if arc.to == end_index && label.visited.count() == 0 {
                    continue;
                }
                if let Some(customer_index) = node.customer_index {
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
                let arrival = &label.time + duration_value(&request.instance, arc.travel_time)?;
                let service_start = if arrival > node.ready_time {
                    arrival
                } else {
                    node.ready_time.clone()
                };
                if service_start > node.due_time {
                    continue;
                }
                let new_load = &label.load + &node.demand;
                if new_load > graph.vehicle_capacity {
                    continue;
                }
                let visited = node
                    .customer_index
                    .map_or_else(|| label.visited.clone(), |index| label.visited.add(index));
                let new_time = service_start + &node.service_time;
                let (new_forbidden, unreachable_markings) = update_forbidden_customers(
                    &label.forbidden,
                    graph,
                    arc.to,
                    &visited,
                    &new_time,
                    &new_load,
                    &request.instance,
                )?;
                statistics.unreachable_markings += unreachable_markings;
                let new_label = ExactLabel {
                    reduced_cost: &label.reduced_cost + &arc.reduced_cost,
                    time: new_time,
                    load: new_load,
                    current_node: arc.to,
                    visited,
                    forbidden: new_forbidden,
                    predecessor: Some(label_index),
                    predecessor_arc: Some(arc_index),
                };
                statistics.extensions += 1;
                statistics.dominance_checks += labels_at_node[arc.to].len();
                if labels_at_node[arc.to].iter().any(|candidate| {
                    !dominated[*candidate] && dominates(&labels[*candidate], &new_label)
                }) {
                    statistics.dominated_labels += 1;
                    continue;
                }
                let dominated_indices = labels_at_node[arc.to]
                    .iter()
                    .copied()
                    .filter(|candidate| {
                        !dominated[*candidate] && dominates(&new_label, &labels[*candidate])
                    })
                    .collect::<Vec<_>>();
                for candidate in dominated_indices {
                    dominated[candidate] = true;
                    statistics.dominated_labels += 1;
                }
                labels_at_node[arc.to].retain(|candidate| !dominated[*candidate]);
                if request
                    .max_labels
                    .is_some_and(|limit| labels.len() >= limit)
                {
                    truncated = true;
                    truncation_reason.get_or_insert(TruncationReason::MaxLabels);
                    break;
                }
                let new_index = labels.len();
                let reduced_cost = new_label.reduced_cost.clone();
                labels.push(new_label);
                dominated.push(false);
                labels_at_node[arc.to].push(new_index);
                queue.push_back(new_index);
                statistics.labels_created += 1;
                if arc.to == end_index && labels[new_index].visited.count() > 0 {
                    if min_reduced_cost
                        .as_ref()
                        .is_none_or(|minimum| reduced_cost < *minimum)
                    {
                        min_reduced_cost = Some(reduced_cost.clone());
                    }
                    if reduced_cost < -&request.pricing_tolerance {
                        negative_label_indices.push(new_index);
                    }
                }
            }
            if interrupted
                || truncated && matches!(truncation_reason, Some(TruncationReason::MaxLabels))
            {
                break;
            }
        }

        let mut routes = negative_label_indices
            .into_iter()
            .map(|index| self.backtrack_route(graph, &labels, index, request))
            .collect::<Result<Vec<_>>>()?;
        for route in &routes {
            self.validate_backtracked_route(route, request)?;
        }
        routes.sort_by_key(Route::signature);
        let selected_count = routes.len().min(request.max_columns_per_pricing);
        if selected_count < routes.len() {
            truncated = true;
            truncation_reason.get_or_insert(TruncationReason::MaxColumns);
            routes.truncate(selected_count);
        }
        let exact_pricing_complete = !(interrupted
            || matches!(
                truncation_reason,
                Some(TruncationReason::MaxLabels | TruncationReason::MaxDepth)
            ));
        Ok(ExactBigDecimalPricingResult {
            routes,
            min_reduced_cost: min_reduced_cost.unwrap_or_else(BigDecimal::zero),
            exact_pricing_complete,
            interrupted,
            truncated,
            truncation_reason,
            statistics,
        })
    }

    fn backtrack_route(
        &self,
        graph: &ExactBigDecimalPricingGraph,
        labels: &[ExactLabel],
        end_label_index: usize,
        request: &ExactBigDecimalPricingRequest,
    ) -> Result<Route<BigDecimal>> {
        let mut label_indices = Vec::new();
        let mut arc_indices = Vec::new();
        let mut current = Some(end_label_index);
        while let Some(index) = current {
            let label = labels.get(index).ok_or_else(|| {
                NetworkSchedulingError::contract(
                    "BigDecimal 回溯标签索引无效 / BigDecimal backtracking label index is invalid",
                )
            })?;
            label_indices.push(index);
            if let Some(arc_index) = label.predecessor_arc {
                arc_indices.push(arc_index);
            }
            current = label.predecessor;
        }
        label_indices.reverse();
        arc_indices.reverse();
        if label_indices.len() != arc_indices.len() + 1 {
            return Err(NetworkSchedulingError::contract(
                "BigDecimal 前驱链长度不一致 / BigDecimal predecessor chain length is inconsistent",
            ));
        }
        let mut total_distance = BigDecimal::zero();
        let mut total_cost = BigDecimal::zero();
        let mut stops = Vec::with_capacity(label_indices.len());
        for (position, label_index) in label_indices.iter().copied().enumerate() {
            let label = labels.get(label_index).ok_or_else(|| {
                NetworkSchedulingError::contract(
                    "BigDecimal 路线标签不存在 / BigDecimal route label does not exist",
                )
            })?;
            let node = graph.nodes.get(label.current_node).ok_or_else(|| {
                NetworkSchedulingError::structure(
                    "BigDecimal 路线节点不存在 / BigDecimal route node does not exist",
                )
            })?;
            let arrival = if position == 0 {
                label.time.clone()
            } else {
                let previous = labels.get(label_indices[position - 1]).ok_or_else(|| {
                    NetworkSchedulingError::contract(
                        "BigDecimal 路线前驱不存在 / BigDecimal route predecessor does not exist",
                    )
                })?;
                &previous.time
                    + duration_value(
                        &request.instance,
                        graph.arcs[arc_indices[position - 1]].travel_time,
                    )?
            };
            let service_start = if arrival > node.ready_time {
                arrival.clone()
            } else {
                node.ready_time.clone()
            };
            if position > 0 {
                let arc = graph.arcs.get(arc_indices[position - 1]).ok_or_else(|| {
                    NetworkSchedulingError::structure(
                        "BigDecimal 路线弧不存在 / BigDecimal route arc does not exist",
                    )
                })?;
                total_distance += &arc.distance;
                total_cost += &arc.route_cost;
            }
            let arrival_instant = instant_value_to_time(&request.instance, &arrival)?;
            let service_instant = instant_value_to_time(&request.instance, &service_start)?;
            let departure_instant = instant_value_to_time(&request.instance, &label.time)?;
            stops.push(RouteStop {
                node_id: node.node_id.clone(),
                customer_id: node
                    .customer_index
                    .and_then(|index| request.instance.customers.get(index))
                    .map(|customer| customer.id.clone()),
                arrival: arrival_instant,
                service_start: service_instant,
                departure: departure_instant,
                accumulated_load: Quantity::new(
                    label.load.clone(),
                    request.instance.units.load_unit.clone(),
                ),
            });
        }
        Route::with_arc_ids(
            request.vehicle_type_id.clone(),
            stops,
            arc_indices
                .iter()
                .map(|index| graph.arcs[*index].arc_id.clone())
                .collect(),
            Quantity::new(total_distance, request.instance.units.distance_unit.clone()),
            Quantity::new(total_cost, request.instance.units.cost_unit.clone()),
        )
    }

    fn validate_backtracked_route(
        &self,
        route: &Route<BigDecimal>,
        request: &ExactBigDecimalPricingRequest,
    ) -> Result<()> {
        let distance_calculator = EuclideanDistanceCalculator;
        let travel_time_calculator =
            DistanceAsTravelTimeCalculator::new(EuclideanDistanceCalculator);
        let arc_cost_calculator = DistanceArcCostCalculator;
        let route_cost_policy = FixedPlusArcCostPolicy;
        let arc_feasibility_policy: &dyn ArcFeasibilityPolicy<BigDecimal> =
            &DefaultArcFeasibilityPolicy;
        RouteValidator::validate_with_policy(
            &request.instance,
            route,
            RouteValidationPolicy {
                distance_calculator: &distance_calculator,
                travel_time_calculator: &travel_time_calculator,
                arc_cost_calculator: &arc_cost_calculator,
                route_cost_policy: &route_cost_policy,
                arc_feasibility_policy,
                branch_mask: request.branch_mask.as_ref(),
            },
        )
    }
}

fn validate_reduced_cost_snapshot(
    graph: &ExactBigDecimalPricingGraph,
    request: &ExactBigDecimalPricingRequest,
) -> Result<()> {
    for (index, arc) in graph.arcs.iter().enumerate() {
        let from = graph.nodes.get(arc.from).ok_or_else(|| {
            NetworkSchedulingError::structure(
                "BigDecimal 定价图弧起点索引无效 / BigDecimal pricing graph arc origin index is invalid",
            )
        })?;
        let to = graph.nodes.get(arc.to).ok_or_else(|| {
            NetworkSchedulingError::structure(
                "BigDecimal 定价图弧终点索引无效 / BigDecimal pricing graph arc destination index is invalid",
            )
        })?;
        let customer_dual = to
            .customer_index
            .and_then(|customer_index| request.instance.customers.get(customer_index))
            .map(|customer| graph.duals.customer_dual(&customer.id))
            .unwrap_or_else(BigDecimal::zero);
        let fleet_dual = if from.is_start_depot {
            graph.duals.fleet_dual(&request.vehicle_type_id)
        } else {
            BigDecimal::zero()
        };
        let phase_cost = match graph.duals.phase {
            PricingPhase::PhaseOne => BigDecimal::zero(),
            PricingPhase::PhaseTwo => arc.route_cost.clone(),
        };
        let expected = phase_cost - customer_dual - fleet_dual;
        if arc.reduced_cost != expected {
            return Err(NetworkSchedulingError::pricing(format!(
                "BigDecimal 定价图弧 {} 的 reduced cost 快照无效 / BigDecimal reduced-cost snapshot for pricing arc {} is invalid",
                index, index
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ExactLabel {
    reduced_cost: BigDecimal,
    time: BigDecimal,
    load: BigDecimal,
    current_node: usize,
    visited: crate::domain::route_generation::VisitedCustomers,
    forbidden: ForbiddenCustomers,
    predecessor: Option<usize>,
    predecessor_arc: Option<usize>,
}

fn dominates(a: &ExactLabel, b: &ExactLabel) -> bool {
    a.current_node == b.current_node
        && a.reduced_cost <= b.reduced_cost
        && a.time <= b.time
        && a.load <= b.load
        && a.visited.is_subset_of(&b.visited)
        // 只有限制更少的标签才能支配限制更多的标签。
        // Only a label with fewer forbidden customers may dominate a more restricted label.
        && a.forbidden.is_subset_of(&b.forbidden)
        && (a.reduced_cost < b.reduced_cost || a.time < b.time || a.load < b.load)
}

fn update_forbidden_customers(
    current_forbidden: &ForbiddenCustomers,
    graph: &ExactBigDecimalPricingGraph,
    current_node: usize,
    visited: &crate::domain::route_generation::VisitedCustomers,
    departure_time: &BigDecimal,
    load: &BigDecimal,
    instance: &VrptwInstance<BigDecimal>,
) -> Result<(ForbiddenCustomers, usize)> {
    let mut forbidden = current_forbidden.clone();
    let mut markings = 0;
    if let Some(outgoing) = graph.outgoing.get(current_node) {
        for arc_index in outgoing {
            let arc = graph.arcs.get(*arc_index).ok_or_else(|| {
                NetworkSchedulingError::structure(
                    "BigDecimal 不可达标记弧索引无效 / BigDecimal Feillet marking arc index is invalid",
                )
            })?;
            let candidate = graph.nodes.get(arc.to).ok_or_else(|| {
                NetworkSchedulingError::structure(
                    "BigDecimal 不可达标记节点不存在 / BigDecimal Feillet marking node does not exist",
                )
            })?;
            let Some(index) = candidate.customer_index else {
                continue;
            };
            if visited.contains(index) || forbidden.contains(index) {
                continue;
            }
            let arrival = departure_time + duration_value(instance, arc.travel_time)?;
            let service_start = if arrival > candidate.ready_time {
                arrival
            } else {
                candidate.ready_time.clone()
            };
            if service_start > candidate.due_time
                || load + &candidate.demand > graph.vehicle_capacity
            {
                forbidden = forbidden.add(index);
                markings += 1;
            }
        }
    }
    Ok((forbidden, markings))
}

fn instant_value(
    instance: &VrptwInstance<BigDecimal>,
    instant: OffsetDateTime,
) -> Result<BigDecimal> {
    let duration = instant - instance.scheduling_window.window.start;
    Ok(decimal_duration(
        duration,
        instance.scheduling_window.duration_unit,
    ))
}

fn duration_value(instance: &VrptwInstance<BigDecimal>, duration: Duration) -> Result<BigDecimal> {
    if duration < Duration::ZERO {
        return Err(NetworkSchedulingError::validation(
            "持续时间不能为负 / duration cannot be negative",
        ));
    }
    Ok(decimal_duration(
        duration,
        instance.scheduling_window.duration_unit,
    ))
}

fn decimal_duration(duration: Duration, unit: DurationUnit) -> BigDecimal {
    let denominator = match unit {
        DurationUnit::Seconds => 1_000_000_000_i128,
        DurationUnit::Minutes => 60_000_000_000_i128,
        DurationUnit::Hours => 3_600_000_000_000_i128,
    };
    BigDecimal::from(duration.whole_nanoseconds()) / BigDecimal::from(denominator)
}

fn instant_value_to_time(
    instance: &VrptwInstance<BigDecimal>,
    value: &BigDecimal,
) -> Result<OffsetDateTime> {
    let denominator = match instance.scheduling_window.duration_unit {
        DurationUnit::Seconds => 1_000_000_000_i128,
        DurationUnit::Minutes => 60_000_000_000_i128,
        DurationUnit::Hours => 3_600_000_000_000_i128,
    };
    let requested_nanos = value * BigDecimal::from(denominator);
    let nanos = requested_nanos
        .to_i128()
        .ok_or_else(|| NetworkSchedulingError::Conversion {
        message: format!(
            "BigDecimal 时间无法表示为纳秒：{} / BigDecimal time cannot be represented as nanoseconds: {}",
            value, value
        ),
        })?;
    let nanos_decimal = BigDecimal::from(nanos);
    if nanos_decimal != requested_nanos {
        return Err(NetworkSchedulingError::Conversion {
            message: format!(
                "BigDecimal 时间不是整数纳秒：{} / BigDecimal time is not an integral nanosecond: {}",
                value, value
            ),
        });
    }
    let nanos = i64::try_from(nanos).map_err(|error| NetworkSchedulingError::Conversion {
        message: format!(
            "BigDecimal 时间超出 time::Duration 范围：{} / BigDecimal time exceeds time::Duration range: {}",
            error, error
        ),
    })?;
    instance
        .scheduling_window
        .window
        .start
        .checked_add(Duration::nanoseconds(nanos))
        .ok_or_else(|| {
            NetworkSchedulingError::Conversion {
                message: "BigDecimal 时间超出 OffsetDateTime 范围 / BigDecimal time exceeds OffsetDateTime range".to_owned(),
            }
        })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::borrow_interior_mutable_const)]

    use std::collections::{BTreeMap, BTreeSet};
    use std::str::FromStr;

    use super::{
        ExactBigDecimalEspprcPricer, ExactBigDecimalPricingDuals, ExactBigDecimalPricingRequest,
        ExactBigDecimalRouteGraphBuilder,
    };
    use crate::domain::vrp::{
        BranchMask, Customer, CustomerId, Depot, PricingPhase, ResourceArc, ServiceTimeWindow,
        TravelTime, VehicleType, VehicleTypeId, VrptwArc, VrptwInstance, VrptwUnits,
        coordinate_node,
    };
    use crate::infrastructure::NetworkNodeId;
    use ospf_rust_framework_gantt_scheduling::infrastructure::{TimeRange, TimeWindow};
    use ospf_rust_quantities::Quantity;
    use ospf_rust_quantities::unit::{CTUnit, Kilogram, Meter};
    use time::{Duration, OffsetDateTime};

    fn decimal(value: &str) -> bigdecimal::BigDecimal {
        bigdecimal::BigDecimal::from_str(value).expect("valid decimal")
    }

    #[test]
    fn exact_pricer_keeps_decimal_reduced_cost_and_route_cost_without_f64() {
        let start = OffsetDateTime::UNIX_EPOCH;
        let end = start + Duration::hours(1);
        let service_window = ServiceTimeWindow::new(start, end).expect("valid service window");
        let units = VrptwUnits::default();
        let start_node =
            coordinate_node("start", decimal("0"), decimal("0"), Meter::INSTANT.clone())
                .expect("start node");
        let customer_node = coordinate_node(
            "customer-node",
            decimal("1"),
            decimal("0"),
            Meter::INSTANT.clone(),
        )
        .expect("customer node");
        let end_node = coordinate_node("end", decimal("0"), decimal("0"), Meter::INSTANT.clone())
            .expect("end node");
        let customer = Customer::new(
            CustomerId::from("c1"),
            customer_node,
            Quantity::new(decimal("1"), Kilogram::INSTANT.clone()),
            service_window,
            Duration::ZERO,
        )
        .expect("customer");
        let vehicle = VehicleType::new(
            VehicleTypeId::from("v1"),
            Quantity::new(decimal("2"), Kilogram::INSTANT.clone()),
            Quantity::new(
                decimal("0.1111111111111111111111111111"),
                units.cost_unit.clone(),
            ),
            1,
        )
        .expect("vehicle");
        let travel = TravelTime::new(Duration::seconds(1)).expect("travel");
        let arcs = vec![
            VrptwArc::new(
                "s-c",
                NetworkNodeId::from("start"),
                NetworkNodeId::from("customer-node"),
                Quantity::new(
                    decimal("0.1234567890123456789012345678"),
                    Meter::INSTANT.clone(),
                ),
                travel,
                Quantity::new(
                    decimal("0.2222222222222222222222222222"),
                    units.cost_unit.clone(),
                ),
            )
            .expect("start arc"),
            VrptwArc::new(
                "c-e",
                NetworkNodeId::from("customer-node"),
                NetworkNodeId::from("end"),
                Quantity::new(
                    decimal("0.3333333333333333333333333333"),
                    Meter::INSTANT.clone(),
                ),
                travel,
                Quantity::new(
                    decimal("0.4444444444444444444444444444"),
                    units.cost_unit.clone(),
                ),
            )
            .expect("end arc"),
        ];
        let instance = std::sync::Arc::new(
            VrptwInstance::new_with_arcs(
                "exact-decimal",
                Depot {
                    node: start_node,
                    time_window: service_window,
                },
                Depot {
                    node: end_node,
                    time_window: service_window,
                },
                vec![customer],
                vec![vehicle],
                TimeWindow::seconds(
                    TimeRange::new(start, end),
                    decimal("0"),
                    false,
                    decimal("1"),
                ),
                units,
                Default::default(),
                arcs,
            )
            .expect("exact instance"),
        );
        let vehicle_type_id = VehicleTypeId::from("v1");
        let customer_dual = decimal("3.3333333333333333333333333333");
        let fleet_dual = decimal("0.0111111111111111111111111111");
        let duals = ExactBigDecimalPricingDuals::new(
            PricingPhase::PhaseTwo,
            [(CustomerId::from("c1"), customer_dual.clone())],
            [(vehicle_type_id.clone(), fleet_dual.clone())],
        );
        let pricer = ExactBigDecimalEspprcPricer::new(ExactBigDecimalRouteGraphBuilder::new(
            instance.clone(),
        ));
        let request = ExactBigDecimalPricingRequest::new(
            instance.clone(),
            duals.clone(),
            vehicle_type_id.clone(),
        );
        let graph = pricer
            .graph_builder
            .build(&vehicle_type_id, &duals, None)
            .expect("unmasked exact graph");
        let mut mismatched_request = request.clone();
        mismatched_request.branch_mask = Some(
            BranchMask::empty(
                instance.start_depot.node.id.clone(),
                instance.end_depot.node.id.clone(),
            )
            .expect("empty branch mask"),
        );
        assert!(pricer.price(&graph, &mismatched_request).is_err());

        let required_arc = ResourceArc::new(
            vehicle_type_id.clone(),
            "different-start-arc".into(),
            instance.start_depot.node.id.clone(),
            instance.customers[0].node.id.clone(),
        );
        let defensive_mask = BranchMask::new(
            instance.start_depot.node.id.clone(),
            instance.end_depot.node.id.clone(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            [required_arc].into_iter().collect(),
        )
        .expect("defensive branch mask");
        let mut corrupted_graph = graph.clone();
        corrupted_graph.branch_mask = Some(defensive_mask.clone());
        let mut defensive_request = request.clone();
        defensive_request.branch_mask = Some(defensive_mask);
        assert!(pricer.price(&corrupted_graph, &defensive_request).is_err());

        let mut mismatched_duals_request = request.clone();
        mismatched_duals_request.duals.customer.insert(
            CustomerId::from("c1"),
            decimal("4.3333333333333333333333333333"),
        );
        assert!(pricer.price(&graph, &mismatched_duals_request).is_err());

        let mut stale_instance = (*instance).clone();
        stale_instance.name.push_str("-mutated");
        let mut stale_instance_request = request.clone();
        stale_instance_request.instance = std::sync::Arc::new(stale_instance);
        assert!(pricer.price(&graph, &stale_instance_request).is_err());

        let mut corrupted_reduced_cost = graph.clone();
        corrupted_reduced_cost.arcs[0].reduced_cost += decimal("1");
        assert!(pricer.price(&corrupted_reduced_cost, &request).is_err());

        let mut corrupted_outgoing = graph.clone();
        corrupted_outgoing.outgoing[graph.start_index().expect("start node")].clear();
        assert!(pricer.price(&corrupted_outgoing, &request).is_err());

        let mut corrupted_cost_snapshot = graph.clone();
        corrupted_cost_snapshot.arcs[0].route_cost += decimal("1");
        corrupted_cost_snapshot.arcs[0].reduced_cost += decimal("1");
        assert!(pricer.price(&corrupted_cost_snapshot, &request).is_err());

        let mut corrupted_resources = graph.clone();
        corrupted_resources.arcs[0].travel_time = Duration::seconds(99);
        assert!(
            pricer.price(&corrupted_resources, &request).is_err(),
            "backtracking must independently replay travel time"
        );

        let result = pricer.price_request(&request).expect("exact pricing");
        let expected = decimal("0.1111111111111111111111111111")
            + decimal("0.2222222222222222222222222222")
            + decimal("0.4444444444444444444444444444")
            - customer_dual
            - fleet_dual;
        assert_eq!(result.min_reduced_cost, expected);
        assert_eq!(result.routes.len(), 1);
        assert_eq!(
            result.routes[0].cost.value,
            decimal("0.7777777777777777777777777777")
        );
        assert!(result.exact_pricing_complete);
        assert_eq!(result.statistics.unreachable_markings, 0);

        let (forbidden, markings) = super::update_forbidden_customers(
            &crate::domain::route_generation::ForbiddenCustomers::empty(1),
            &graph,
            graph.start_index().expect("start node"),
            &crate::domain::route_generation::VisitedCustomers::empty(1),
            &decimal("0"),
            &graph.vehicle_capacity,
            &instance,
        )
        .expect("capacity marking");
        assert_eq!(markings, 1);
        assert!(forbidden.contains(0));
    }
}
