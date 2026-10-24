//! 通用 flow 数据模型 / Generic flow data model.

use std::collections::{HashMap, HashSet};

use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};

use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{
    CapacityBounds, NetworkArcId, NetworkCost, NetworkGraph, NetworkNode, NetworkNodeId,
    NetworkSchedulingSolverValueAdapter, validate_linear_units,
};

/// 商品稳定 ID / Stable commodity ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CommodityId(String);

impl CommodityId {
    /// 创建商品 ID / Create a commodity ID.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 获取字符串 / Get the string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CommodityId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for CommodityId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// flow 节点 / Flow node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FlowNode {
    /// 稳定节点 ID / Stable node ID.
    pub id: NetworkNodeId,
}

impl FlowNode {
    /// 创建 flow 节点 / Create a flow node.
    pub fn new(id: impl Into<NetworkNodeId>) -> Self {
        Self { id: id.into() }
    }
}

/// flow 弧 / Flow arc.
#[derive(Debug, Clone)]
pub struct FlowArc<V: SolveValue + UnitConversionValue> {
    /// 稳定弧 ID / Stable arc ID.
    pub id: NetworkArcId,
    /// 起点 / Origin.
    pub from: NetworkNodeId,
    /// 终点 / Destination.
    pub to: NetworkNodeId,
    /// 容量上下界 / Capacity bounds.
    pub capacity: CapacityBounds<V, Unit>,
    /// 单位流成本 / Unit-flow cost.
    pub cost: NetworkCost<V, Unit>,
}

impl<V> FlowArc<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建 flow 弧 / Create a flow arc.
    pub fn new(
        id: impl Into<NetworkArcId>,
        from: impl Into<NetworkNodeId>,
        to: impl Into<NetworkNodeId>,
        capacity: CapacityBounds<V, Unit>,
        cost: NetworkCost<V, Unit>,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            capacity,
            cost,
        }
    }
}

/// 一个商品的节点供需 / Supply-demand map for one commodity.
#[derive(Debug, Clone)]
pub struct FlowCommodity<V: SolveValue + UnitConversionValue> {
    /// 商品 ID / Commodity ID.
    pub commodity: CommodityId,
    /// 节点平衡；正值供给、负值需求 / Node balances; positive is supply and negative is demand.
    pub balances: HashMap<NetworkNodeId, Quantity<V, Unit>>,
    /// 独立商品单位；为空时使用问题默认流单位 / Independent commodity unit, or the problem default when absent.
    pub unit: Option<Unit>,
    /// 可用弧；为空表示所有弧 / Allowed arcs, or all arcs when absent.
    pub allowed_arcs: Option<HashSet<NetworkArcId>>,
    /// 每商品弧资费，覆盖默认弧费用 / Per-commodity arc tariffs overriding default arc costs.
    pub costs: HashMap<NetworkArcId, NetworkCost<V, Unit>>,
    /// 每商品单位消耗的弧容量；跨量纲必须显式声明 / Arc capacity per commodity unit; explicit across dimensions.
    pub capacity_consumption: HashMap<NetworkArcId, Quantity<V, Unit>>,
    /// 整数计数变量对应的物理量；为空使用连续流 / Physical amount per integer count, or continuous flow when absent.
    pub integer_step: Option<Quantity<V, Unit>>,
    /// 节点有界弃供/缺货，默认没有松弛 / Bounded node spill/shortage; hard balances by default.
    pub slacks: HashMap<NetworkNodeId, FlowBalanceSlack<V>>,
}

impl<V> FlowCommodity<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建供需表 / Create a supply-demand map.
    pub fn new(commodity: impl Into<CommodityId>) -> Self {
        Self {
            commodity: commodity.into(),
            balances: HashMap::new(),
            unit: None,
            allowed_arcs: None,
            costs: HashMap::new(),
            capacity_consumption: HashMap::new(),
            integer_step: None,
            slacks: HashMap::new(),
        }
    }

    /// 写入节点平衡 / Set a node balance.
    pub fn with_balance(
        mut self,
        node: impl Into<NetworkNodeId>,
        balance: Quantity<V, Unit>,
    ) -> Self {
        self.balances.insert(node.into(), balance);
        self
    }
}

