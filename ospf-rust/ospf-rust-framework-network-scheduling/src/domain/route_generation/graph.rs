//! Branch-aware ESPPRC 定价图 / Branch-aware ESPPRC pricing graph.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use time::Duration;

use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::unit::UnitConversionValue;

use crate::domain::vrp::{
    ArcCostCalculator, ArcFeasibilityPolicy, BranchMask, DistanceCalculator, PricingDuals,
    RouteCostPolicy, TravelTimeCalculator, VehicleTypeId, VrpNode, VrptwInstance, default_arc_id,
};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::NetworkNodeId;

/// 定价图节点 / Pricing-graph node.
#[derive(Debug, Clone)]
pub struct PricingNode {
    /// 节点 ID / Node ID.
    pub node_id: NetworkNodeId,
    /// 客户索引；仓库为 `None` / Customer index; `None` for depots.
    pub customer_index: Option<usize>,
    /// 是否起始仓库 / Whether start depot.
    pub is_start_depot: bool,
    /// 是否结束仓库 / Whether end depot.
    pub is_end_depot: bool,
    /// 最早服务开始值 / Earliest service-start value.
    pub ready_time: f64,
    /// 最晚服务开始值 / Latest service-start value.
    pub due_time: f64,
    /// 服务时长值 / Service-duration value.
    pub service_time: f64,
    /// 客户需求值 / Customer demand value.
    pub demand: f64,
}

/// 定价图弧 / Pricing-graph arc.
#[derive(Debug, Clone)]
pub struct PricingArc {
    /// 稳定弧 ID / Stable arc ID.
    pub arc_id: crate::infrastructure::NetworkArcId,
    /// 起点图索引 / Origin graph index.
    pub from: usize,
    /// 终点图索引 / Destination graph index.
    pub to: usize,
    /// 距离值 / Distance value.
    pub distance: f64,
    /// 行驶时间 / Travel duration.
    pub travel_time: Duration,
    /// 弧成本 / Arc cost.
    pub arc_cost: f64,
    /// 进入该弧时计入的真实路线成本（起始弧含固定成本） / Real route cost charged on this arc (start arcs include fixed cost).
    pub route_cost: f64,
    /// 当前阶段 reduced-cost 增量 / Current-phase reduced-cost increment.
    pub reduced_cost: f64,
}

/// 定价图快照 / Pricing graph snapshot.
#[derive(Debug, Clone)]
pub struct PricingGraph {
    /// 图节点 / Graph nodes.
    pub nodes: Vec<PricingNode>,
    /// 图弧 / Graph arcs.
    pub arcs: Vec<PricingArc>,
    /// 按节点索引的出弧索引 / Outgoing-arc indices by node.
    pub outgoing: Vec<Vec<usize>>,
    /// 车辆类型 / Vehicle type.
    pub vehicle_type_id: VehicleTypeId,
    /// 车辆容量 / Vehicle capacity.
    pub vehicle_capacity: f64,
    /// 构图时使用的分支遮罩 / Branch mask used while building the graph.
    pub branch_mask: Option<BranchMask<VehicleTypeId>>,
    /// 构图时使用的对偶快照 / Dual snapshot used while building the graph.
    pub duals: PricingDuals,
    /// 构图时使用的实例身份 / Instance identity used while building the graph.
    pub instance_identity: u64,
    /// 构图时使用的实例内容指纹 / Instance-content fingerprint used while building the graph.
    pub instance_fingerprint: u64,
    /// 图快照完整性指纹；外部调用方不能伪造 / Graph-snapshot integrity fingerprint; callers outside this crate cannot forge it.
    pub(crate) integrity_fingerprint: u64,
}

impl PricingGraph {
    /// 获取起始仓库索引 / Get start-depot index.
    pub fn start_index(&self) -> usize {
        self.nodes
            .iter()
            .position(|node| node.is_start_depot)
            .unwrap_or(0)
    }

