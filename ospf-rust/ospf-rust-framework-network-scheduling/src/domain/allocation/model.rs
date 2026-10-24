//! 运输分配输入到既有 Flow/Design 的单一映射边界。 / Single mapping boundary from allocation inputs to the existing Flow/Design kernel.

use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, NoneUnit, Unit, UnitConversionValue};
use ospf_rust_core::solver::value::SolveValue;
use crate::domain::design::{CapacityLevel, NetworkDesignProblem, NetworkFacility, NetworkService};
use crate::domain::flow::{CommodityId, FlowArc, FlowCommodity, FlowNode, FlowProblem, FlowResource, FlowSolution, FlowUnits};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{CapacityBounds, NetworkArcId, NetworkCost, NetworkNodeId, NetworkSchedulingSolverValueAdapter, validate_linear_units};

/// 地点；可选层级约束服务方向为非递减。 / Location with optional nondecreasing service layers.
#[derive(Debug, Clone)]
pub struct AllocationLocation<V> {
    /// 稳定地点 ID / Stable location ID.
    pub id: NetworkNodeId,
    /// 可选业务层级 / Optional business layer.
    pub layer: Option<i64>,
    /// 保留的地点物理属性 / Preserved physical location attributes.
    pub attributes: BTreeMap<String, Quantity<V, Unit>>,
}

impl<V> AllocationLocation<V> {
    /// 创建无层级地点。 / Create a location without a layer.
    pub fn new(id: impl Into<NetworkNodeId>) -> Self { Self { id: id.into(), layer: None, attributes: BTreeMap::new() } }
}

/// 固定或带建设容量等级的服务。 / Fixed service or service with construction capacity levels.
#[derive(Debug, Clone)]
pub struct AllocationService<V> {
    /// 稳定服务 ID，保留平行服务 / Stable service ID preserving parallel services.
    pub id: NetworkArcId,
    /// 起点 / Origin.
    pub from: NetworkNodeId,
    /// 终点 / Destination.
    pub to: NetworkNodeId,
    /// 默认单位容量运行资费；请求可覆盖 / Default per-capacity-unit operating tariff, overridable per request.
    pub cost: Quantity<V, Unit>,
    /// 固定容量或有限建设包络 / Fixed capacity or finite construction envelope.
    pub capacity: CapacityBounds<V, Unit>,
    /// 总容量等级；空集合表示固定启用 / Total-capacity levels; empty means fixed availability.
    pub capacity_levels: Vec<CapacityLevel<V>>,
    /// 有等级时必须选择一个 / Require one level when levels exist.
    pub required: bool,
    /// 保留的服务物理属性 / Preserved physical service attributes.
    pub attributes: BTreeMap<String, Quantity<V, Unit>>,
}

/// 单来源至单去向请求，整数步长仍允许分拆运输。 / Single-origin/destination request; integer steps still allow split transportation.
#[derive(Debug, Clone)]
pub struct AllocationRequest<V> {
    /// 同时作为 Flow 商品 ID / Also the Flow commodity ID.
    pub id: CommodityId,
    /// 起运地点 / Shipment origin.
    pub origin: NetworkNodeId,
    /// 目的地点 / Shipment destination.
    pub destination: NetworkNodeId,
    /// 有限正请求量 / Finite positive requested amount.
    pub quantity: Quantity<V, Unit>,
    /// 请求自己的流量单位 / Request-specific flow unit.
    pub unit: Unit,
    /// 每个整数计数的物理量；不表示整单二元选择 / Physical quantity per integer count, not a whole-order binary choice.
    pub integer_step: Option<Quantity<V, Unit>>,
    /// 可用服务；None 表示全部 / Compatible services; None means all.
    pub allowed_services: Option<BTreeSet<NetworkArcId>>,
    /// 按请求单位计量的服务资费覆盖 / Per-request-unit service tariff overrides.
    pub costs: BTreeMap<NetworkArcId, Quantity<V, Unit>>,
}

