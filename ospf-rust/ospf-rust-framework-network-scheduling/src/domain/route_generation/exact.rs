//! BigDecimal 精确定价路径 / BigDecimal exact pricing path.
//!
//! 该实现只接受显式基础弧。显式弧的距离、成本和时间由实例直接提供，避免默认
//! 欧氏距离以及 `TimeWindow` 的 f64 适配器破坏精度。solver 对偶仍由调用方以
//! BigDecimal 提供，整个 ESPPRC 标签扩展不经过 f64。
//! This implementation accepts explicit base arcs only. Distances, costs, and travel
//! times come directly from the instance, avoiding the f64 adapters used by default
//! Euclidean and `TimeWindow` policies. Solver duals are supplied as BigDecimal and
//! the complete ESPPRC label extension stays outside f64.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;
use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use time::{Duration, OffsetDateTime};
use ospf_rust_quantities::Quantity;
use ospf_rust_framework_gantt_scheduling::infrastructure::DurationUnit;
use crate::domain::path::{PathProblem, PathSearchBudget, PathSearchCompletion, PathSearchContext};
use crate::domain::route_generation::path_search::{self, PricingLabel, PricingPathPolicy};
use crate::domain::route_generation::{
    CancellationToken, ForbiddenCustomers, LabelStatistics, TruncationReason,
};
use crate::domain::vrp::{
    ArcFeasibilityPolicy, BranchMask, CustomerId, DefaultArcFeasibilityPolicy,
    DistanceArcCostCalculator, DistanceAsTravelTimeCalculator, EuclideanDistanceCalculator,
    FixedPlusArcCostPolicy, PricingPhase, Route, RouteCostPolicy, RouteStop, RouteValidationPolicy, RouteValidator,
    RoutePricingArc, RoutePricingRows, VehicleTypeId, VrptwInstance,
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
    /// 声明行单位下的逐弧精确系数 / Exact incremental coefficients in declared row units.
    pub row_coefficients: BTreeMap<String, BigDecimal>,
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
    /// 全部声明扩展行的有符号价格，零也须显式存在 / Signed prices for every declared extension row, including explicit zeros.
    pub extension: BTreeMap<String, BigDecimal>,
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
            extension: BTreeMap::new(),
        }
    }

    /// 附加精确扩展行价格 / Attach exact extension-row prices.
    pub fn with_extension_duals(mut self, prices: impl IntoIterator<Item = (String, BigDecimal)>) -> Self {
        self.extension = prices.into_iter().collect();
        self
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
pub struct ExactBigDecimalRouteGraphBuilder<C = FixedPlusArcCostPolicy> {
    /// VRPTW 实例 / VRPTW instance.
    pub instance: Arc<VrptwInstance<BigDecimal>>,
    /// 完整路线成本策略及可加性认证 / Complete route-cost policy and additive certification.
    pub route_cost_policy: C,
    /// 主问题与定价共用的声明行提供器 / Declared-row providers shared by masters and pricing.
    pub pricing_rows: Arc<RoutePricingRows<BigDecimal>>,
}

impl ExactBigDecimalRouteGraphBuilder {
    /// 创建精确图构建器 / Create an exact graph builder.
    pub fn new(instance: Arc<VrptwInstance<BigDecimal>>) -> Self {
        Self::with_cost_policy(instance, FixedPlusArcCostPolicy)
    }
}

impl<C: RouteCostPolicy<BigDecimal>> ExactBigDecimalRouteGraphBuilder<C> {
    /// 指定完整费用策略；未经认证时禁用标签费用支配 / Set whole-route costs; disable label-cost dominance unless certified.
    pub fn with_cost_policy(instance: Arc<VrptwInstance<BigDecimal>>, route_cost_policy: C) -> Self {
        Self { instance, route_cost_policy, pricing_rows: Arc::new(RoutePricingRows::default()) }
    }

    /// 设置共享声明行目录 / Set the shared declared-row catalog.
    pub fn with_pricing_rows(mut self, rows: Arc<RoutePricingRows<BigDecimal>>) -> Self {
        self.pricing_rows = rows;
        self
    }

    /// 构建精确定价图 / Build an exact pricing graph.
    pub fn build(
        &self,
        vehicle_type_id: &VehicleTypeId,
        duals: &ExactBigDecimalPricingDuals,
        branch_mask: Option<&BranchMask<VehicleTypeId>>,
    ) -> Result<ExactBigDecimalPricingGraph> {
        self.pricing_rows.validate_dual_keys(&duals.extension)?;
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
            let row_coefficients = self.pricing_rows.arc_coefficients(&RoutePricingArc {
                instance: &self.instance, vehicle_type: vehicle_type_id, arc: &base_arc.id,
                from: &base_arc.from, to: &base_arc.to,
            })?;
            let row_price: BigDecimal = row_coefficients.iter().map(|(id, value)| value * &duals.extension[id]).sum();
            arcs.push(ExactBigDecimalPricingArc {
                arc_id: base_arc.id.clone(),
                from,
                to,
                distance,
                travel_time: base_arc.travel_time.duration,
                arc_cost,
                route_cost,
                reduced_cost: phase_cost - customer_dual - fleet_dual - row_price,
                row_coefficients,
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
pub struct ExactBigDecimalEspprcPricer<C = FixedPlusArcCostPolicy> {
    /// 显式弧图构建器 / Explicit-arc graph builder.
    pub graph_builder: ExactBigDecimalRouteGraphBuilder<C>,
}

impl<C: RouteCostPolicy<BigDecimal>> ExactBigDecimalEspprcPricer<C> {
    /// 创建精确定价器 / Create an exact pricer.
    pub fn new(graph_builder: ExactBigDecimalRouteGraphBuilder<C>) -> Self {
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
        self.graph_builder.pricing_rows.validate_dual_keys(&request.duals.extension)?;
        for arc in &graph.arcs {
            let coefficients = self.graph_builder.pricing_rows.arc_coefficients(&RoutePricingArc {
                instance: &request.instance, vehicle_type: &request.vehicle_type_id, arc: &arc.arc_id,
                from: &graph.nodes[arc.from].node_id, to: &graph.nodes[arc.to].node_id,
            })?;
            if coefficients != arc.row_coefficients {
                return Err(NetworkSchedulingError::pricing("扩展行提供器与图快照不一致 / extension-row provider disagrees with graph snapshot"));
            }
        }
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

        let depth_limited = Cell::new(false);
        let unreachable = Cell::new(0);
        let minimum: RefCell<Option<BigDecimal>> = RefCell::new(None);
        let root = ExactLabel {
            reduced_cost: BigDecimal::zero(), time: graph.nodes[start_index].ready_time.clone(),
            load: BigDecimal::zero(), current_node: start_index,
            visited: crate::domain::route_generation::VisitedCustomers::empty(customer_count),
            forbidden: ForbiddenCustomers::empty(customer_count), predecessor: None, predecessor_arc: None,
        };
        let extend = |index: usize, label: &ExactLabel, arc_index: usize, _: usize| -> Result<Option<ExactLabel>> {
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
            let arrival = &label.time + duration_value(&request.instance, arc.travel_time)?;
            let service_start = arrival.max(node.ready_time.clone());
            let load = &label.load + &node.demand;
            if service_start > node.due_time || load > graph.vehicle_capacity { return Ok(None); }
            let visited = node.customer_index.map_or_else(|| label.visited.clone(), |index| label.visited.add(index));
            let (forbidden, markings) = update_forbidden_customers(&label.forbidden, graph, arc.to, &visited, &load)?;
            unreachable.set(unreachable.get() + markings);
            Ok(Some(ExactLabel {
                reduced_cost: &label.reduced_cost + &arc.reduced_cost,
                time: service_start + &node.service_time, load, current_node: arc.to, visited, forbidden,
                predecessor: Some(index), predecessor_arc: Some(arc_index),
            }))
        };
        let finish = |labels: &[ExactLabel]| -> Result<Option<(ExactLabel, Route<BigDecimal>)>> {
            let route = self.backtrack_route(graph, labels, labels.len() - 1, request)?;
            self.validate_backtracked_route(&route, request)?;
            let cost = match request.duals.phase {
                PricingPhase::PhaseOne => BigDecimal::zero(),
                PricingPhase::PhaseTwo => route.cost.value.clone(),
            };
            let coefficients = self.graph_builder.pricing_rows.route_coefficients(&route, &request.instance)?;
            let row_price: BigDecimal = coefficients.iter().map(|(id, value)| value * &request.duals.extension[id]).sum();
            let reduced_cost = cost - request.duals.fleet_dual(&request.vehicle_type_id)
                - route.customer_ids().iter().map(|id| request.duals.customer_dual(id)).sum::<BigDecimal>() - row_price;
            let mut current_minimum = minimum.borrow_mut();
            if current_minimum.as_ref().is_none_or(|current| reduced_cost < *current) {
                *current_minimum = Some(reduced_cost.clone());
            }
            Ok((reduced_cost < -&request.pricing_tolerance).then(|| {
                let mut label = labels[labels.len() - 1].clone();
                label.reduced_cost = reduced_cost;
                (label, route)
            }))
        };
        let compare = |left: &ExactLabel, a: &Route<BigDecimal>, right: &ExactLabel, b: &Route<BigDecimal>|
            left.reduced_cost.cmp(&right.reduced_cost).then_with(|| a.signature().cmp(&b.signature()));
        let graph_view = path_search::graph(graph.nodes.len(), graph.arcs.iter().map(|arc| (arc.arc_id.clone(), arc.from, arc.to)))?;
        let policy = PricingPathPolicy { root, extend: &extend, finish: &finish,
            dominates: (self.graph_builder.route_cost_policy.additive_label_cost_is_exact()
                && self.graph_builder.pricing_rows.additive_decomposition_is_exact()).then_some(&dominates), compare: &compare };
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
        let mut routes: Vec<_> = search.paths.into_iter().filter_map(|candidate| candidate.state.route).collect();
        let selected_count = routes.len().min(request.max_columns_per_pricing);
        let truncation_reason = if search.completion == PathSearchCompletion::BudgetLimit { Some(TruncationReason::MaxLabels) }
            else if depth_limited.get() { Some(TruncationReason::MaxDepth) }
            else if selected_count < routes.len() { Some(TruncationReason::MaxColumns) }
            else { None };
        routes.truncate(selected_count);
        Ok(ExactBigDecimalPricingResult {
            routes, min_reduced_cost: minimum.into_inner().unwrap_or_else(BigDecimal::zero),
            exact_pricing_complete, interrupted, truncated: truncation_reason.is_some(), truncation_reason, statistics,
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
        let mut arc_costs = Vec::with_capacity(arc_indices.len());
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
                arc_costs.push(Quantity::new(arc.arc_cost.clone(), request.instance.units.cost_unit.clone()));
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
        let vehicle = request.instance.vehicle_type(&request.vehicle_type_id).ok_or_else(||
            NetworkSchedulingError::validation("路线车辆类型不存在 / route vehicle type does not exist"))?;
        let cost = self.graph_builder.route_cost_policy.cost(vehicle, &arc_costs, &request.instance)?;
        crate::infrastructure::validate_linear_units([&cost.unit, &request.instance.units.cost_unit])?;
        let cost = cost.to_unit(&request.instance.units.cost_unit).map_err(|error|
            NetworkSchedulingError::Conversion { message: error.to_string() })?;
        Route::with_arc_ids(
            request.vehicle_type_id.clone(),
            stops,
            arc_indices
                .iter()
                .map(|index| graph.arcs[*index].arc_id.clone())
                .collect(),
            Quantity::new(total_distance, request.instance.units.distance_unit.clone()),
            cost,
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
        let arc_feasibility_policy: &dyn ArcFeasibilityPolicy<BigDecimal> =
            &DefaultArcFeasibilityPolicy;
        RouteValidator::validate_with_policy(
            &request.instance,
            route,
            RouteValidationPolicy {
                distance_calculator: &distance_calculator,
                travel_time_calculator: &travel_time_calculator,
                arc_cost_calculator: &arc_cost_calculator,
                route_cost_policy: &self.graph_builder.route_cost_policy,
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
        if arc.row_coefficients.len() != request.duals.extension.len() {
            return Err(NetworkSchedulingError::pricing("图扩展行与对偶域不一致 / graph extension rows disagree with dual domain"));
        }
        let mut row_price = BigDecimal::zero();
        for (id, value) in &arc.row_coefficients {
            let price = request.duals.extension.get(id).ok_or_else(|| NetworkSchedulingError::pricing(
                "图扩展行缺少对偶 / graph extension row lacks a dual"))?;
            row_price += value * price;
        }
        let expected = phase_cost - customer_dual - fleet_dual - row_price;
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

impl PricingLabel for ExactLabel {
    fn set_predecessor(&mut self, predecessor: Option<usize>, arc: Option<usize>) {
        self.predecessor = predecessor;
        self.predecessor_arc = arc;
    }
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
    load: &BigDecimal,
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
            // 与普通入口一致，不从直达超窗推导永久不可达。 / Match ordinary pricing: direct-window failure does not prove permanent unreachability.
            if load + &candidate.demand > graph.vehicle_capacity {
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

    mod pricing_rows;

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

    fn precise_instance() -> std::sync::Arc<VrptwInstance<bigdecimal::BigDecimal>> {
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
        std::sync::Arc::new(
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
        )
    }

    #[test]
    fn exact_pricer_keeps_decimal_reduced_cost_and_route_cost_without_f64() {
        let instance = precise_instance();
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
            &graph.vehicle_capacity,
        )
        .expect("capacity marking");
        assert_eq!(markings, 1);
        assert!(forbidden.contains(0));
    }

    fn parallel_instance() -> std::sync::Arc<VrptwInstance<bigdecimal::BigDecimal>> {
        let mut instance = (*precise_instance()).clone();
        instance.arcs.push(VrptwArc::new(
            "a-expensive", "start", "customer-node",
            Quantity::new(decimal("2"), Meter::INSTANT.clone()),
            TravelTime::new(Duration::seconds(2)).unwrap(),
            Quantity::new(decimal("10"), instance.units.cost_unit.clone()),
        ).unwrap());
        std::sync::Arc::new(instance)
    }

    struct WholeCosts { discount: bool }

    impl crate::domain::vrp::RouteCostPolicy<bigdecimal::BigDecimal> for WholeCosts {
        fn cost(
            &self,
            vehicle: &VehicleType<bigdecimal::BigDecimal>,
            arcs: &[Quantity<bigdecimal::BigDecimal, ospf_rust_quantities::unit::Unit>],
            instance: &VrptwInstance<bigdecimal::BigDecimal>,
        ) -> crate::Result<Quantity<bigdecimal::BigDecimal, ospf_rust_quantities::unit::Unit>> {
            if !self.discount {
                return crate::domain::vrp::RouteCostPolicy::cost(&crate::domain::vrp::FixedPlusArcCostPolicy, vehicle, arcs, instance);
            }
            let total: bigdecimal::BigDecimal = arcs.iter().map(|arc| arc.value.clone()).sum();
            Ok(Quantity::new(if total > decimal("1") { decimal("0.1") } else { decimal("100") }, instance.units.cost_unit.clone()))
        }
    }

    #[test]
    fn exact_whole_route_cost_controls_filtering_order_and_both_phases() {
        let instance = parallel_instance();
        for discount in [false, true] {
            let pricer = ExactBigDecimalEspprcPricer::new(ExactBigDecimalRouteGraphBuilder::with_cost_policy(
                instance.clone(), WholeCosts { discount },
            ));
            for phase in [PricingPhase::PhaseOne, PricingPhase::PhaseTwo] {
                let dual = if discount { decimal("1") } else { decimal("20") };
                let mut request = ExactBigDecimalPricingRequest::new(instance.clone(), ExactBigDecimalPricingDuals::new(
                    phase, [("c1".into(), dual)], [("v1".into(), decimal("0"))],
                ), "v1".into());
                let result = pricer.price_request(&request).unwrap();
                assert!(result.exact_pricing_complete);
                assert_eq!(result.statistics.dominance_checks, 0);
                if phase == PricingPhase::PhaseTwo {
                    assert_eq!(result.routes[0].arc_ids[0].as_str(), if discount { "a-expensive" } else { "s-c" });
                    assert_eq!(result.routes.len(), if discount { 1 } else { 2 });
                    assert_eq!(result.min_reduced_cost, if discount { decimal("-0.9") }
                        else { decimal("-19.2222222222222222222222222223") });
                } else {
                    assert_eq!(result.routes.len(), 2);
                    assert_eq!(result.min_reduced_cost, if discount { decimal("-1") } else { decimal("-20") });
                }
                request.max_columns_per_pricing = 0;
                let limited = pricer.price_request(&request).unwrap();
                assert!(limited.routes.is_empty());
                assert!(limited.exact_pricing_complete);
                assert_eq!(limited.min_reduced_cost, result.min_reduced_cost);
                assert_eq!(limited.truncation_reason, Some(crate::domain::route_generation::TruncationReason::MaxColumns));
            }
        }
    }

    #[test]
    fn exact_sub_f64_improvement_and_zero_budget_remain_distinct() {
        let instance = precise_instance();
        let pricer = ExactBigDecimalEspprcPricer::new(ExactBigDecimalRouteGraphBuilder::new(instance.clone()));
        let epsilon = decimal("0.0000000000000000000000000000000000000001");
        let mut request = ExactBigDecimalPricingRequest::new(instance, ExactBigDecimalPricingDuals::new(
            PricingPhase::PhaseTwo, [("c1".into(), decimal("0.7777777777777777777777777777") + &epsilon)],
            [("v1".into(), decimal("0"))],
        ), "v1".into());
        let result = pricer.price_request(&request).unwrap();
        assert_eq!(result.min_reduced_cost, -&epsilon);
        assert_eq!(result.routes.len(), 1);
        assert!(result.exact_pricing_complete);
        request.max_labels = Some(0);
        let result = pricer.price_request(&request).unwrap();
        assert!(result.routes.is_empty());
        assert_eq!(result.statistics.labels_created, 0);
        assert!(!result.exact_pricing_complete);
        assert_eq!(result.truncation_reason, Some(crate::domain::route_generation::TruncationReason::MaxLabels));
    }

    #[test]
    fn exact_budget_preserves_an_admitted_route_without_claiming_exhaustion() {
        let instance = parallel_instance();
        let pricer = ExactBigDecimalEspprcPricer::new(ExactBigDecimalRouteGraphBuilder::with_cost_policy(
            instance.clone(), WholeCosts { discount: false },
        ));
        let mut request = ExactBigDecimalPricingRequest::new(instance, ExactBigDecimalPricingDuals::new(
            PricingPhase::PhaseTwo, [("c1".into(), decimal("20"))], [("v1".into(), decimal("0"))],
        ), "v1".into());
        request.max_labels = Some(4);
        let result = pricer.price_request(&request).unwrap();
        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].arc_ids[0].as_str(), "s-c");
        assert_eq!(result.statistics.labels_created, 4);
        assert!(!result.exact_pricing_complete);
        assert_eq!(result.truncation_reason, Some(crate::domain::route_generation::TruncationReason::MaxLabels));
        request.max_labels = None;
        request.max_depth = Some(0);
        let result = pricer.price_request(&request).unwrap();
        assert!(!result.exact_pricing_complete);
        assert_eq!(result.truncation_reason, Some(crate::domain::route_generation::TruncationReason::MaxDepth));
        let token = crate::domain::route_generation::CancellationToken::new();
        token.cancel();
        request.cancellation = Some(token);
        let result = pricer.price_request(&request).unwrap();
        assert!(result.interrupted);
        assert!(!result.exact_pricing_complete);
        assert_eq!(result.statistics.labels_created, 0);
    }

    #[test]
    fn exact_shared_search_matches_independent_small_multigraph_oracle() {
        let template = precise_instance();
        let start = template.scheduling_window.window.start;
        let mut customers = template.customers.clone();
        customers.push(Customer::new("c2",
            coordinate_node("c2-node", decimal("2"), decimal("0"), Meter::INSTANT.clone()).unwrap(),
            Quantity::new(decimal("1"), Kilogram::INSTANT.clone()),
            ServiceTimeWindow::new(start, start + Duration::seconds(3)).unwrap(), Duration::ZERO,
        ).unwrap());
        let edges = [("start", "customer-node", 1), ("start", "customer-node", 2),
            ("start", "c2-node", 4), ("customer-node", "c2-node", 1),
            ("c2-node", "customer-node", 1), ("customer-node", "end", 1), ("c2-node", "end", 1)];
        let arcs: Vec<_> = edges.into_iter().enumerate().map(|(index, (from, to, seconds))| VrptwArc::new(
            format!("e{index}"), from, to, Quantity::new(decimal("1"), Meter::INSTANT.clone()),
            TravelTime::new(Duration::seconds(seconds)).unwrap(),
            Quantity::new(decimal("0.10000000000000000000000000001") * bigdecimal::BigDecimal::from(u64::try_from(index + 1).unwrap()), template.units.cost_unit.clone()),
        ).unwrap()).collect();
        // 两客户的全部有序访问及平行弧选择，独立于标签扩展和支配。
        // All ordered two-customer visits and parallel-arc choices, independent of label extension and dominance.
        let paths: [&[usize]; 6] = [&[0, 5], &[1, 5], &[2, 6], &[0, 3, 6], &[1, 3, 6], &[2, 4, 5]];
        for mask in 1_u32..128 {
            let instance = std::sync::Arc::new(VrptwInstance::new_with_arcs(
                "small-exact-oracle", template.start_depot.clone(), template.end_depot.clone(), customers.clone(),
                template.vehicle_types.clone(), template.scheduling_window.clone(), template.units.clone(), template.tolerances,
                arcs.iter().enumerate().filter(|(index, _)| mask & (1 << index) != 0).map(|(_, arc)| arc.clone()).collect(),
            ).unwrap());
            for phase in [PricingPhase::PhaseOne, PricingPhase::PhaseTwo] {
                let mut expected = BTreeMap::new();
                let mut minimum = None;
                for path in paths {
                    if path.iter().any(|index| mask & (1 << index) == 0) { continue; }
                    let mut time = start;
                    let mut cost = instance.vehicle_types[0].fixed_cost.value.clone();
                    let mut dual = decimal("0.1");
                    let mut feasible = true;
                    for &index in path {
                        let arc = &arcs[index];
                        time += arc.travel_time.duration;
                        cost += &arc.cost.value;
                        if let Some(customer) = customers.iter().find(|customer| customer.node.id == arc.to) {
                            time = time.max(customer.time_window.ready_time);
                            if time > customer.time_window.due_time { feasible = false; }
                            time += customer.service_time;
                            dual += if customer.id.as_str() == "c1" { decimal("0.6") } else { decimal("0.8") };
                        }
                    }
                    if !feasible { continue; }
                    let reduced_cost = if phase == PricingPhase::PhaseOne { -dual } else { &cost - dual };
                    if minimum.as_ref().is_none_or(|current| reduced_cost < *current) { minimum = Some(reduced_cost.clone()); }
                    if reduced_cost < decimal("0") {
                        expected.insert(path.iter().map(|&index| arcs[index].id.clone()).collect::<Vec<_>>(), cost);
                    }
                }
                let pricer = ExactBigDecimalEspprcPricer::new(ExactBigDecimalRouteGraphBuilder::with_cost_policy(
                    instance.clone(), WholeCosts { discount: false },
                ));
                let request = ExactBigDecimalPricingRequest::new(instance.clone(), ExactBigDecimalPricingDuals::new(
                    phase, [("c1".into(), decimal("0.6")), ("c2".into(), decimal("0.8"))],
                    [("v1".into(), decimal("0.1"))],
                ), "v1".into());
                let result = pricer.price_request(&request).unwrap();
                assert!(result.exact_pricing_complete, "mask {mask}, phase {phase:?}");
                assert_eq!(result.min_reduced_cost, minimum.unwrap_or_else(|| decimal("0")), "mask {mask}, phase {phase:?}");
                let actual: BTreeMap<_, _> = result.routes.iter().map(|route| (route.effective_arc_ids(), route.cost.value.clone())).collect();
                assert_eq!(actual, expected, "mask {mask}, phase {phase:?}");
                let certified = ExactBigDecimalEspprcPricer::new(ExactBigDecimalRouteGraphBuilder::new(request.instance.clone()))
                    .price_request(&request).unwrap();
                assert!(certified.exact_pricing_complete);
                assert_eq!(certified.min_reduced_cost, result.min_reduced_cost, "certified mask {mask}, phase {phase:?}");
                for route in &certified.routes {
                    assert_eq!(expected.get(&route.effective_arc_ids()), Some(&route.cost.value));
                }
            }
        }
    }
}