    /// 获取结束仓库索引 / Get end-depot index.
    pub fn end_index(&self) -> usize {
        self.nodes
            .iter()
            .position(|node| node.is_end_depot)
            .unwrap_or(self.nodes.len().saturating_sub(1))
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
                self.vehicle_capacity,
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

/// Branch-aware 定价图构建器 / Branch-aware pricing-graph builder.
pub struct RouteGraphBuilder<
    V,
    D = crate::domain::vrp::EuclideanDistanceCalculator,
    T = crate::domain::vrp::DistanceAsTravelTimeCalculator<D>,
    A = crate::domain::vrp::DistanceArcCostCalculator,
    C = crate::domain::vrp::FixedPlusArcCostPolicy,
> where
    V: SolveValue + UnitConversionValue,
{
    /// 实例快照 / Instance snapshot.
    pub instance: Arc<VrptwInstance<V>>,
    /// 距离策略 / Distance policy.
    pub distance_calculator: D,
    /// 行驶时间策略 / Travel-time policy.
    pub travel_time_calculator: T,
    /// 弧成本策略 / Arc-cost policy.
    pub arc_cost_calculator: A,
    /// 路线成本策略 / Route-cost policy.
    pub route_cost_policy: C,
    /// 静态弧可行性策略 / Static arc-feasibility policy.
    pub arc_feasibility_policy: Arc<dyn ArcFeasibilityPolicy<V>>,
}

impl<V, D, T, A, C> RouteGraphBuilder<V, D, T, A, C>
where
    V: SolveValue + UnitConversionValue,
    D: DistanceCalculator<V>,
    T: TravelTimeCalculator<V>,
    A: ArcCostCalculator<V>,
    C: RouteCostPolicy<V>,
{
    /// 创建构建器 / Create a builder.
    pub fn new(
        instance: Arc<VrptwInstance<V>>,
        distance_calculator: D,
        travel_time_calculator: T,
        arc_cost_calculator: A,
        route_cost_policy: C,
    ) -> Self {
        Self {
            instance,
            distance_calculator,
            travel_time_calculator,
            arc_cost_calculator,
            route_cost_policy,
            arc_feasibility_policy: Arc::new(crate::domain::vrp::DefaultArcFeasibilityPolicy),
        }
    }

    /// 设置静态弧可行性策略 / Set the static arc-feasibility policy.
    pub fn with_arc_feasibility_policy(mut self, policy: Arc<dyn ArcFeasibilityPolicy<V>>) -> Self {
        self.arc_feasibility_policy = policy;
        self
    }

    /// 构建指定车辆类型的 branch-aware 图 / Build a branch-aware graph for one vehicle type.
    pub fn build(
        &self,
        vehicle_type_id: &VehicleTypeId,
        duals: &PricingDuals,
        branch_mask: Option<&BranchMask<VehicleTypeId>>,
    ) -> Result<PricingGraph> {
        let vehicle_type = self.instance.vehicle_type(vehicle_type_id).ok_or_else(|| {
            NetworkSchedulingError::validation(format!(
                "车辆类型不存在：{} / vehicle type does not exist: {}",
                vehicle_type_id, vehicle_type_id
            ))
        })?;
        let window = &self.instance.scheduling_window;
        let value_of = |instant| {
            window
                .value_of_instant(instant)
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })
        };
        let mut nodes = Vec::with_capacity(self.instance.customers.len() + 2);
        nodes.push(PricingNode {
            node_id: self.instance.start_depot.node.id.clone(),
            customer_index: None,
            is_start_depot: true,
            is_end_depot: false,
            ready_time: value_of(self.instance.start_depot.time_window.ready_time)?,
            due_time: value_of(self.instance.start_depot.time_window.due_time)?,
            service_time: 0.0,
            demand: 0.0,
        });
        for (index, customer) in self.instance.customers.iter().enumerate() {
            if branch_mask.is_some_and(|mask| !mask.allows_node(vehicle_type_id, &customer.node.id))
            {
                continue;
            }
            let demand = customer
                .demand
                .to_unit(&self.instance.units.load_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?
                .value
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            let service_time = window
                .value_of_duration(customer.service_time)
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            nodes.push(PricingNode {
                node_id: customer.node.id.clone(),
                customer_index: Some(index),
                is_start_depot: false,
                is_end_depot: false,
                ready_time: value_of(customer.time_window.ready_time)?,
                due_time: value_of(customer.time_window.due_time)?,
                service_time,
                demand,
            });
        }
        nodes.push(PricingNode {
            node_id: self.instance.end_depot.node.id.clone(),
            customer_index: None,
            is_start_depot: false,
            is_end_depot: true,
            ready_time: value_of(self.instance.end_depot.time_window.ready_time)?,
            due_time: value_of(self.instance.end_depot.time_window.due_time)?,
            service_time: 0.0,
            demand: 0.0,
        });

        let capacity = vehicle_type
            .capacity
            .to_unit(&self.instance.units.load_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let fixed_cost = vehicle_type
            .fixed_cost
            .to_unit(&self.instance.units.cost_unit)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?
            .value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        let mut arcs = Vec::new();
        let mut outgoing = vec![Vec::new(); nodes.len()];
        let candidates = if self.instance.arcs.is_empty() {
            let mut candidates = Vec::new();
            for from_index in 0..nodes.len() {
                for to_index in 0..nodes.len() {
                    if from_index == to_index
                        || nodes[to_index].is_start_depot
                        || nodes[from_index].is_end_depot
                    {
                        continue;
                    }
                    candidates.push((from_index, to_index, None));
                }
            }
            candidates
        } else {
            self.instance
                .arcs
                .iter()
                .filter_map(|arc| {
                    let from_index = nodes.iter().position(|node| node.node_id == arc.from)?;
                    let to_index = nodes.iter().position(|node| node.node_id == arc.to)?;
                    Some((from_index, to_index, Some(arc.clone())))
                })
                .collect()
        };
        for (from_index, to_index, base_arc) in candidates {
            if from_index == to_index
                || nodes[to_index].is_start_depot
                || nodes[from_index].is_end_depot
                || base_arc.as_ref().is_some_and(|arc| !arc.feasible)
            {
                continue;
            }
            let from = resolve_node(&self.instance, &nodes[from_index].node_id)?;
            let to = resolve_node(&self.instance, &nodes[to_index].node_id)?;
            if !self
                .arc_feasibility_policy
                .is_feasible(from, to, vehicle_type)?
            {
                continue;
            }
            let arc_id = base_arc
                .as_ref()
                .map(|arc| arc.id.clone())
                .unwrap_or_else(|| {
                    default_arc_id(&nodes[from_index].node_id, &nodes[to_index].node_id)
                });
            if branch_mask.is_some_and(|mask| {
                !mask.allows_arc(
                    vehicle_type_id,
                    &arc_id,
                    &nodes[from_index].node_id,
                    &nodes[to_index].node_id,
                )
            }) {
                continue;
            }
            let (distance, travel_time, arc_cost) = if let Some(arc) = base_arc {
                let distance = arc
                    .distance
                    .to_unit(&self.instance.units.distance_unit)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?
                    .value
                    .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?;
                let arc_cost = arc
                    .cost
                    .to_unit(&self.instance.units.cost_unit)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?
                    .value
                    .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?;
                (distance, arc.travel_time.duration, arc_cost)
            } else {
                let distance = self
                    .distance_calculator
                    .distance(from, to, &self.instance.units.distance_unit)?
                    .value
                    .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?;
                let travel_time = self.travel_time_calculator.travel_time(
                    from,
                    to,
                    vehicle_type,
                    &self.instance,
                )?;
                let arc_cost = self
                    .arc_cost_calculator
                    .cost(
                        from,
                        to,
                        &ospf_rust_quantities::Quantity::new(
                            V::from_f64_with_policy(
                                distance,
                                SolveValueConversionPolicy::AllowRounding,
                            )
                            .map_err(|error| {
                                NetworkSchedulingError::Conversion {
                                    message: error.to_string(),
                                }
                            })?,
                            self.instance.units.distance_unit.clone(),
                        ),
                        travel_time,
                        vehicle_type,
                        &self.instance,
                    )?
                    .to_unit(&self.instance.units.cost_unit)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?
                    .value
                    .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?;
                (distance, travel_time, arc_cost)
            };
            if travel_time < Duration::ZERO {
                return Err(NetworkSchedulingError::validation(
                    "行驶时间不能为负 / travel time cannot be negative",
                ));
            }
            let route_cost = arc_cost
                + if nodes[from_index].is_start_depot {
                    fixed_cost
                } else {
                    0.0
                };
            let customer_dual = nodes[to_index]
                .customer_index
                .and_then(|index| self.instance.customers.get(index))
                .map(|customer| duals.customer_dual(&customer.id))
                .unwrap_or(0.0);
            let fleet_dual = if nodes[from_index].is_start_depot {
                duals.fleet_dual(vehicle_type_id)
            } else {
                0.0
            };
            let phase_cost = match duals.phase {
                crate::domain::vrp::PricingPhase::PhaseOne => 0.0,
                crate::domain::vrp::PricingPhase::PhaseTwo => route_cost,
            };
            let index = arcs.len();
            arcs.push(PricingArc {
                arc_id,
                from: from_index,
                to: to_index,
                distance,
                travel_time,
                arc_cost,
                route_cost,
                reduced_cost: phase_cost - customer_dual - fleet_dual,
            });
            outgoing[from_index].push(index);
        }
        let mut graph = PricingGraph {
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
}

fn resolve_node<'a, V>(instance: &'a VrptwInstance<V>, id: &NetworkNodeId) -> Result<&'a VrpNode<V>>
where
    V: SolveValue + UnitConversionValue,
{
    if id == &instance.start_depot.node.id {
        return Ok(&instance.start_depot.node);
    }
    if id == &instance.end_depot.node.id {
        return Ok(&instance.end_depot.node);
    }
    instance
        .customers
        .iter()
        .find(|customer| &customer.node.id == id)
        .map(|customer| &customer.node)
        .ok_or_else(|| {
            NetworkSchedulingError::structure(format!(
                "定价图节点不存在：{} / pricing graph node does not exist: {}",
                id, id
            ))
        })
}