/// 跨弧、跨商品共享资源行。 / Shared resource row spanning arcs and commodities.
#[derive(Debug, Clone)]
pub struct FlowResource<V> {
    /// 资源稳定 ID / Stable resource ID.
    pub id: String,
    /// 资源使用上下界 / Resource usage bounds.
    pub capacity: CapacityBounds<V, Unit>,
    /// 每商品单位的资源消耗；未声明项为零 / Resource per commodity unit; unspecified terms are zero.
    pub consumptions: HashMap<(CommodityId, NetworkArcId), Quantity<V, Unit>>,
}

/// 节点处理能力计量方向。 / Node-throughput measurement direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowThroughput {
    /// 所有到达流量 / All arriving flows.
    Incoming,
    /// 所有离开流量 / All departing flows.
    Outgoing,
}

/// 编译为共享资源行的节点处理能力。 / Node processing capacity compiled into a shared resource row.
#[derive(Debug, Clone)]
pub struct FlowNodeCapacity<V> {
    /// 全部共享资源中唯一的 ID / ID unique among all shared resources.
    pub id: String,
    /// 节点身份 / Node identity.
    pub node: NetworkNodeId,
    /// 处理方向 / Processing direction.
    pub direction: FlowThroughput,
    /// 有限处理上下界 / Finite processing bounds.
    pub capacity: CapacityBounds<V, Unit>,
    /// 每商品单位的处理用量；同量纲时可省略 / Processing usage per commodity unit, optional for the same dimension.
    pub consumptions: HashMap<CommodityId, Quantity<V, Unit>>,
}

/// 显式有界弃供和缺货及其单位费用。 / Explicit bounded spill and shortage with per-unit penalties.
#[derive(Debug, Clone)]
pub struct FlowBalanceSlack<V> {
    /// 最大弃供量 / Maximum spilled supply.
    pub spill_limit: Quantity<V, Unit>,
    /// 最大缺货量 / Maximum unmet demand.
    pub shortage_limit: Quantity<V, Unit>,
    /// 每单位弃供费用 / Penalty per unit of spill.
    pub spill_cost: NetworkCost<V, Unit>,
    /// 每单位缺货费用 / Penalty per unit of shortage.
    pub shortage_cost: NetworkCost<V, Unit>,
}

/// 流量与独立费用分解的领域解。 / Domain solution with flows and independent cost decomposition.
#[derive(Debug, Clone)]
pub struct FlowSolution<V> {
    /// 每个商品弧的实际物理流量，不是整数计数 / Physical commodity-arc flow, not integer counts.
    pub flows: std::collections::BTreeMap<(CommodityId, NetworkArcId), Quantity<V, Unit>>,
    /// 弃供量 / Spilled supply.
    pub spills: std::collections::BTreeMap<(CommodityId, NetworkNodeId), Quantity<V, Unit>>,
    /// 缺货量 / Unmet demand.
    pub shortages: std::collections::BTreeMap<(CommodityId, NetworkNodeId), Quantity<V, Unit>>,
    /// 共享资源使用 / Shared resource usage.
    pub resources: std::collections::BTreeMap<String, Quantity<V, Unit>>,
    /// 运输运行费用 / Transport operating cost.
    pub operating_cost: Quantity<V, Unit>,
    /// 弃供费用 / Spill cost.
    pub spill_cost: Quantity<V, Unit>,
    /// 缺货费用 / Shortage cost.
    pub shortage_cost: Quantity<V, Unit>,
    /// 总费用 / Total cost.
    pub total_cost: Quantity<V, Unit>,
}

/// 通用 flow 的规范单位口径 / Canonical unit contract for generic flow.
#[derive(Debug, Clone)]
pub struct FlowUnits {
    /// 流量、容量和节点供需的共同单位 / Common unit for flow, capacity, and balances.
    pub flow_unit: Unit,
    /// 单位流成本的共同单位 / Common unit for unit-flow costs.
    pub cost_unit: Unit,
}