impl<V> AllocationRequest<V> {
    /// 使用请求量单位创建连续流请求。 / Create a continuous-flow request using the quantity's unit.
    pub fn new(
        id: impl Into<CommodityId>,
        origin: impl Into<NetworkNodeId>,
        destination: impl Into<NetworkNodeId>,
        quantity: Quantity<V, Unit>,
    ) -> Self {
        Self { id: id.into(), origin: origin.into(), destination: destination.into(), unit: quantity.unit.clone(),
            quantity, integer_step: None, allowed_services: None, costs: BTreeMap::new() }
    }
}

/// 明确绑定请求与稳定服务的用量键。 / Usage key explicitly binding a request to a stable service.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AllocationFlowKey {
    /// 请求 / Request.
    pub request: CommodityId,
    /// 服务 / Service.
    pub service: NetworkArcId,
}

/// 显式共享配额；未声明的用量为零。 / Explicit shared quota; unspecified usage is zero.
#[derive(Debug, Clone)]
pub struct AllocationQuota<V> {
    /// 唯一配额 ID / Unique quota ID.
    pub id: String,
    /// 有限上下界 / Finite bounds.
    pub bounds: CapacityBounds<V, Unit>,
    /// 每请求单位消耗的配额量 / Quota consumed per request unit.
    pub usage: BTreeMap<AllocationFlowKey, Quantity<V, Unit>>,
}

/// 枢纽建设及受其启用控制的配额。 / Hub construction and quotas gated by its activation.
#[derive(Debug, Clone)]
pub struct AllocationHub<V> {
    /// 枢纽地点 / Hub location.
    pub location: NetworkNodeId,
    /// 开设费用 / Opening cost.
    pub opening_cost: Quantity<V, Unit>,
    /// 是否必选 / Whether opening is required.
    pub required: bool,
    /// 每个配额只允许一个枢纽负责 / Each quota may have only one controlling hub.
    pub quota_ids: BTreeSet<String>,
}

/// 物理口径与可选共享资源配置。 / Physical units and optional shared-resource configuration.
#[derive(Debug, Clone)]
pub struct AllocationOptions<V> {
    /// 所有服务容量的规范单位 / Canonical unit for service capacities.
    pub capacity_unit: Unit,
    /// 运行加建设的总费用单位 / Total operating and construction cost unit.
    pub cost_unit: Unit,
    /// 共享配额 / Shared quotas.
    pub quotas: Vec<AllocationQuota<V>>,
    /// 可选枢纽 / Optional hubs.
    pub hubs: Vec<AllocationHub<V>>,
    /// 每请求单位对服务容量的显式消耗，跨量纲时必需 / Explicit service-capacity usage per request unit, required across dimensions.
    pub service_usage: BTreeMap<AllocationFlowKey, Quantity<V, Unit>>,
    /// 可选建设预算 / Optional construction budget.
    pub budget: Option<Quantity<V, Unit>>,
}

impl<V> Default for AllocationOptions<V> {
    fn default() -> Self {
        let unit = NoneUnit::INSTANT;
        Self { capacity_unit: unit.clone(), cost_unit: unit.clone(), quotas: Vec::new(),
            hubs: Vec::new(), service_usage: BTreeMap::new(), budget: None }
    }
}

/// 不可变分配问题及经预校验的单一下层模型。 / Immutable allocation problem and its single prevalidated underlying model.
#[derive(Debug, Clone)]
pub struct AllocationProblem<V: SolveValue + UnitConversionValue> {
    locations: Vec<AllocationLocation<V>>,
    services: Vec<AllocationService<V>>,
    requests: Vec<AllocationRequest<V>>,
    options: AllocationOptions<V>,
    design: NetworkDesignProblem<V>,
}

