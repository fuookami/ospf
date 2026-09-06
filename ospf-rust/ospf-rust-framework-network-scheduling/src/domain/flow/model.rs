//! 通用 flow 数据模型 / Generic flow data model.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};

use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{
    CapacityBounds, NetworkArcId, NetworkCost, NetworkGraph, NetworkNode, NetworkNodeId,
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
pub struct SupplyDemand<V: SolveValue + UnitConversionValue> {
    /// 商品 ID / Commodity ID.
    pub commodity: CommodityId,
    /// 节点平衡；正值供给、负值需求 / Node balances; positive is supply and negative is demand.
    pub balances: HashMap<NetworkNodeId, Quantity<V, Unit>>,
}

impl<V> SupplyDemand<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建供需表 / Create a supply-demand map.
    pub fn new(commodity: impl Into<CommodityId>) -> Self {
        Self {
            commodity: commodity.into(),
            balances: HashMap::new(),
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

/// flow 商品 / Flow commodity.
pub type FlowCommodity<V> = SupplyDemand<V>;

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
pub struct FlowGraph<V: SolveValue + UnitConversionValue> {
    /// 节点快照 / Node snapshot.
    pub nodes: Vec<FlowNode>,
    /// 弧快照 / Arc snapshot.
    pub arcs: Vec<FlowArc<V>>,
    /// 商品与供需 / Commodities and balances.
    pub commodities: Vec<FlowCommodity<V>>,
    /// 经过归一化的单位口径 / Normalized unit contract.
    pub units: FlowUnits,
    node_indices: HashMap<NetworkNodeId, usize>,
    arc_indices: HashMap<NetworkArcId, usize>,
}

impl<V> FlowGraph<V>
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
        let arcs = arcs
            .into_iter()
            .map(|mut arc| {
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
                commodity.balances = commodity
                    .balances
                    .into_iter()
                    .map(|(node, balance)| {
                        let balance = balance.to_unit(&units.flow_unit).map_err(|error| {
                            NetworkSchedulingError::Conversion {
                                message: error.to_string(),
                            }
                        })?;
                        Ok((node, balance))
                    })
                    .collect::<Result<HashMap<_, _>>>()?;
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
        let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        for commodity in &commodities {
            if !commodity_ids.insert(commodity.commodity.clone()) {
                return Err(NetworkSchedulingError::validation(format!(
                    "商品 ID 重复：{} / duplicate commodity ID: {}",
                    commodity.commodity, commodity.commodity
                )));
            }
            let total = commodity
                .balances
                .iter()
                .try_fold(zero.clone(), |sum, (node, value)| {
                    if !node_indices.contains_key(node) {
                        return Err(NetworkSchedulingError::structure(format!(
                            "商品 {} 引用了不存在的节点 {} / commodity {} references missing node {}",
                            commodity.commodity, node, commodity.commodity, node
                        )));
                    }
                    Ok(sum + value.value.clone())
                })?;
            if total.partial_cmp(&zero) != Some(Ordering::Equal) {
                return Err(NetworkSchedulingError::validation(format!(
                    "商品 {} 的总供需必须为零 / total balance of commodity {} must be zero",
                    commodity.commodity, commodity.commodity
                )));
            }
        }

        let _ = generic;
        Ok(Self {
            nodes,
            arcs,
            commodities,
            units,
            node_indices,
            arc_indices,
        })
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