impl FlowUnits {
    /// 创建并校验 flow 单位口径 / Create and validate a flow unit contract.
    pub fn new(flow_unit: Unit, cost_unit: Unit) -> Result<Self> {
        if !flow_unit.is_linear() || !cost_unit.is_linear() {
            return Err(NetworkSchedulingError::validation(
                "flow 和成本单位必须是线性单位 / flow and cost units must be linear",
            ));
        }
        Ok(Self {
            flow_unit,
            cost_unit,
        })
    }
}

/// 通用 flow 图快照 / Generic flow graph snapshot.
#[derive(Debug, Clone)]
pub struct FlowProblem<V: SolveValue + UnitConversionValue> {
    /// 节点快照 / Node snapshot.
    pub(super) nodes: Vec<FlowNode>,
    /// 弧快照 / Arc snapshot.
    pub(super) arcs: Vec<FlowArc<V>>,
    /// 商品与供需 / Commodities and balances.
    pub(super) commodities: Vec<FlowCommodity<V>>,
    /// 经过归一化的单位口径 / Normalized unit contract.
    pub(super) units: FlowUnits,
    resources: Vec<FlowResource<V>>,
    node_indices: HashMap<NetworkNodeId, usize>,
    arc_indices: HashMap<NetworkArcId, usize>,
}