impl<V: SolveValue + UnitConversionValue> AllocationProblem<V> {
    /// 在修改模型前验证业务引用、层级、单位与费用，再映射 Flow/Design。 / Validate business references, layers, units and costs before model mutation, then map to Flow/Design.
    pub fn new(
        locations: Vec<AllocationLocation<V>>,
        services: Vec<AllocationService<V>>,
        requests: Vec<AllocationRequest<V>>,
        options: AllocationOptions<V>,
    ) -> Result<Self> {
        validate_linear_units([&options.capacity_unit, &options.cost_unit])?;
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let mut location_ids = BTreeMap::new();
        for location in &locations {
            if location.id.as_str().trim().is_empty() || location_ids.insert(location.id.clone(), location).is_some() {
                return Err(NetworkSchedulingError::validation("地点 ID 为空或重复 / empty or duplicate location ID"));
            }
            for attribute in location.attributes.values() { adapter.normalize(attribute, &attribute.unit)?; }
        }
        let mut service_ids = BTreeSet::new();
        for service in &services {
            let from = location_ids.get(&service.from);
            let to = location_ids.get(&service.to);
            if service.id.as_str().trim().is_empty() || !service_ids.insert(service.id.clone()) || from.is_none() || to.is_none()
                || from.and_then(|location| location.layer).zip(to.and_then(|location| location.layer)).is_some_and(|(from, to)| from > to)
            {
                return Err(NetworkSchedulingError::validation("服务 ID、端点或层级方向非法 / invalid service ID, endpoints or layer direction"));
            }
            for attribute in service.attributes.values() { adapter.normalize(attribute, &attribute.unit)?; }
        }
        let mut request_ids = BTreeMap::new();
        for request in &requests {
            if request.id.as_str().trim().is_empty() || request_ids.insert(request.id.clone(), request).is_some()
                || !location_ids.contains_key(&request.origin) || !location_ids.contains_key(&request.destination)
                || request.origin == request.destination || request.allowed_services.iter().flatten().any(|id| !service_ids.contains(id))
                || request.costs.keys().any(|id| !service_ids.contains(id))
            {
                return Err(NetworkSchedulingError::validation("请求 ID、端点、兼容服务或费用覆盖非法 / invalid request ID, endpoints, compatible service or cost override"));
            }
            if adapter.normalize(&request.quantity, &request.unit)? <= 0.0 {
                return Err(NetworkSchedulingError::validation("请求数量必须为有限正数 / request amount must be finite and positive"));
            }
        }
        let validate_key = |key: &AllocationFlowKey| -> Result<()> {
            if !service_ids.contains(&key.service) || request_ids.get(&key.request).is_none_or(|request|
                request.allowed_services.as_ref().is_some_and(|ids| !ids.contains(&key.service)))
            {
                return Err(NetworkSchedulingError::structure("配额或容量用量引用未知或不兼容的请求服务 / quota or capacity usage references an unknown or incompatible request-service pair"));
            }
            Ok(())
        };
        for key in options.service_usage.keys() { validate_key(key)?; }
        let mut quota_ids = BTreeSet::new();
        for quota in &options.quotas {
            if quota.id.trim().is_empty() || quota.id.starts_with("arc:") || !quota_ids.insert(quota.id.clone()) {
                return Err(NetworkSchedulingError::validation("配额 ID 为空、重复或使用保留前缀 / empty, duplicate or reserved quota ID"));
            }
            for key in quota.usage.keys() { validate_key(key)?; }
        }
        let mut hubs = BTreeSet::new();
        let mut controlled = BTreeSet::new();
        for hub in &options.hubs {
            if !location_ids.contains_key(&hub.location) || !hubs.insert(hub.location.clone())
                || hub.quota_ids.iter().any(|id| !quota_ids.contains(id) || !controlled.insert(id.clone()))
            {
                return Err(NetworkSchedulingError::validation("枢纽或受控配额未知、重复 / unknown or duplicate hub or controlled quota"));
            }
        }
        let commodities = requests.iter().map(|request| {
            let amount = request.quantity.to_unit(&request.unit)
                .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
            let negative = adapter.from_solver(0.0)? - amount.value.clone();
            adapter.to_solver(&negative)?;
            let mut commodity = FlowCommodity::new(request.id.clone())
                .with_balance(request.origin.clone(), amount)
                .with_balance(request.destination.clone(), Quantity::new(negative, request.unit.clone()));
            commodity.unit = Some(request.unit.clone());
            commodity.integer_step = request.integer_step.clone();
            commodity.allowed_arcs = request.allowed_services.as_ref().map(|ids| ids.iter().cloned().collect());
            commodity.costs = request.costs.iter().map(|(id, cost)| (id.clone(), NetworkCost::new(cost.clone()))).collect();
            commodity.capacity_consumption = options.service_usage.iter().filter(|(key, _)| key.request == request.id)
                .map(|(key, usage)| (key.service.clone(), usage.clone())).collect();
            Ok(commodity)
        }).collect::<Result<Vec<_>>>()?;
        let flow = FlowProblem::new_with_units(
            locations.iter().map(|location| FlowNode::new(location.id.clone())).collect(),
            services.iter().map(|service| FlowArc::new(service.id.clone(), service.from.clone(), service.to.clone(),
                service.capacity.clone(), NetworkCost::new(service.cost.clone()))).collect(),
            commodities,
            FlowUnits::new(options.capacity_unit.clone(), (&options.cost_unit / &options.capacity_unit).build())?,
        )?.with_resources(options.quotas.iter().map(|quota| FlowResource {
            id: quota.id.clone(), capacity: quota.bounds.clone(),
            consumptions: quota.usage.iter().map(|(key, usage)| ((key.request.clone(), key.service.clone()), usage.clone())).collect(),
        }).collect())?;
        let design = NetworkDesignProblem::new(flow, services.iter().filter(|service| !service.capacity_levels.is_empty())
            .map(|service| NetworkService { arc: service.id.clone(), levels: service.capacity_levels.clone(), required: service.required }).collect(),
            options.hubs.iter().map(|hub| NetworkFacility { node: hub.location.clone(), cost: hub.opening_cost.clone(),
                required: hub.required, capacity_rows: hub.quota_ids.clone() }).collect(), options.budget.clone())?;
        Ok(Self { locations, services, requests, options, design })
    }

    /// 地点快照 / Location snapshot.
    pub fn locations(&self) -> &[AllocationLocation<V>] { &self.locations }

    /// 服务快照 / Service snapshot.
    pub fn services(&self) -> &[AllocationService<V>] { &self.services }

    /// 请求快照 / Request snapshot.
    pub fn requests(&self) -> &[AllocationRequest<V>] { &self.requests }

    /// 单位、配额和枢纽配置快照 / Units, quotas and hub configuration snapshot.
    pub fn options(&self) -> &AllocationOptions<V> { &self.options }

    /// 预校验的唯一 Flow/Design 映射，供扩展复用 / Prevalidated single Flow/Design mapping for extensions.
    pub fn design(&self) -> &NetworkDesignProblem<V> { &self.design }
}

/// 运输分配和建设决策。 / Transportation allocation and construction decisions.
#[derive(Debug, Clone)]
pub struct AllocationSolution<V> {
    /// 各请求-服务的实际物理量，而非整数计数 / Actual request-service physical quantities, not integer counts.
    pub shipments: BTreeMap<AllocationFlowKey, Quantity<V, Unit>>,
    /// 经生产复核的底层流与资源结果 / Production-validated underlying flow and resource results.
    pub flow: FlowSolution<V>,
    /// 已开设枢纽 / Open hubs.
    pub open_hubs: BTreeSet<NetworkNodeId>,
    /// 选中的服务容量等级 / Selected service capacity levels.
    pub service_levels: BTreeMap<NetworkArcId, String>,
    /// 建设费用 / Construction cost.
    pub construction_cost: Quantity<V, Unit>,
    /// 运行加建设费用 / Operating plus construction cost.
    pub total_cost: Quantity<V, Unit>,
}