impl<V> FlowProblem<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建并校验 flow 图 / Create and validate a flow graph.
    pub fn new(
        nodes: Vec<FlowNode>,
        arcs: Vec<FlowArc<V>>,
        commodities: Vec<FlowCommodity<V>>,
    ) -> Result<Self> {
        let first_arc = arcs.first().ok_or_else(|| {
            NetworkSchedulingError::validation(
                "至少需要一条弧以推导 flow 单位；空图请使用 new_with_units / at least one arc is required to infer flow units; use new_with_units for an empty graph",
            )
        })?;
        let units = FlowUnits::new(
            first_arc.capacity.lower.unit.clone(),
            first_arc.cost.quantity.unit.clone(),
        )?;
        Self::new_with_units(nodes, arcs, commodities, units)
    }

    /// 使用显式单位口径创建并归一化 flow 图 / Create and normalize a flow graph with an explicit unit contract.
    pub fn new_with_units(
        nodes: Vec<FlowNode>,
        arcs: Vec<FlowArc<V>>,
        commodities: Vec<FlowCommodity<V>>,
        units: FlowUnits,
    ) -> Result<Self> {
        validate_linear_units([&units.flow_unit, &units.cost_unit])?;
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let total_cost_unit = (&units.flow_unit * &units.cost_unit).build();
        let arcs = arcs
            .into_iter()
            .map(|mut arc| {
                validate_linear_units([&arc.capacity.lower.unit, &arc.capacity.upper.unit, &arc.cost.quantity.unit])?;
                let lower = arc
                    .capacity
                    .lower
                    .to_unit(&units.flow_unit)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?;
                let upper = arc
                    .capacity
                    .upper
                    .to_unit(&units.flow_unit)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?;
                arc.capacity = CapacityBounds::try_new(lower, upper)?;
                arc.cost = NetworkCost::new(arc.cost.quantity.to_unit(&units.cost_unit).map_err(
                    |error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    },
                )?);
                adapter.to_solver(&arc.capacity.lower.value)?;
                adapter.to_solver(&arc.capacity.upper.value)?;
                adapter.to_solver(&arc.cost.quantity.value)?;
                Ok(arc)
            })
            .collect::<Result<Vec<_>>>()?;
        let commodities = commodities
            .into_iter()
            .map(|mut commodity| {
                if commodity.commodity.as_str().is_empty() {
                    return Err(NetworkSchedulingError::validation(
                        "商品 ID 不能为空 / commodity ID cannot be empty",
                    ));
                }
                let unit = commodity.unit.as_ref().unwrap_or(&units.flow_unit).clone();
                validate_linear_units([&unit])?;
                commodity.balances = commodity
                    .balances
                    .into_iter()
                    .map(|(node, balance)| {
                        validate_linear_units([&balance.unit])?;
                        let balance = balance.to_unit(&unit).map_err(|error| {
                            NetworkSchedulingError::Conversion {
                                message: error.to_string(),
                            }
                        })?;
                        adapter.to_solver(&balance.value)?;
                        Ok((node, balance))
                    })
                    .collect::<Result<HashMap<_, _>>>()?;
                if let Some(step) = &mut commodity.integer_step {
                    validate_linear_units([&step.unit])?;
                    *step = step.to_unit(&unit).map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
                    if adapter.to_solver(&step.value)? <= 0.0 {
                        return Err(NetworkSchedulingError::validation("整数步长必须为正 / integer step must be positive"));
                    }
                }
                let cost_unit = (&total_cost_unit / &unit).build();
                let consumption_unit = (&units.flow_unit / &unit).build();
                for slack in commodity.slacks.values_mut() {
                    for limit in [&mut slack.spill_limit, &mut slack.shortage_limit] {
                        validate_linear_units([&limit.unit])?;
                        *limit = limit.to_unit(&unit).map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
                        if adapter.to_solver(&limit.value)? < 0.0 {
                            return Err(NetworkSchedulingError::validation("松弛上界不能为负 / slack upper bound cannot be negative"));
                        }
                    }
                    for cost in [&mut slack.spill_cost, &mut slack.shortage_cost] {
                        validate_linear_units([&cost.quantity.unit])?;
                        cost.quantity = cost.quantity.to_unit(&cost_unit).map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
                        if adapter.to_solver(&cost.quantity.value)? < 0.0 {
                            return Err(NetworkSchedulingError::validation("松弛费用不能为负 / slack cost cannot be negative"));
                        }
                    }
                }
                for cost in commodity.costs.values_mut() {
                    validate_linear_units([&cost.quantity.unit])?;
                    cost.quantity = cost.quantity.to_unit(&cost_unit).map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
                    adapter.to_solver(&cost.quantity.value)?;
                }
                for consumption in commodity.capacity_consumption.values_mut() {
                    validate_linear_units([&consumption.unit])?;
                    *consumption = consumption.to_unit(&consumption_unit).map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
                    if adapter.to_solver(&consumption.value)? <= 0.0 {
                        return Err(NetworkSchedulingError::validation("弧容量用量必须为正 / arc capacity consumption must be positive"));
                    }
                }
                commodity.unit = Some(unit);
                Ok(commodity)
            })
            .collect::<Result<Vec<_>>>()?;

        let generic_nodes = nodes
            .iter()
            .cloned()
            .map(|node| NetworkNode::new(node.id.clone(), node))
            .collect::<Vec<_>>();
        let generic_arcs = arcs
            .iter()
            .cloned()
            .map(|arc| {
                crate::infrastructure::NetworkArc::new(
                    arc.id.clone(),
                    arc.from.clone(),
                    arc.to.clone(),
                    arc,
                )
            })
            .collect::<Vec<_>>();
        let generic = NetworkGraph::new(generic_nodes, generic_arcs)?;

        let node_indices = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.clone(), index))
            .collect::<HashMap<_, _>>();
        let arc_indices = arcs
            .iter()
            .enumerate()
            .map(|(index, arc)| (arc.id.clone(), index))
            .collect::<HashMap<_, _>>();

        if commodities.is_empty() {
            return Err(NetworkSchedulingError::validation(
                "至少需要一个商品 / at least one commodity is required",
            ));
        }
        let mut commodity_ids = HashSet::with_capacity(commodities.len());
        for commodity in &commodities {
            if !commodity_ids.insert(commodity.commodity.clone()) {
                return Err(NetworkSchedulingError::validation(format!(
                    "商品 ID 重复：{} / duplicate commodity ID: {}",
                    commodity.commodity, commodity.commodity
                )));
            }
            for node in commodity.balances.keys() {
                    if !node_indices.contains_key(node) {
                        return Err(NetworkSchedulingError::structure(format!(
                            "商品 {} 引用了不存在的节点 {} / commodity {} references missing node {}",
                            commodity.commodity, node, commodity.commodity, node
                        )));
                    }
            }
            for node in commodity.slacks.keys() {
                if !node_indices.contains_key(node) {
                    return Err(NetworkSchedulingError::structure("松弛引用了未知节点 / slack references an unknown node"));
                }
            }
            for arc in commodity.costs.keys().chain(commodity.capacity_consumption.keys())
                .chain(commodity.allowed_arcs.iter().flatten())
            {
                if !arc_indices.contains_key(arc) {
                    return Err(NetworkSchedulingError::structure("商品引用了未知弧 / commodity references an unknown arc"));
                }
            }
        }

        let _ = generic;
        let problem = Self {
            nodes,
            arcs,
            commodities,
            units,
            node_indices,
            arc_indices,
            resources: Vec::new(),
        };
        for commodity in problem.commodities() {
            for arc in problem.arcs().iter().filter(|arc| problem.is_allowed(commodity, &arc.id)) {
                problem.capacity_coefficient(commodity, &arc.id)?;
                problem.cost_coefficient(commodity, arc)?;
            }
        }
        Ok(problem)
    }

    /// 节点快照 / Node snapshot.
    pub fn nodes(&self) -> &[FlowNode] { &self.nodes }

    /// 弧快照 / Arc snapshot.
    pub fn arcs(&self) -> &[FlowArc<V>] { &self.arcs }

    /// 商品快照 / Commodity snapshot.
    pub fn commodities(&self) -> &[FlowCommodity<V>] { &self.commodities }

    /// 默认单位口径 / Default unit contract.
    pub fn units(&self) -> &FlowUnits { &self.units }

    /// 共享资源行 / Shared resource rows.
    pub fn resources(&self) -> &[FlowResource<V>] { &self.resources }

    /// 消费问题快照并附加资源，所有转换在建模前完成。 / Consume the snapshot and attach resources, converting everything before modeling.
    pub fn with_resources(mut self, resources: Vec<FlowResource<V>>) -> Result<Self> {
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let mut ids = HashSet::new();
        let mut normalized = Vec::new();
        for mut resource in resources {
            if resource.id.is_empty() || !ids.insert(resource.id.clone()) {
                return Err(NetworkSchedulingError::validation("资源 ID 为空或重复 / empty or duplicate resource ID"));
            }
            validate_linear_units([&resource.capacity.lower.unit, &resource.capacity.upper.unit])?;
            let unit = resource.capacity.lower.unit.clone();
            resource.capacity.upper = resource.capacity.upper.to_unit(&unit)
                .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
            let lower = adapter.to_solver(&resource.capacity.lower.value)?;
            let upper = adapter.to_solver(&resource.capacity.upper.value)?;
            if lower < 0.0 || upper < lower {
                return Err(NetworkSchedulingError::validation("资源容量非法 / invalid resource capacity"));
            }
            for ((commodity_id, arc), amount) in &mut resource.consumptions {
                let commodity = self.commodities.iter().find(|item| &item.commodity == commodity_id)
                    .ok_or_else(|| NetworkSchedulingError::structure("资源引用未知商品 / resource references unknown commodity"))?;
                if self.arc(arc).is_none() {
                    return Err(NetworkSchedulingError::structure("资源引用未知弧 / resource references unknown arc"));
                }
                validate_linear_units([&amount.unit])?;
                *amount = amount.to_unit(&(&unit / self.commodity_unit(commodity)).build())
                    .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
                if adapter.to_solver(&amount.value)? < 0.0 {
                    return Err(NetworkSchedulingError::validation("资源用量不能为负 / resource usage cannot be negative"));
                }
            }
            normalized.push(resource);
        }
        self.resources = normalized;
        Ok(self)
    }

    /// 把节点能力转为同一注册路径的共享资源行。 / Convert node capacities to shared resource rows using the same registration path.
    pub fn with_node_capacities(self, capacities: Vec<FlowNodeCapacity<V>>) -> Result<Self> {
        let mut resources = self.resources.clone();
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        for capacity in capacities {
            if self.node(&capacity.node).is_none() {
                return Err(NetworkSchedulingError::structure("处理能力引用未知节点 / processing capacity references unknown node"));
            }
            validate_linear_units([&capacity.capacity.lower.unit, &capacity.capacity.upper.unit])?;
            for commodity in capacity.consumptions.keys() {
                if !self.commodities.iter().any(|item| &item.commodity == commodity) {
                    return Err(NetworkSchedulingError::structure("处理能力引用未知商品 / processing capacity references unknown commodity"));
                }
            }
            let mut terms = HashMap::new();
            for commodity in &self.commodities {
                for arc in self.arcs.iter().filter(|arc| self.is_allowed(commodity, &arc.id) && match capacity.direction {
                    FlowThroughput::Incoming => arc.to == capacity.node,
                    FlowThroughput::Outgoing => arc.from == capacity.node,
                }) {
                    let unit = (&capacity.capacity.lower.unit / self.commodity_unit(commodity)).build();
                    let usage = if let Some(usage) = capacity.consumptions.get(&commodity.commodity) {
                        usage.clone()
                    } else {
                        let one = Quantity::new(adapter.from_solver(1.0)?, self.commodity_unit(commodity).clone());
                        let value = one.to_unit(&capacity.capacity.lower.unit)
                            .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?.value;
                        Quantity::new(value, unit)
                    };
                    terms.insert((commodity.commodity.clone(), arc.id.clone()), usage);
                }
            }
            resources.push(FlowResource { id: capacity.id, capacity: capacity.capacity, consumptions: terms });
        }
        self.with_resources(resources)
    }

    /// 归一化商品单位 / Normalized commodity unit.
    pub fn commodity_unit<'a>(&'a self, commodity: &'a FlowCommodity<V>) -> &'a Unit {
        commodity.unit.as_ref().unwrap_or(&self.units.flow_unit)
    }

    /// 商品是否允许使用弧 / Whether the commodity may use an arc.
    pub fn is_allowed(&self, commodity: &FlowCommodity<V>, arc: &NetworkArcId) -> bool {
        commodity.allowed_arcs.as_ref().is_none_or(|allowed| allowed.contains(arc))
    }

    pub(super) fn step(&self, commodity: &FlowCommodity<V>) -> Result<V> {
        commodity.integer_step.as_ref().map(|step| Ok(step.value.clone())).unwrap_or_else(||
            V::from_f64_with_policy(1.0, SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() }))
    }

    pub(super) fn capacity_coefficient(&self, commodity: &FlowCommodity<V>, arc: &NetworkArcId) -> Result<V> {
        if let Some(consumption) = commodity.capacity_consumption.get(arc) {
            return Ok(consumption.value.clone());
        }
        let one = NetworkSchedulingSolverValueAdapter::<V>::default().from_solver(1.0)?;
        Quantity::new(one, self.commodity_unit(commodity).clone()).to_unit(&self.units.flow_unit)
            .map(|quantity| quantity.value)
            .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })
    }

    pub(super) fn cost_coefficient(&self, commodity: &FlowCommodity<V>, arc: &FlowArc<V>) -> Result<V> {
        let cost = commodity.costs.get(&arc.id).unwrap_or(&arc.cost);
        let total = (&self.units.cost_unit * &self.units.flow_unit).build();
        cost.quantity.to_unit(&(&total / self.commodity_unit(commodity)).build())
            .map(|quantity| quantity.value)
            .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })
    }

    /// 获取节点 / Get a node.
    pub fn node(&self, id: &NetworkNodeId) -> Option<&FlowNode> {
        self.node_indices
            .get(id)
            .and_then(|index| self.nodes.get(*index))
    }

    /// 获取弧 / Get an arc.
    pub fn arc(&self, id: &NetworkArcId) -> Option<&FlowArc<V>> {
        self.arc_indices
            .get(id)
            .and_then(|index| self.arcs.get(*index))
    }

    /// 获取指定商品的供需 / Get one commodity's balance.
    pub fn balance(
        &self,
        commodity: &CommodityId,
        node: &NetworkNodeId,
    ) -> Option<&Quantity<V, Unit>> {
        self.commodities
            .iter()
            .find(|item| &item.commodity == commodity)
            .and_then(|item| item.balances.get(node))
    }
}
